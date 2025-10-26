use async_trait::async_trait;

use crate::{async_child::AsyncChild, behavior_nodes::AsyncAction, util::yield_now};

pub struct AsyncSequenceState<R> {
    children: Vec<AsyncChild<R>>,
    completed: bool,
}

impl<R> AsyncSequenceState<R> {
    pub fn new(children: Vec<AsyncChild<R>>) -> Self {
        Self {
            children,
            completed: false,
        }
    }
}

#[async_trait(?Send)]
impl<R> AsyncAction<R> for AsyncSequenceState<R> {
    #[tracing::instrument(level = "trace", name = "Sequence::run", skip_all, ret)]
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, runner: &mut R) -> bool {
        match self.completed {
            true => {
                unreachable!()
            }
            false => {}
        }
        let mut status = true;
        let last = self.children.len() - 1;
        for (index, child) in self.children.iter_mut().enumerate() {
            let child_status = child.run(delta.clone(), runner).await;
            if !child_status {
                status = false;
                break;
            }
            // Only one child should be run per tick
            // This means that if they are more children after the current child,
            // we must yield back to the executor
            if index != last {
                yield_now().await;
            }
        }
        self.completed = true;
        status
    }

    #[tracing::instrument(level = "trace", name = "Sequence::reset", skip_all, ret)]
    fn reset(&mut self, runner: &mut R) {
        self.children
            .iter_mut()
            .for_each(|child| child.reset(runner));
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

    use crate::test_async_behavior_interface::{DELTA, TestAction, TestRunner};

    use super::*;

    #[test]
    fn test_sequence_success() {
        let behavior = Behavior::Sequence(vec![Behavior::Action(TestAction::Success)]);
        let mut sequence = AsyncChild::from_behavior(behavior);

        let mut executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let mut runner = TestRunner;

        executor
            .spawn_local("SequenceFuture", async move {
                let status = sequence.run(delta, &mut runner).await;
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

        let mut executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let mut runner = TestRunner;

        executor
            .spawn_local("SequenceFuture", async move {
                let status = sequence.run(delta, &mut runner).await;
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

        let mut executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let mut runner = TestRunner;

        executor
            .spawn_local("SequenceFuture", async move {
                let status = sequence.run(delta, &mut runner).await;
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

        let mut executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let mut runner = TestRunner;

        executor
            .spawn_local("SequenceFuture", async move {
                let status = sequence.run(delta, &mut runner).await;
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

        let mut executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let mut runner = TestRunner;

        executor
            .spawn_local("SequenceFuture", async move {
                let status = sequence.run(delta.clone(), &mut runner).await;
                assert!(!status);
                sequence.reset(&mut runner);
                let status = sequence.run(delta, &mut runner).await;
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
