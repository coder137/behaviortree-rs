use async_trait::async_trait;

use crate::{async_child::AsyncChild, behavior_nodes::AsyncAction};

pub struct AsyncInvertState<R> {
    child: AsyncChild<R>,
    completed: bool,
}

impl<R> AsyncInvertState<R> {
    pub fn new(child: AsyncChild<R>) -> Self {
        Self {
            child,
            completed: false,
        }
    }
}

#[async_trait(?Send)]
impl<R> AsyncAction<R> for AsyncInvertState<R> {
    #[tracing::instrument(level = "trace", name = "Invert::run", skip_all, ret)]
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, runner: &mut R) -> bool {
        match self.completed {
            true => unreachable!(),
            false => {}
        }
        let status = !self.child.run(delta, runner).await;
        self.completed = true;
        status
    }

    #[tracing::instrument(level = "trace", name = "Invert::reset", skip_all, ret)]
    fn reset(&mut self, runner: &mut R) {
        self.child.reset(runner);
        self.completed = false;
    }

    fn name(&self) -> &'static str {
        "Invert"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_async_behavior_interface::{DELTA, TestAction, TestRunner};
    use behaviortree_common::Behavior;
    use ticked_async_executor::TickedAsyncExecutor;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    #[test]
    fn test_invert_success() {
        let behavior = Behavior::Invert(Behavior::Action(TestAction::Success).into());
        let mut invert = AsyncChild::from_behavior(behavior);

        let mut executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let mut runner = TestRunner;

        executor
            .spawn_local("InvertFuture", async move {
                let status = invert.run(delta, &mut runner).await;
                assert!(!status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_invert_failure() {
        let behavior = Behavior::Invert(Behavior::Action(TestAction::Failure).into());
        let mut invert = AsyncChild::from_behavior(behavior);

        let mut executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let mut runner = TestRunner;

        executor
            .spawn_local("InvertFuture", async move {
                let status = invert.run(delta, &mut runner).await;
                assert!(status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_invert_running_with_reset() {
        let _ignore = tracing_subscriber::Registry::default()
            .with(tracing_forest::ForestLayer::default())
            .try_init();

        let mut runner = TestRunner;

        let behavior =
            Behavior::Invert(Behavior::Action(TestAction::SuccessAfter { times: 2 }).into());
        let mut invert = AsyncChild::from_behavior(behavior);

        let mut executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        executor
            .spawn_local("InvertFuture", async move {
                let status = invert.run(delta.clone(), &mut runner).await;
                assert!(!status);
                invert.reset(&mut runner);
                let status = invert.run(delta, &mut runner).await;
                assert!(!status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        executor.tick(DELTA, None);

        // Reset here

        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }
}
