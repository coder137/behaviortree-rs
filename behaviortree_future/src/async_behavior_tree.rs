use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::ActionName;
use crate::BehaviorActionState;

use crate::BehaviorObserver;
use crate::BehaviorObserverTree;
use crate::IntoBehaviorActionState;
use crate::RootBehavior;
use crate::async_generic_behavior_tree::AsyncBehaviorTreeController;
use crate::async_generic_behavior_tree::AsyncGenericBehaviorTree;
use crate::observer_builder::AsyncBehaviorObserver;
use crate::observer_builder::NoObserverBuilder;
use crate::observer_builder::ObserverBuilder;

pub type AsyncBehaviorTree<AS, O> = AsyncGenericBehaviorTree<AS, O, Arc<AtomicBool>>;

impl<AS> AsyncBehaviorTree<AS, ()>
where
    AS: BehaviorActionState,
{
    pub fn from_behavior<A, R>(
        root_behavior: RootBehavior<A>,
        runner: &mut R,
    ) -> (Self, AsyncBehaviorTreeController<Arc<AtomicBool>>)
    where
        A: IntoBehaviorActionState<R, State = AS>,
        AS: BehaviorActionState,
    {
        let (bt, bt_controller, _bt_tree) = AsyncBehaviorTree::from_behavior_and_observer_builder(
            root_behavior,
            runner,
            &mut NoObserverBuilder,
        );
        (bt, bt_controller)
    }
}

impl<AS, O> AsyncBehaviorTree<AS, AsyncBehaviorObserver<Arc<O>>>
where
    AS: BehaviorActionState,
{
    pub fn from_behavior_with_observer<A, R>(
        root_behavior: RootBehavior<A>,
        runner: &mut R,
        observer: O,
    ) -> (
        Self,
        AsyncBehaviorTreeController<Arc<AtomicBool>>,
        BehaviorObserverTree,
    )
    where
        A: IntoBehaviorActionState<R, State = AS> + ActionName,
        AS: BehaviorActionState,
        O: BehaviorObserver,
    {
        let mut observer_builder = ObserverBuilder::new(Arc::new(observer));
        let (bt, bt_controller, bt_tree) = AsyncBehaviorTree::from_behavior_and_observer_builder(
            root_behavior,
            runner,
            &mut observer_builder,
        );
        observer_builder.transmit_init();
        (bt, bt_controller, bt_tree)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Behavior,
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    use super::*;

    #[test]
    fn test_async_behavior_tree() {
        static_assertions::assert_impl_all!(AsyncBehaviorTree<(), ()>: Send, Sync);
    }

    #[test]
    fn test_behaviortree_no_loop_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();

        let bt = {
            let _profiler = DhatTester::new("test_behaviortree_no_loop_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, _bt_controller) = AsyncBehaviorTree::from_behavior(
                RootBehavior::Once(Behavior::Action(action)),
                &mut runner,
            );
            bt
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_behaviortree_no_loop_with_dhat_post");
                let status = bt.await;
                assert!(status);
            })
            .detach();

        executor.wait_till_completed(16.67);
    }

    #[test]
    fn test_behaviortree_loop_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let inner = runner.num.clone();

        let action = {
            let _profiler = DhatTester::new("test_behaviortree_loop_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, _bt_controller) = AsyncBehaviorTree::from_behavior(
                RootBehavior::Loop(Behavior::Action(action).into()),
                &mut runner,
            );
            bt
        };

        let cancel = tokio_util::sync::CancellationToken::new();
        let cancel_clone = cancel.clone();
        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_behaviortree_loop_with_dhat_post");
                let ret = cancel_clone.run_until_cancelled_owned(action).await;
                assert!(ret.is_none());
            })
            .detach();

        executor.tick(10.0, None);

        executor.tick(20.0, None);
        assert_eq!(inner.get(), 3);

        // Reset takes place
        executor.tick(30.0, None);

        executor.tick(40.0, None);
        assert_eq!(inner.get(), 6);

        //Reset takes place
        executor.tick(50.0, None);

        executor.tick(60.0, None);
        assert_eq!(inner.get(), 9);

        // shutdown gracefully
        cancel.cancel();
        executor.tick(70.0, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_behaviortree_loop_with_early_shutdown_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let inner = runner.num.clone();

        let (bt, _bt_controller) = {
            let _profiler =
                DhatTester::new("test_behaviortree_loop_with_early_shutdown_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, bt_controller) = AsyncBehaviorTree::from_behavior(
                RootBehavior::Loop(Behavior::Action(action).into()),
                &mut runner,
            );
            (bt, bt_controller)
        };

        let cancel = tokio_util::sync::CancellationToken::new();
        let cancel_clone = cancel.clone();
        executor
            .spawn_local("_", async move {
                let _profiler =
                    DhatTester::new("test_behaviortree_loop_with_early_shutdown_with_dhat_post");
                let status = cancel_clone.run_until_cancelled_owned(bt).await;
                assert!(status.is_none());
            })
            .detach();

        executor.tick(10.0, None);

        executor.tick(20.0, None);
        assert_eq!(inner.get(), 3);

        // Reset takes place
        executor.tick(30.0, None);

        executor.tick(40.0, None);
        assert_eq!(inner.get(), 6);

        //Reset takes place
        executor.tick(50.0, None);

        executor.tick(60.0, None);
        assert_eq!(inner.get(), 9);

        // shutdown gracefully
        cancel.cancel();
        executor.tick(70.0, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_behaviortree_no_loop_with_early_reset_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let inner = runner.num.clone();

        let (bt, bt_controller) = {
            let _profiler =
                DhatTester::new("test_behaviortree_no_loop_with_early_reset_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, bt_controller) = AsyncBehaviorTree::from_behavior(
                RootBehavior::Once(Behavior::Action(action)),
                &mut runner,
            );
            (bt, bt_controller)
        };

        executor
            .spawn_local("_", async move {
                let _profiler =
                    DhatTester::new("test_behaviortree_no_loop_with_early_reset_with_dhat_post");
                let status = bt.await;
                assert!(status);
            })
            .detach();

        executor.tick(10.0, None);

        bt_controller.reset();

        executor.tick(20.0, None);
        assert_eq!(inner.get(), 0);

        bt_controller.reset();

        executor.tick(30.0, None);
        assert_eq!(inner.get(), 0);

        executor.tick(40.0, None);
        assert_eq!(inner.get(), 3);

        assert_eq!(executor.num_tasks(), 0);
    }
}
