use behaviortree_common::{Behavior, State, Status};

use crate::async_action_type::AsyncActionType;

use crate::behavior_nodes::{
    AsyncInvertState, AsyncSelectState, AsyncSequenceState, AsyncWaitState, AsyncWhileAll,
};

pub struct AsyncChild<S> {
    action_type: AsyncActionType<S>,
    status: tokio::sync::watch::Sender<Option<Status>>,
}

impl<S> AsyncChild<S> {
    pub fn new(
        action_type: AsyncActionType<S>,
        status: tokio::sync::watch::Sender<Option<Status>>,
    ) -> Self {
        Self {
            action_type,
            status,
        }
    }

    #[cfg(test)]
    pub fn from_behavior<A>(behavior: Behavior<A>) -> Self
    where
        A: Into<AsyncActionType<S>>,
        S: 'static,
    {
        let (child, _state) = Self::from_behavior_with_state(behavior);
        child
    }

    #[cfg(test)]
    pub fn from_behavior_with_state<A>(behavior: Behavior<A>) -> (Self, State)
    where
        A: Into<AsyncActionType<S>>,
        S: 'static,
    {
        let mut statuses = vec![];
        Self::from_behavior_with_state_and_status(behavior, &mut statuses)
    }

    pub fn from_behavior_with_state_and_status<A>(
        behavior: Behavior<A>,
        statuses: &mut Vec<tokio::sync::watch::Sender<Option<Status>>>,
    ) -> (Self, State)
    where
        A: Into<AsyncActionType<S>>,
        S: 'static,
    {
        match behavior {
            Behavior::Action(action) => {
                let action = action.into();

                let (tx, rx) = tokio::sync::watch::channel(None);
                statuses.push(tx.clone());

                let state = State::NoChild(action.name(), rx);
                (Self::new(action, tx), state)
            }
            Behavior::Wait(target) => {
                let action = Box::new(AsyncWaitState::new(target));
                let action = AsyncActionType::Async(action);

                let (tx, rx) = tokio::sync::watch::channel(None);
                statuses.push(tx.clone());

                let state = State::NoChild(action.name(), rx);
                (Self::new(action, tx), state)
            }
            Behavior::Invert(child) => {
                let (child, child_state) =
                    Self::from_behavior_with_state_and_status(*child, statuses);

                let action = Box::new(AsyncInvertState::new(child));
                let action = AsyncActionType::Async(action);

                let (tx, rx) = tokio::sync::watch::channel(None);
                statuses.push(tx.clone());

                let state = State::SingleChild(action.name(), rx, child_state.into());
                (Self::new(action, tx), state)
            }
            Behavior::Sequence(children) => {
                let (children, children_states): (Vec<_>, Vec<_>) = children
                    .into_iter()
                    .map(|child| AsyncChild::from_behavior_with_state_and_status(child, statuses))
                    .unzip();
                let children_states = std::rc::Rc::from_iter(children_states);

                let action = Box::new(AsyncSequenceState::new(children));
                let action = AsyncActionType::Async(action);

                let (tx, rx) = tokio::sync::watch::channel(None);
                statuses.push(tx.clone());

                let state = State::MultipleChildren(action.name(), rx, children_states);
                (Self::new(action, tx), state)
            }
            Behavior::Select(children) => {
                let (children, children_states): (Vec<_>, Vec<_>) = children
                    .into_iter()
                    .map(|child| AsyncChild::from_behavior_with_state_and_status(child, statuses))
                    .unzip();
                let children_states = std::rc::Rc::from_iter(children_states);

                let action = Box::new(AsyncSelectState::new(children));
                let action = AsyncActionType::Async(action);

                let (tx, rx) = tokio::sync::watch::channel(None);
                statuses.push(tx.clone());

                let state = State::MultipleChildren(action.name(), rx, children_states);
                (Self::new(action, tx), state)
            }
            Behavior::WhileAll(conditions, child) => {
                let (conditions, mut children_states): (Vec<_>, Vec<_>) = conditions
                    .into_iter()
                    .map(|condition| Self::from_behavior_with_state_and_status(condition, statuses))
                    .unzip();

                //
                let (child, child_state) =
                    Self::from_behavior_with_state_and_status(*child, statuses);
                children_states.push(child_state);

                let children_states = std::rc::Rc::from_iter(children_states);

                let action = Box::new(AsyncWhileAll::new(conditions, child));
                let action = AsyncActionType::Async(action);

                let (tx, rx) = tokio::sync::watch::channel(None);
                statuses.push(tx.clone());

                let state = State::MultipleChildren(action.name(), rx, children_states);
                (Self::new(action, tx), state)
            }
        }
    }

    pub async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, shared: &S) -> bool {
        self.status.send_replace(Some(Status::Running));
        let success = self.action_type.run(delta, shared).await;
        let status = if success {
            Status::Success
        } else {
            Status::Failure
        };
        self.status.send_replace(Some(status));
        success
    }

    // Reset action and status
    pub fn reset(&mut self, shared: &S) {
        self.action_type.reset(shared);
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
        let mut executor = TickedAsyncExecutor::default();

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
