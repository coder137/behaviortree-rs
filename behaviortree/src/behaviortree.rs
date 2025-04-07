use crate::{Behavior, Child, State, Status, ToAction};

pub enum BehaviorTreePolicy {
    /// Resets/Reloads the behavior tree once it is completed
    ReloadOnCompletion,
    /// On completion, needs manual reset
    RetainOnCompletion,
}

pub struct BehaviorTree<S> {
    behavior_policy: BehaviorTreePolicy,
    child: Child<S>,
}

impl<S> BehaviorTree<S> {
    pub fn new<A>(behavior: Behavior<A>, behavior_policy: BehaviorTreePolicy) -> Self
    where
        A: ToAction<S>,
        S: 'static,
    {
        let child = Child::from_behavior(behavior);
        Self {
            behavior_policy,
            child,
        }
    }

    pub fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
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

        self.child.tick(dt, shared)
    }

    pub fn state(&self) -> State {
        self.child.state()
    }

    pub fn reset(&mut self) {
        self.child.reset();
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
        let mut tree = BehaviorTree::new(behavior, BehaviorTreePolicy::RetainOnCompletion);

        // For unit tests
        let _state = tree.state();
        assert_eq!(tree.status(), None);

        let mut shared = TestShared::default();

        let status = tree.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = tree.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);

        tree.reset();

        let status = tree.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = tree.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn behavior_tree_with_auto_reset() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);
        let mut tree = BehaviorTree::new(behavior, BehaviorTreePolicy::ReloadOnCompletion);

        let mut shared = TestShared::default();

        let status = tree.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = tree.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);

        // Automatically resets after success (Reload on Completion)

        let status = tree.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = tree.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }
}
