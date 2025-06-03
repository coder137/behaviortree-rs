use tracing::Instrument;

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
}

#[async_trait::async_trait(?Send)]
impl<S> AsyncAction<S> for AsyncWhileAll<S> {
    #[tracing::instrument(level = "trace", name = "WhileAll::run", skip_all, ret)]
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, shared: &S) -> bool {
        let failure_token = tokio_util::sync::CancellationToken::new();

        let delta_clone = delta.clone();
        let conditions_future = async {
            loop {
                // Check conditions
                let mut conditions_status = true;
                for condition in self.conditions.iter_mut() {
                    let status = condition.run(delta_clone.clone(), shared).await;
                    if !status {
                        conditions_status = false;
                        break;
                    }
                }

                // Reset
                self.conditions.iter_mut().for_each(|condition| {
                    condition.reset(shared);
                });

                //
                if !conditions_status {
                    break;
                }
                yield_now().await;
            }
            failure_token.cancel();
        }
        .instrument(tracing::trace_span!("ConditionsFuture"));

        let child_future = async {
            loop {
                let status = self.child.run(delta.clone(), shared).await;

                // Reset
                self.child.reset(shared);

                //
                if !status {
                    break;
                }
                yield_now().await;
            }
            failure_token.cancel();
        }
        .instrument(tracing::trace_span!("ChildFuture"));

        tokio::join!(
            failure_token.run_until_cancelled(conditions_future),
            failure_token.run_until_cancelled(child_future)
        );

        // NOTE: This reset is important here since the condition task could return failure abruptly
        // In that case the child task should stop gracefully
        self.reset(shared);
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
            vec![TestAction::Success, TestAction::Success],
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
        assert_eq!(executor.num_tasks(), 0);
    }
}
