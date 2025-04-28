use behaviortree_common::{Behavior, State, Status};

use crate::async_action_type::AsyncActionType;

use crate::behavior_nodes::{
    AsyncInvertState, AsyncSelectState, AsyncSequenceState, AsyncWaitState,
};

pub struct AsyncChild<S> {
    action_type: AsyncActionType<S>,
    status: tokio::sync::watch::Sender<Option<Status>>,
    state: State,
}

impl<S> AsyncChild<S> {
    pub fn new(
        action_type: AsyncActionType<S>,
        status: tokio::sync::watch::Sender<Option<Status>>,
        state: State,
    ) -> Self {
        Self {
            action_type,
            status,
            state,
        }
    }

    pub fn from_behavior<A>(behavior: Behavior<A>) -> Self
    where
        A: Into<AsyncActionType<S>>,
        S: 'static,
    {
        match behavior {
            Behavior::Action(action) => {
                let action = action.into();

                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::NoChild(action.name(), rx);
                Self::new(action, tx, state)
            }
            Behavior::Wait(target) => {
                let action = Box::new(AsyncWaitState::new(target));
                let action = AsyncActionType::Async(action);

                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::NoChild(action.name(), rx);
                Self::new(action, tx, state)
            }
            Behavior::Invert(child) => {
                let child = Self::from_behavior(*child);
                let child_state = child.state();

                let action = Box::new(AsyncInvertState::new(child));
                let action = AsyncActionType::Async(action);

                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::SingleChild(action.name(), rx, child_state.into());
                Self::new(action, tx, state)
            }
            Behavior::Sequence(children) => {
                let children = children
                    .into_iter()
                    .map(|child| AsyncChild::from_behavior(child))
                    .collect::<Vec<_>>();
                let children_state = children.iter().map(|child| child.state());
                let children_state = std::rc::Rc::from_iter(children_state);

                let action = Box::new(AsyncSequenceState::new(children));
                let action = AsyncActionType::Async(action);

                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::MultipleChildren(action.name(), rx, children_state);
                Self::new(action, tx, state)
            }
            Behavior::Select(children) => {
                let children = children
                    .into_iter()
                    .map(|child| AsyncChild::from_behavior(child))
                    .collect::<Vec<_>>();
                let children_state = children.iter().map(|child| child.state());
                let children_state = std::rc::Rc::from_iter(children_state);

                let action = Box::new(AsyncSelectState::new(children));
                let action = AsyncActionType::Async(action);

                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::MultipleChildren(action.name(), rx, children_state);
                Self::new(action, tx, state)
            }
        }
    }

    pub async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, shared: &S) -> bool {
        let _ignore = self.status.send(Some(Status::Running));
        let success = self.action_type.run(delta, shared).await;
        let status = if success {
            Status::Success
        } else {
            Status::Failure
        };
        let _ignore = self.status.send(Some(status));
        success
    }

    pub fn reset(&mut self, shared: &mut S) {
        if self.status.borrow().is_none() {
            return;
        }
        self.action_type.reset(shared);
        let _ignore = self.status.send(None);
    }

    pub fn state(&self) -> State {
        self.state.clone()
    }
}

#[cfg(test)]
mod tests {
    use ticked_async_executor::TickedAsyncExecutor;

    use crate::test_async_behavior_interface::{DELTA, TestAction, TestShared};

    use super::*;

    #[test]
    fn test_basic_behavior() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Wait(10.0),
            Behavior::Action(TestAction::Success),
            Behavior::Invert(Behavior::Action(TestAction::Failure).into()),
            Behavior::Action(TestAction::Success),
        ]);

        let mut child = AsyncChild::from_behavior(behavior);
        let _state = child.state();

        let executor = TickedAsyncExecutor::default();

        let shared = TestShared;
        let delta = executor.tick_channel();
        executor
            .spawn_local("WaitFuture", async move {
                child.run(delta, &shared).await;
            })
            .detach();

        executor.wait_till_completed(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }
}
