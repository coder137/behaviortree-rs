use behaviortree_common::{Behavior, State, Status};

use crate::{action_type::ActionType, child::Child};

pub enum BehaviorTreePolicy {
    /// Resets/Reloads the behavior tree once it is completed
    ReloadOnCompletion,
    /// On completion, needs manual reset
    RetainOnCompletion,
}

pub struct BehaviorTree<S> {
    behavior_policy: BehaviorTreePolicy,
    child: Child<S>,
    shared: S,
}

impl<S> BehaviorTree<S> {
    pub fn new<A>(behavior: Behavior<A>, behavior_policy: BehaviorTreePolicy, shared: S) -> Self
    where
        A: Into<ActionType<S>>,
        S: 'static,
    {
        let child = Child::from_behavior(behavior);
        Self {
            behavior_policy,
            child,
            shared,
        }
    }

    #[tracing::instrument(level = "trace", name = "BehaviorTree::tick", skip(self), ret)]
    pub fn tick(&mut self, dt: f64) -> Status {
        if let Some(status) = self.child.status() {
            if status != Status::Running {
                match self.behavior_policy {
                    BehaviorTreePolicy::ReloadOnCompletion => {
                        self.reset();
                        // Ticks the action below
                    }
                    BehaviorTreePolicy::RetainOnCompletion => {
                        // Do nothing!
                        // `status` returns the already completed value
                        return status;
                    }
                }
            }
        }

        let shared = &mut self.shared;
        self.child.tick(dt, shared)
    }

    pub fn state(&self) -> State {
        self.child.state()
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
    use super::*;
    use crate::test_behavior_interface::{TestAction, TestShared};

    #[test]
    fn behavior_tree_with_reset() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);
        let mut tree =
            BehaviorTree::new(behavior, BehaviorTreePolicy::RetainOnCompletion, TestShared);

        // For unit tests
        let _state = tree.state();
        assert_eq!(tree.status(), None);

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Running);

        let status = tree.tick(0.1);
        assert_eq!(status, Status::Success);

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
        let mut tree =
            BehaviorTree::new(behavior, BehaviorTreePolicy::ReloadOnCompletion, TestShared);

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
