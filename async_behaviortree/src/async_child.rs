use behaviortree_common::{Behavior, State, Status};

use crate::{
    behavior_nodes::{AsyncInvertState, AsyncSelectState, AsyncSequenceState, AsyncWaitState},
    AsyncAction, ToAsyncAction,
};

pub struct AsyncChild<S> {
    action: Box<dyn AsyncAction<S>>,
    status: tokio::sync::watch::Sender<Option<Status>>,
    state: State,
}

impl<S> AsyncChild<S> {
    pub fn new(
        action: Box<dyn AsyncAction<S>>,
        status: tokio::sync::watch::Sender<Option<Status>>,
        state: State,
    ) -> Self {
        Self {
            action,
            status,
            state,
        }
    }

    pub fn from_behavior<A>(behavior: Behavior<A>) -> Self
    where
        A: ToAsyncAction<S>,
        S: 'static,
    {
        match behavior {
            Behavior::Action(action) => {
                let action = action.to_async_action();

                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::NoChild(action.name(), rx);
                Self::new(action, tx, state)
            }
            Behavior::Wait(target) => {
                let action: Box<dyn AsyncAction<S>> = Box::new(AsyncWaitState::new(target));

                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::NoChild(action.name(), rx);
                Self::new(action, tx, state)
            }
            Behavior::Invert(child) => {
                let child = Self::from_behavior(*child);
                let child_state = child.state();

                let action = Box::new(AsyncInvertState::new(child));

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

                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::MultipleChildren(action.name(), rx, children_state);
                Self::new(action, tx, state)
            }
        }
    }

    pub async fn run(
        &mut self,
        delta: &mut tokio::sync::watch::Receiver<f64>,
        shared: &mut S,
    ) -> bool {
        let _ignore = self.status.send(Some(Status::Running));
        let success = self.action.run(delta, shared).await;
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
        self.action.reset(shared);
        let _ignore = self.status.send(None);
    }

    pub fn state(&self) -> State {
        self.state.clone()
    }
}

#[cfg(test)]
mod tests {
    use ticked_async_executor::TickedAsyncExecutor;

    use crate::test_async_behavior_interface::{TestAction, TestShared, DELTA};

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

        let mut shared = TestShared;
        let mut delta = executor.tick_channel();
        executor
            .spawn_local("WaitFuture", async move {
                child.run(&mut delta, &mut shared).await;
            })
            .detach();

        executor.wait_till_completed(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }
}
