use behaviortree_common::{Behavior, State, Status};

use crate::{action_type::ActionType, behavior_nodes::*};

pub struct Child<S> {
    action: ActionType<S>,
    status: tokio::sync::watch::Sender<Option<Status>>,
}

impl<S> Child<S> {
    pub fn new(action: ActionType<S>, status: tokio::sync::watch::Sender<Option<Status>>) -> Self {
        Self { action, status }
    }

    #[cfg(test)]
    pub fn from_behavior<A>(behavior: Behavior<A>) -> Self
    where
        A: Into<ActionType<S>>,
        S: 'static,
    {
        let (child, _state) = Self::from_behavior_with_state(behavior);
        child
    }

    #[cfg(test)]
    pub fn from_behavior_with_state<A>(behavior: Behavior<A>) -> (Self, State)
    where
        A: Into<ActionType<S>>,
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
        A: Into<ActionType<S>>,
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
                let action = Box::new(WaitState::new(target));
                let action = ActionType::Sync(action);

                let (tx, rx) = tokio::sync::watch::channel(None);
                statuses.push(tx.clone());

                let state = State::NoChild(action.name(), rx);
                (Self::new(action, tx), state)
            }
            Behavior::Invert(child) => {
                let (child, child_state) =
                    Self::from_behavior_with_state_and_status(*child, statuses);

                let action = Box::new(InvertState::new(child));
                let action = ActionType::Sync(action);

                let (tx, rx) = tokio::sync::watch::channel(None);
                statuses.push(tx.clone());

                let state = State::SingleChild(action.name(), rx, child_state.into());
                (Self::new(action, tx), state)
            }
            Behavior::Sequence(children) => {
                let (children, children_state): (Vec<_>, Vec<_>) = children
                    .into_iter()
                    .map(|child| Child::from_behavior_with_state_and_status(child, statuses))
                    .unzip();
                let children_state = std::rc::Rc::from_iter(children_state);

                let action = Box::new(SequenceState::new(children));
                let action = ActionType::Sync(action);

                let (tx, rx) = tokio::sync::watch::channel(None);
                statuses.push(tx.clone());

                let state = State::MultipleChildren(action.name(), rx, children_state);
                (Self::new(action, tx), state)
            }
            Behavior::Select(children) => {
                let (children, children_state): (Vec<_>, Vec<_>) = children
                    .into_iter()
                    .map(|child| Child::from_behavior_with_state_and_status(child, statuses))
                    .unzip();
                let children_state = std::rc::Rc::from_iter(children_state);

                let action = Box::new(SelectState::new(children));
                let action = ActionType::Sync(action);

                let (tx, rx) = tokio::sync::watch::channel(None);
                statuses.push(tx.clone());

                let state = State::MultipleChildren(action.name(), rx, children_state);
                (Self::new(action, tx), state)
            }
            Behavior::WhileAll(_conditions, _child) => {
                todo!()
            }
        }
    }

    pub fn tick(&mut self, delta: f64, shared: &mut S) -> Status {
        let status = self.action.tick(delta, shared);
        self.status.send_replace(Some(status));
        status
    }

    pub fn reset(&mut self, shared: &mut S) {
        self.action.reset(shared);
    }

    pub fn status(&self) -> Option<Status> {
        *self.status.borrow()
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

        let (mut child, state) = Child::from_behavior_with_state(behavior);
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
