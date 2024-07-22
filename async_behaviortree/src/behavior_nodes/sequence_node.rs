use async_trait::async_trait;

use crate::{AsyncAction, AsyncChild};

pub struct AsyncSequenceState<S> {
    pub children: Vec<AsyncChild<S>>,
}

#[async_trait(?Send)]
impl<S> AsyncAction<S> for AsyncSequenceState<S> {
    async fn run(&mut self, delta: &mut tokio::sync::watch::Receiver<f64>, shared: &mut S) -> bool {
        let last = self.children.len() - 1;
        for (index, child) in self.children.iter_mut().enumerate() {
            let child_status = child.run(delta, shared).await;
            if !child_status {
                return false;
            }
            // Only one child should be run per tick
            // This means that if they are more children after the current child,
            // we must yield back to the executor
            if index != last {
                async_std::task::yield_now().await;
            }
        }
        true
    }

    fn reset(&mut self) {
        self.children.iter_mut().for_each(|child| {
            child.reset();
        });
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
        let mut sequence = AsyncSequenceState {
            children: AsyncChild::from_behaviors(vec![Behavior::Action(TestAction::Success)]),
        };

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
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_failure() {
        let mut sequence = AsyncSequenceState {
            children: AsyncChild::from_behaviors(vec![Behavior::Action(TestAction::Failure)]),
        };

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
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_running() {
        let mut sequence = AsyncSequenceState {
            children: AsyncChild::from_behaviors(vec![Behavior::Action(
                TestAction::SuccessAfter { times: 1 },
            )]),
        };

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
        executor.tick(DELTA);
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_multiple_children() {
        let mut sequence = AsyncSequenceState {
            children: AsyncChild::from_behaviors(vec![
                Behavior::Action(TestAction::Success),
                Behavior::Action(TestAction::Success),
            ]),
        };

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
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_multiple_children_early_failure() {
        let mut sequence = AsyncSequenceState {
            children: AsyncChild::from_behaviors(vec![
                Behavior::Action(TestAction::Success),
                Behavior::Action(TestAction::Failure),
                Behavior::Action(TestAction::Success),
            ]),
        };

        let executor = TickedAsyncExecutor::default();

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("SequenceFuture", async move {
                let status = sequence.run(&mut delta, &mut shared).await;
                assert!(!status);
                sequence.reset();
                let status = sequence.run(&mut delta, &mut shared).await;
                assert!(!status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA);
        executor.tick(DELTA);
        // reset

        executor.tick(DELTA);
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }
}
