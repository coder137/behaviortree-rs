use behaviortree_common::{Behavior, State, Status};

use crate::{behavior_nodes::*, SyncAction, ToAction};

pub struct Child<S> {
    action: Box<dyn SyncAction<S>>,
    status: tokio::sync::watch::Sender<Option<Status>>,
    state: State,
}

impl<S> Child<S> {
    pub fn new(
        action: Box<dyn SyncAction<S>>,
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
        A: ToAction<S>,
        S: 'static,
    {
        match behavior {
            Behavior::Action(action) => {
                let action = action.to_action();
                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::NoChild(action.name(), rx);

                Self::new(action, tx, state)
            }
            Behavior::Wait(target) => {
                let action: Box<dyn SyncAction<S>> = Box::new(WaitState::new(target));
                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::NoChild(action.name(), rx);

                Self::new(action, tx, state)
            }
            Behavior::Invert(child) => {
                let child = Child::from_behavior(*child);
                let child_state = child.state();

                let action = InvertState::new(child);
                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::SingleChild(action.name(), rx, child_state.into());

                Self::new(Box::new(action), tx, state)
            }
            Behavior::Sequence(children) => {
                let children = children
                    .into_iter()
                    .map(|child| Child::from_behavior(child))
                    .collect::<Vec<_>>();
                let children_states = children.iter().map(|child| child.state());
                let children_states = std::rc::Rc::from_iter(children_states);

                let action = SequenceState::new(children);
                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::MultipleChildren(action.name(), rx, children_states);

                Self::new(Box::new(action), tx, state)
            }
            Behavior::Select(children) => {
                let children = children
                    .into_iter()
                    .map(|child| Child::from_behavior(child))
                    .collect::<Vec<_>>();
                let children_states = children.iter().map(|child| child.state());
                let children_states = std::rc::Rc::from_iter(children_states);

                let action = SelectState::new(children);
                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::MultipleChildren(action.name(), rx, children_states);

                Self::new(Box::new(action), tx, state)
            }
        }
    }

    pub fn tick(&mut self, delta: f64, shared: &mut S) -> Status {
        let status = self.action.tick(delta, shared);
        let _ignore = self.status.send(Some(status));
        status
    }

    pub fn reset(&mut self, shared: &mut S) {
        self.action.reset(shared);
        let _ignore = self.status.send(None);
    }

    pub fn status(&self) -> Option<Status> {
        *self.status.borrow()
    }

    pub fn state(&self) -> State {
        self.state.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use behaviortree_common::Behavior;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    use crate::test_behavior_interface::{TestAction, TestShared};

    #[test]
    fn test_basic_behavior() {
        let _ignore = tracing_subscriber::Registry::default()
            .with(tracing_forest::ForestLayer::default())
            .try_init();

        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Wait(10.0),
            Behavior::Action(TestAction::Success),
            Behavior::Invert(Behavior::Action(TestAction::Failure).into()),
            Behavior::Action(TestAction::Success),
        ]);

        let mut child = Child::from_behavior(behavior);
        let state = child.state();

        let mut shared = TestShared;

        loop {
            let status = child.tick(1.0, &mut shared);
            tracing::info!("State:\n{:#?}", state);
            if status != Status::Running {
                break;
            }
        }
    }
}
