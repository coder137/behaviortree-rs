use std::future::Future;

use crate::AsyncChild;
use crate::Behavior;

use crate::ToAsyncAction;

pub enum AsyncBehaviorTreePolicy {
    /// Resets/Reloads the behavior tree once it is completed
    ReloadOnCompletion,
    /// On completion, needs manual reset
    RetainOnCompletion,
}

pub struct AsyncBehaviorController {
    pub reset_tx: tokio::sync::mpsc::Sender<()>,
    pub shutdown_tx: tokio::sync::mpsc::Sender<()>,
}

pub struct AsyncBehaviorTree;

impl AsyncBehaviorTree {
    pub fn new<A, S>(
        behavior: Behavior<A>,
        behavior_policy: AsyncBehaviorTreePolicy,
        mut delta: tokio::sync::watch::Receiver<f64>,
        mut shared: S,
    ) -> (impl Future<Output = ()>, AsyncBehaviorController)
    where
        A: ToAsyncAction<S>,
        S: 'static,
    {
        let mut child = AsyncChild::from_behavior(behavior);
        let (reset_tx, mut reset_rx) = tokio::sync::mpsc::channel::<()>(1);
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

        let behavior_fut = async move {
            loop {
                tokio::select! {
                    _ = child.run(&mut delta, &mut shared) => {
                        match behavior_policy {
                            AsyncBehaviorTreePolicy::ReloadOnCompletion => {child.reset();},
                            AsyncBehaviorTreePolicy::RetainOnCompletion => {break;},
                        }
                    }
                    _ = reset_rx.recv() => {
                        child.reset();
                        match behavior_policy {
                            AsyncBehaviorTreePolicy::ReloadOnCompletion => {},
                            AsyncBehaviorTreePolicy::RetainOnCompletion => {break;},
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        child.reset();
                        break;
                    }
                }
            }
        };

        (
            behavior_fut,
            AsyncBehaviorController {
                reset_tx,
                shutdown_tx,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use behaviortree_common::Behavior;
    use ticked_async_executor::TickedAsyncExecutor;

    use crate::test_async_behavior_interface::{TestAction, TestShared, DELTA};

    #[test]
    fn test_async_behaviortree() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);

        let executor = TickedAsyncExecutor::default();
        let shared = TestShared;

        let (behaviortree_future, _controller) = AsyncBehaviorTree::new(
            behavior,
            AsyncBehaviorTreePolicy::RetainOnCompletion,
            executor.tick_channel(),
            shared,
        );

        executor
            .spawn_local("AsyncBehaviorTreeFuture", behaviortree_future)
            .detach();

        executor.tick(DELTA);
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_async_behaviortree_early_reset() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);

        let executor = TickedAsyncExecutor::default();
        let shared = TestShared;

        let (behaviortree_future, controller) = AsyncBehaviorTree::new(
            behavior,
            AsyncBehaviorTreePolicy::RetainOnCompletion,
            executor.tick_channel(),
            shared,
        );

        executor
            .spawn_local("AsyncBehaviorTreeFuture", behaviortree_future)
            .detach();

        executor.tick(DELTA);
        controller.reset_tx.try_send(()).unwrap();

        executor.tick(DELTA);
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_async_behaviortree_shutdown() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);

        let executor = TickedAsyncExecutor::default();
        let shared = TestShared;

        let (behaviortree_future, controller) = AsyncBehaviorTree::new(
            behavior,
            AsyncBehaviorTreePolicy::ReloadOnCompletion,
            executor.tick_channel(),
            shared,
        );

        executor
            .spawn_local("AsyncBehaviorTreeFuture", behaviortree_future)
            .detach();

        for _ in 0..10 {
            executor.tick(DELTA);
        }

        controller.shutdown_tx.try_send(()).unwrap();
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }
}
