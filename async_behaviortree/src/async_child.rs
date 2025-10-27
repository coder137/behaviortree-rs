use behaviortree_common::{Behavior, State, Status};

use crate::behavior_nodes::{
    AsyncAction, AsyncActionState, AsyncInvertState, AsyncSelectState, AsyncSequenceState,
    AsyncWaitState, AsyncWhileAll,
};
use crate::{AsyncActionName, AsyncActionRunner};

pub struct AsyncChild<R> {
    action_type: Box<dyn AsyncAction<R>>,
    status: tokio::sync::watch::Sender<Option<Status>>,
}

impl<R> AsyncChild<R> {
    pub fn new(
        action_type: Box<dyn AsyncAction<R>>,
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
        A: AsyncActionName + 'static,
        R: AsyncActionRunner<A> + 'static,
    {
        let (child, _state) = Self::from_behavior_with_state(behavior);
        child
    }

    pub fn from_behavior_with_state<A>(behavior: Behavior<A>) -> (Self, State)
    where
        A: AsyncActionName + 'static,
        R: AsyncActionRunner<A> + 'static,
    {
        match behavior {
            Behavior::Action(action) => {
                let action: Box<dyn AsyncAction<R>> = Box::new(AsyncActionState::new(action));

                let (tx, rx) = tokio::sync::watch::channel(None);

                let state = State::NoChild(action.name(), rx);
                (Self::new(action, tx), state)
            }
            Behavior::Wait(target) => {
                let action: Box<dyn AsyncAction<R>> = Box::new(AsyncWaitState::new(target));

                let (tx, rx) = tokio::sync::watch::channel(None);

                let state = State::NoChild(action.name(), rx);
                (Self::new(action, tx), state)
            }
            Behavior::Invert(child) => {
                let (child, child_state) = Self::from_behavior_with_state(*child);

                let action = Box::new(AsyncInvertState::new(child));

                let (tx, rx) = tokio::sync::watch::channel(None);

                let state = State::SingleChild(action.name(), rx, child_state.into());
                (Self::new(action, tx), state)
            }
            Behavior::Sequence(children) => {
                let (children, children_states): (Vec<_>, Vec<_>) = children
                    .into_iter()
                    .map(|child| AsyncChild::from_behavior_with_state(child))
                    .unzip();
                let children_states = std::rc::Rc::from_iter(children_states);

                let action = Box::new(AsyncSequenceState::new(children));

                let (tx, rx) = tokio::sync::watch::channel(None);

                let state = State::MultipleChildren(action.name(), rx, children_states);
                (Self::new(action, tx), state)
            }
            Behavior::Select(children) => {
                let (children, children_states): (Vec<_>, Vec<_>) = children
                    .into_iter()
                    .map(|child| AsyncChild::from_behavior_with_state(child))
                    .unzip();
                let children_states = std::rc::Rc::from_iter(children_states);

                let action = Box::new(AsyncSelectState::new(children));

                let (tx, rx) = tokio::sync::watch::channel(None);

                let state = State::MultipleChildren(action.name(), rx, children_states);
                (Self::new(action, tx), state)
            }
            Behavior::WhileAll(conditions, child) => {
                let (conditions, mut children_states): (Vec<_>, Vec<_>) = conditions
                    .into_iter()
                    .map(|condition| Self::from_behavior_with_state(condition))
                    .unzip();

                //
                let (child, child_state) = Self::from_behavior_with_state(*child);
                children_states.push(child_state);

                let children_states = std::rc::Rc::from_iter(children_states);

                let action = Box::new(AsyncWhileAll::new(conditions, child));

                let (tx, rx) = tokio::sync::watch::channel(None);

                let state = State::MultipleChildren(action.name(), rx, children_states);
                (Self::new(action, tx), state)
            }
        }
    }

    pub async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, runner: &mut R) -> bool {
        self.status.send_replace(Some(Status::Running));
        let success = self.action_type.run(delta, runner).await;
        let status = if success {
            Status::Success
        } else {
            Status::Failure
        };
        self.status.send_replace(Some(status));
        success
    }

    pub fn reset(&mut self, runner: &mut R) {
        self.status.send_replace(None);
        self.action_type.reset(runner);
    }
}

#[cfg(test)]
mod tests {
    use ticked_async_executor::TickedAsyncExecutor;

    use crate::test_async_behavior_interface::{DELTA, TestAction, TestRunner};

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

        let mut runner = TestRunner;
        let delta = executor.tick_channel();
        executor
            .spawn_local("WaitFuture", async move {
                child.run(delta, &mut runner).await;
            })
            .detach();

        executor.wait_till_completed(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }
}
