use behaviortree_common::{Behavior, State, Status};

use crate::{action_type::ActionType, child::Child};

pub struct BehaviorTree<S> {
    child: Child<S>,
    state: State,
    shared: S,
}

impl<S> BehaviorTree<S> {
    pub fn new<A>(behavior: Behavior<A>, shared: S) -> Self
    where
        A: Into<ActionType<S>>,
        S: 'static,
    {
        let (child, state) = Child::from_behavior_with_state(behavior);
        Self {
            child,
            state,
            shared,
        }
    }

    #[tracing::instrument(level = "trace", name = "BehaviorTree::tick", skip(self), ret)]
    pub fn tick(&mut self, dt: f64) -> Status {
        if let Some(status) = self.child.status() {
            if status != Status::Running {
                return status;
            }
        }

        let shared = &mut self.shared;
        self.child.tick(dt, shared)
    }

    pub fn state(&self) -> State {
        self.state.clone()
    }

    #[tracing::instrument(level = "trace", name = "BehaviorTree::reset", skip(self))]
    pub fn reset(&mut self) {
        self.child.reset(&mut self.shared);
    }

    pub fn status(&self) -> Option<Status> {
        self.child.status()
    }
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    use super::*;
    use crate::test_behavior_interface::{TestAction, TestShared};

    #[test]
    fn behavior_tree_with_reset() {
        let _ignore = tracing_subscriber::Registry::default()
            .with(tracing_forest::ForestLayer::default())
            .try_init();

        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);
        let mut tree = BehaviorTree::new(behavior, TestShared);
        let state = tree.state();

        // For unit tests
        let _state = tree.state();
        assert_eq!(tree.status(), None);
        tracing::info!("State: {state:?}");

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Running);
        tracing::info!("State: {state:?}");

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Success);
        tracing::info!("State: {state:?}");

        // Ticking again returns the same status
        let status = tree.tick(0.1);
        assert_eq!(status, Status::Success);
        tracing::info!("State: {state:?}");

        tree.reset();

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Running);

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn behavior_tree_with_auto_reset() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);
        let behavior = Behavior::Loop(behavior.into());
        let mut tree = BehaviorTree::new(behavior, TestShared);

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Running);

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Running);

        // Automatically resets after success (Reload on Completion)

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Running);

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Running);
    }
}
