use std::rc::Rc;

use tokio_util::sync::CancellationToken;

use crate::async_child::AsyncChild;
use crate::behavior_nodes::AsyncAction;
use crate::util::yield_now;

pub struct AsyncWhileAll<R> {
    conditions: Vec<AsyncChild<R>>,
    child: AsyncChild<R>,
}

impl<R> AsyncWhileAll<R> {
    pub fn new(conditions: Vec<AsyncChild<R>>, child: AsyncChild<R>) -> Self {
        Self { conditions, child }
    }

    async fn handle_child(
        child: &mut AsyncChild<R>,
        delta: tokio::sync::watch::Receiver<f64>,
        runner: Rc<tokio::sync::Mutex<&mut R>>,
        failure_token: CancellationToken,
        allow_failure: bool,
    ) {
        let future = async {
            loop {
                let status = {
                    let mut runner_lock = runner.lock().await;
                    child.run(delta.clone(), *runner_lock).await
                };

                // If we allow failure + the child returns failure, then we exit
                if allow_failure && !status {
                    break;
                }
                yield_now().await;

                {
                    let mut runner_lock = runner.lock().await;
                    child.reset(*runner_lock);
                }
            }
            failure_token.cancel();
        };
        failure_token.run_until_cancelled(future).await;
    }
}

#[async_trait::async_trait(?Send)]
impl<R> AsyncAction<R> for AsyncWhileAll<R> {
    #[tracing::instrument(level = "trace", name = "WhileAll::run", skip_all, ret)]
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, runner: &mut R) -> bool {
        let failure_token = tokio_util::sync::CancellationToken::new();

        let runner = Rc::new(tokio::sync::Mutex::new(runner));

        let mut futures = self
            .conditions
            .iter_mut()
            .map(|child| {
                Self::handle_child(
                    child,
                    delta.clone(),
                    runner.clone(),
                    failure_token.clone(),
                    true,
                )
            })
            .collect::<Vec<_>>();
        // NOTE: child should not be able to mark failure even if it fails
        futures.push(Self::handle_child(
            &mut self.child,
            delta,
            runner,
            failure_token,
            false,
        ));
        futures::future::join_all(futures).await;
        false
    }

    #[tracing::instrument(level = "trace", name = "WhileAll::reset", skip_all)]
    fn reset(&mut self, runner: &mut R) {
        self.conditions.iter_mut().for_each(|condition| {
            condition.reset(runner);
        });
        self.child.reset(runner);
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

    use crate::test_async_behavior_interface::{DELTA, TestAction, TestRunner};

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
                Behavior::Action(TestAction::Failure),
            ],
            Behavior::Sequence(vec![
                Behavior::Action(TestAction::Success),
                Behavior::Action(TestAction::Success),
            ])
            .into(),
        );
        let mut child = AsyncChild::from_behavior(behavior);

        let mut executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let mut runner = TestRunner;

        executor
            .spawn_local("", async move {
                let status = child.run(delta, &mut runner).await;
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
