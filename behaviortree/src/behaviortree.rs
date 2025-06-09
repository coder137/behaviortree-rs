use behaviortree_common::{Behavior, State, Status};

use crate::{action_type::ActionType, child::Child};

pub struct BehaviorTree<S> {
    child: Child<S>,
    should_loop: bool,
    state: State,
    shared: S,
    statuses: Vec<tokio::sync::watch::Sender<Option<Status>>>,
}

impl<S> BehaviorTree<S> {
    pub fn new<A>(behavior: Behavior<A>, should_loop: bool, shared: S) -> Self
    where
        A: Into<ActionType<S>>,
        S: 'static,
    {
        let mut statuses = vec![];
        let (child, state) = Child::from_behavior_with_state_and_status(behavior, &mut statuses);
        Self {
            child,
            should_loop,
            state,
            shared,
            statuses,
        }
    }

    #[tracing::instrument(level = "trace", name = "BehaviorTree::tick", skip(self), ret)]
    pub fn tick(&mut self, dt: f64) -> Status {
        if let Some(status) = self.child.status() {
            let completed = status != Status::Running;
            if completed {
                if self.should_loop {
                    self.reset();
                } else {
                    return status;
                }
            }
        }

        self.child.tick(dt, &mut self.shared)
    }

    pub fn state(&self) -> State {
        self.state.clone()
    }

    #[tracing::instrument(level = "trace", name = "BehaviorTree::reset", skip(self))]
    pub fn reset(&mut self) {
        self.statuses.iter_mut().for_each(|status| {
            status.send_replace(None);
        });
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
        let mut tree = BehaviorTree::new(behavior, false, TestShared);
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
        let mut tree = BehaviorTree::new(behavior, true, TestShared);

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Running);

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Success);

        // Automatically resets after success (Reload on Completion)

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Running);

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Success);
    }
}
