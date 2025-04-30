use crate::{AsyncAction, async_child::AsyncChild, util::yield_now};

pub struct AsyncLoopState<S> {
    child: AsyncChild<S>,
}

impl<S> AsyncLoopState<S> {
    pub fn new(child: AsyncChild<S>) -> Self {
        Self { child }
    }
}

#[async_trait::async_trait(?Send)]
impl<S> AsyncAction<S> for AsyncLoopState<S> {
    #[tracing::instrument(level = "trace", name = "Loop::run", skip_all, ret)]
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, shared: &S) -> bool {
        loop {
            let _child_status = self.child.run(delta.clone(), shared).await;
            yield_now().await;
            self.child.reset(shared);
        }
    }

    #[tracing::instrument(level = "trace", name = "Loop::reset", skip_all, ret)]
    fn reset(&mut self, shared: &S) {
        self.child.reset(shared);
    }

    fn name(&self) -> &'static str {
        "Loop"
    }
}

#[cfg(test)]
mod tests {
    use behaviortree_common::Behavior;
    use ticked_async_executor::TickedAsyncExecutor;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    use crate::test_async_behavior_interface::TestAction;

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
        let (mut async_loop, state) = AsyncChild::from_behavior_with_state(behavior);

        let executor = TickedAsyncExecutor::default();
        let delta_rx = executor.tick_channel();
        executor
            .spawn_local("_", async move {
                async_loop.run(delta_rx, &()).await;
            })
            .detach();

        for i in 0..6 {
            executor.tick(0.1, None);
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
        let (mut async_loop, state) = AsyncChild::from_behavior_with_state(behavior);

        let executor = TickedAsyncExecutor::default();
        let delta_rx = executor.tick_channel();

        let cancel = tokio_util::sync::CancellationToken::new();
        let cancel_clone = cancel.clone();
        executor
            .spawn_local("_", async move {
                cancel_clone
                    .run_until_cancelled_owned(async {
                        async_loop.run(delta_rx, &()).await;
                    })
                    .await;
            })
            .detach();

        for i in 0..6 {
            executor.tick(0.1, None);
            tracing::info!("{i} : {state:?}");
        }
        cancel.cancel();

        executor.tick(0.1, None);
        assert_eq!(executor.num_tasks(), 0);
    }
}
