use async_trait::async_trait;

use crate::{AsyncAction, AsyncChild};

pub struct AsyncSequenceState<S> {
    children: Vec<AsyncChild<S>>,
    completed: bool,
}

impl<S> AsyncSequenceState<S> {
    pub fn new(children: Vec<AsyncChild<S>>) -> Self {
        Self {
            children,
            completed: false,
        }
    }
}

#[async_trait(?Send)]
impl<S> AsyncAction<S> for AsyncSequenceState<S> {
    async fn run(&mut self, delta: &mut tokio::sync::watch::Receiver<f64>, shared: &mut S) -> bool {
        match self.completed {
            true => {
                unreachable!()
            }
            false => {}
        }
        let mut status = true;
        let last = self.children.len() - 1;
        for (index, child) in self.children.iter_mut().enumerate() {
            let child_status = child.run(delta, shared).await;
            if !child_status {
                status = false;
                break;
            }
            // Only one child should be run per tick
            // This means that if they are more children after the current child,
            // we must yield back to the executor
            if index != last {
                tokio::task::yield_now().await;
            }
        }
        self.completed = true;
        status
    }

    fn reset(&mut self, shared: &mut S) {
        self.children
            .iter_mut()
            .for_each(|child| child.reset(shared));
        self.completed = false;
    }

    fn name(&self) -> &'static str {
        "Sequence"
    }
}

#[cfg(test)]
mod tests {
    use behaviortree_common::Behavior;
    use ticked_async_executor::TickedAsyncExecutor;

    use crate::test_async_behavior_interface::{TestAction, TestShared, DELTA};

    use super::*;

    #[test]
    fn test_sequence_success() {
        let behavior = Behavior::Sequence(vec![Behavior::Action(TestAction::Success)]);
        let mut sequence = AsyncChild::from_behavior(behavior);

        let executor = TickedAsyncExecutor::default();

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("SequenceFuture", async move {
                let status = sequence.run(&mut delta, &mut shared).await;
                assert!(status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_failure() {
        let behavior = Behavior::Sequence(vec![Behavior::Action(TestAction::Failure)]);
        let mut sequence = AsyncChild::from_behavior(behavior);

        let executor = TickedAsyncExecutor::default();

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("SequenceFuture", async move {
                let status = sequence.run(&mut delta, &mut shared).await;
                assert!(!status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_running() {
        let behavior = Behavior::Sequence(vec![Behavior::Action(TestAction::SuccessAfter {
            times: 1,
        })]);
        let mut sequence = AsyncChild::from_behavior(behavior);

        let executor = TickedAsyncExecutor::default();

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("SequenceFuture", async move {
                let status = sequence.run(&mut delta, &mut shared).await;
                assert!(status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_multiple_children() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);
        let mut sequence = AsyncChild::from_behavior(behavior);

        let executor = TickedAsyncExecutor::default();

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("SequenceFuture", async move {
                let status = sequence.run(&mut delta, &mut shared).await;
                assert!(status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_multiple_children_early_failure() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Failure),
            Behavior::Action(TestAction::Success),
        ]);
        let mut sequence = AsyncChild::from_behavior(behavior);

        let executor = TickedAsyncExecutor::default();

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("SequenceFuture", async move {
                let status = sequence.run(&mut delta, &mut shared).await;
                assert!(!status);
                sequence.reset(&mut shared);
                let status = sequence.run(&mut delta, &mut shared).await;
                assert!(!status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        // reset

        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }
}
