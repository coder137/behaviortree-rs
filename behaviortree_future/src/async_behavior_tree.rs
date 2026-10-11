use std::cell::Cell;
use std::rc::Rc;

use crate::ActionName;
use crate::Behavior;
use crate::BehaviorActionState;
use crate::BehaviorObserver;
use crate::BehaviorObserverRunner;
use crate::BehaviorObserverTree;
use crate::BehaviorTreeReset;
use crate::IntoBehaviorActionState;
use crate::async_behavior::AsyncBehavior;
use crate::observer_builder::AsyncBehaviorObserver;

#[derive(Clone, Copy)]
enum Control {
    None,
    Reset,
    Shutdown,
}

#[derive(Clone)]
pub struct AsyncBehaviorTreeController {
    control: Rc<Cell<Control>>,
}

impl AsyncBehaviorTreeController {
    pub fn reset(&self) {
        self.control.replace(Control::Reset);
    }

    pub fn shutdown(&self) {
        self.control.replace(Control::Shutdown);
    }
}

pub struct AsyncBehaviorTree<AS, O> {
    state: AsyncBehavior<AS, O>,
    control: Rc<Cell<Control>>,
}

impl<AS> AsyncBehaviorTree<AS, ()> {
    pub fn from_behavior<A, R>(
        behavior: Behavior<A>,
        runner: &mut R,
    ) -> (Self, AsyncBehaviorTreeController)
    where
        A: IntoBehaviorActionState<AS, R>,
        AS: BehaviorActionState,
    {
        let state = AsyncBehavior::new(behavior, runner);
        let control = Rc::new(Cell::new(Control::None));
        let behaviortree = Self {
            state,
            control: control.clone(),
        };
        let behaviortree_controller = AsyncBehaviorTreeController { control };
        (behaviortree, behaviortree_controller)
    }
}

impl<AS, O> AsyncBehaviorTree<AS, AsyncBehaviorObserver<O>>
where
    O: BehaviorObserver,
{
    pub fn from_behavior_with_observer<A, R>(
        behavior: Behavior<A>,
        runner: &mut R,
        observer: O,
    ) -> (Self, AsyncBehaviorTreeController, BehaviorObserverTree)
    where
        A: IntoBehaviorActionState<AS, R> + ActionName,
        AS: BehaviorActionState,
    {
        let (state, tree) = AsyncBehavior::new_with_observer(behavior, runner, observer);
        let control = Rc::new(Cell::new(Control::None));
        let behaviortree = Self {
            state,
            control: control.clone(),
        };
        let behaviortree_controller = AsyncBehaviorTreeController { control };
        (behaviortree, behaviortree_controller, tree)
    }
}

impl<AS, O> std::future::Future for AsyncBehaviorTree<AS, O>
where
    AS: BehaviorActionState,
    O: BehaviorObserverRunner + Unpin,
{
    type Output = Option<bool>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();
        match bt.control.get() {
            Control::None => {}
            Control::Reset => {
                bt.control.replace(Control::None);
                bt.state.reset();
            }
            Control::Shutdown => {
                return std::task::Poll::Ready(None);
            }
        }
        let state = std::pin::Pin::new(&mut bt.state);
        state.poll(cx).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_nodes::{DhatTester, TestOperation, TestOperationRunner};

    use super::*;

    #[test]
    fn test_behaviortree_no_loop_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();

        let bt = {
            let _profiler = DhatTester::new("test_behaviortree_no_loop_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, _bt_controller) =
                AsyncBehaviorTree::from_behavior(Behavior::Action(action), &mut runner);
            bt
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_behaviortree_no_loop_with_dhat_post");
                let status = bt.await;
                assert!(status.unwrap());
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
                Behavior::Loop(Behavior::Action(action).into()),
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

        let (bt, bt_controller) = {
            let _profiler =
                DhatTester::new("test_behaviortree_loop_with_early_shutdown_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, bt_controller) = AsyncBehaviorTree::from_behavior(
                Behavior::Loop(Behavior::Action(action).into()),
                &mut runner,
            );
            (bt, bt_controller)
        };

        executor
            .spawn_local("_", async move {
                let _profiler =
                    DhatTester::new("test_behaviortree_loop_with_early_shutdown_with_dhat_post");
                let status = bt.await;
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
        bt_controller.shutdown();
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
            let (bt, bt_controller) =
                AsyncBehaviorTree::from_behavior(Behavior::Action(action).into(), &mut runner);
            (bt, bt_controller)
        };

        executor
            .spawn_local("_", async move {
                let _profiler =
                    DhatTester::new("test_behaviortree_no_loop_with_early_reset_with_dhat_post");
                let status = bt.await;
                let status = status.unwrap();
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
