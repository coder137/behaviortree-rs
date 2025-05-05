use behaviortree_common::Status;

use crate::{SyncAction, child::Child};

pub struct LoopState<S> {
    child: Child<S>,
}

impl<S> LoopState<S> {
    pub fn new(child: Child<S>) -> Self {
        Self { child }
    }
}

impl<S> SyncAction<S> for LoopState<S> {
    #[tracing::instrument(level = "trace", name = "Loop::tick", skip_all, ret)]
    fn tick(&mut self, delta: f64, shared: &mut S) -> Status {
        let child_status = self.child.status();
        if let Some(child_status) = child_status {
            if child_status != Status::Running {
                self.child.reset(shared);
            }
        }

        let _child_status = self.child.tick(delta, shared);
        Status::Running
    }

    fn reset(&mut self, shared: &mut S) {
        self.child.reset(shared);
    }

    fn name(&self) -> &'static str {
        "Loop"
    }
}

#[cfg(test)]
mod tests {
    use behaviortree_common::Behavior;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    use crate::test_behavior_interface::{TestAction, TestShared};

    use super::*;

    #[test]
    fn test_loop_all_success() {
        let _ignore = tracing_subscriber::Registry::default()
            .with(tracing_forest::ForestLayer::default())
            .try_init();

        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);
        let behavior = Behavior::Loop(behavior.into());
        let (mut child, state) = Child::from_behavior_with_state(behavior);

        let mut shared = TestShared;
        for i in 0..6 {
            child.tick(0.1, &mut shared);
            tracing::info!("{i} : {state:?}");
        }
    }

    #[test]
    fn test_loop_with_failure() {
        let _ignore = tracing_subscriber::Registry::default()
            .with(tracing_forest::ForestLayer::default())
            .try_init();

        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Failure),
            Behavior::Action(TestAction::Success),
        ]);
        let behavior = Behavior::Loop(behavior.into());
        let (mut child, state) = Child::from_behavior_with_state(behavior);

        let mut shared = TestShared;
        for i in 0..6 {
            child.tick(0.1, &mut shared);
            tracing::info!("{i} : {state:?}");
        }
    }
}
