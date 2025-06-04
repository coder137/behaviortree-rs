use tokio_util::sync::CancellationToken;

use crate::AsyncAction;
use crate::async_child::AsyncChild;
use crate::util::yield_now;

pub struct AsyncWhileAll<S> {
    conditions: Vec<AsyncChild<S>>,
    child: AsyncChild<S>,
}

impl<S> AsyncWhileAll<S> {
    pub fn new(conditions: Vec<AsyncChild<S>>, child: AsyncChild<S>) -> Self {
        Self { conditions, child }
    }

    async fn handle_child(
        child: &mut AsyncChild<S>,
        delta: tokio::sync::watch::Receiver<f64>,
        shared: &S,
        failure_token: CancellationToken,
    ) {
        let future = async {
            loop {
                let status = child.run(delta.clone(), shared).await;
                if !status {
                    break;
                }
                yield_now().await;
                child.reset(shared);
            }
            failure_token.cancel();
        };
        failure_token.run_until_cancelled(future).await;
    }
}

#[async_trait::async_trait(?Send)]
impl<S> AsyncAction<S> for AsyncWhileAll<S> {
    #[tracing::instrument(level = "trace", name = "WhileAll::run", skip_all, ret)]
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, shared: &S) -> bool {
        let failure_token = tokio_util::sync::CancellationToken::new();

        let mut futures = self
            .conditions
            .iter_mut()
            .map(|child| Self::handle_child(child, delta.clone(), shared, failure_token.clone()))
            .collect::<Vec<_>>();
        futures.push(Self::handle_child(
            &mut self.child,
            delta,
            shared,
            failure_token,
        ));
        futures::future::join_all(futures).await;

        // Reset action only
        self.conditions.iter_mut().for_each(|condition| {
            condition.reset_action(shared);
        });
        self.child.reset_action(shared);
        false
    }

    #[tracing::instrument(level = "trace", name = "WhileAll::reset", skip_all, ret)]
    fn reset(&mut self, shared: &S) {
        self.conditions.iter_mut().for_each(|condition| {
            condition.reset(shared);
        });
        self.child.reset(shared);
    }

    fn name(&self) -> &'static str {
        "WhileAll"
    }
}

#[cfg(test)]
mod tests {
    use behaviortree_common::Behavior;
    use ticked_async_executor::TickedAsyncExecutor;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    use crate::test_async_behavior_interface::{DELTA, TestAction, TestShared};

    use super::*;

    #[test]
    fn test_sequence_success() {
        let _ignore = tracing_subscriber::Registry::default()
            .with(tracing_subscriber::fmt::layer())
            // .with(ForestLayer::default())
            .try_init();

        let behavior = Behavior::WhileAll(
            vec![
                Behavior::Action(TestAction::Success),
                Behavior::Action(TestAction::Success),
            ],
            Behavior::Sequence(vec![
                Behavior::Action(TestAction::Success),
                Behavior::Action(TestAction::Failure),
            ])
            .into(),
        );
        let mut child = AsyncChild::from_behavior(behavior);

        let mut executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let shared = TestShared;

        executor
            .spawn_local("", async move {
                let status = child.run(delta, &shared).await;
                assert!(!status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);

        executor.tick(DELTA, None);
        tracing::info!("TICK 1");
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(DELTA, None);
        tracing::info!("TICK 2");
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(DELTA, None);
        tracing::info!("TICK 3");
        assert_eq!(executor.num_tasks(), 0);
    }
}
