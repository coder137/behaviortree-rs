use std::cell::Cell;
use std::rc::Rc;

use crate::AsyncActionContextOwned;
use crate::Behavior;
use crate::BehaviorTreeAsyncAction;
use crate::BehaviorTreeObserver;
use crate::BehaviorTreeReset;
use crate::async_behavior_state::AsyncBehaviorState;
use crate::async_behavior_state::AsyncBehaviorStateTree;

#[derive(Clone, Copy)]
enum Control {
    None,
    Reset,
    Shutdown,
}

pub struct AsyncBehaviorTreeController {
    control: Rc<Cell<Control>>,
}

impl AsyncBehaviorTreeController {
    pub fn reset(&self) {
        self.control.replace(Control::Reset);
    }

    pub fn shutdown(self) {
        self.control.replace(Control::Shutdown);
    }
}

pub struct AsyncBehaviorTree<A, R, O> {
    state: AsyncBehaviorState<A, R, O>,
    ctx: AsyncActionContextOwned<R>,
    delta: Rc<Cell<f64>>,

    // control
    control: Rc<Cell<Control>>,
}

impl<A, R, O> AsyncBehaviorTree<A, R, O> {
    pub fn from_behavior_with_observer(
        behavior: Behavior<A>,
        runner: R,
        delta: std::rc::Rc<std::cell::Cell<f64>>,
        observer: Rc<O>,
    ) -> (Self, AsyncBehaviorTreeController, AsyncBehaviorStateTree)
    where
        A: BehaviorTreeAsyncAction<R>,
        O: BehaviorTreeObserver<A>,
    {
        let ctx = AsyncActionContextOwned::new(runner, delta.get());
        let mut id = 0;
        let (state, state_observer) = AsyncBehaviorState::from_behavior_with_observer(
            behavior,
            ctx.create_ctx(),
            observer,
            &mut id,
        );
        let control = Rc::new(Cell::new(Control::None));
        let behaviortree = Self {
            state,
            ctx,
            delta,
            control: control.clone(),
        };
        let behaviortree_controller = AsyncBehaviorTreeController { control };
        (behaviortree, behaviortree_controller, state_observer)
    }
}

impl<A, R> AsyncBehaviorTree<A, R, ()> {
    pub fn from_behavior(
        behavior: Behavior<A>,
        runner: R,
        delta: std::rc::Rc<std::cell::Cell<f64>>,
    ) -> (Self, AsyncBehaviorTreeController)
    where
        A: BehaviorTreeAsyncAction<R>,
    {
        let ctx = AsyncActionContextOwned::new(runner, delta.get());
        let state = AsyncBehaviorState::from_behavior(behavior, ctx.create_ctx());
        let control = Rc::new(Cell::new(Control::None));
        let behaviortree = Self {
            state,
            ctx,
            delta,
            control: control.clone(),
        };
        let behaviortree_controller = AsyncBehaviorTreeController { control };
        (behaviortree, behaviortree_controller)
    }
}

impl<A, R, O> std::future::Future for AsyncBehaviorTree<A, R, O>
where
    A: BehaviorTreeAsyncAction<R>,
    O: BehaviorTreeObserver<A>,
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
                bt.state.reset(bt.ctx.create_ctx());
            }
            Control::Shutdown => {
                return std::task::Poll::Ready(None);
            }
        }
        let current_delta = bt.delta.get();
        bt.ctx.update_delta(current_delta);
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

        let runner = TestOperationRunner::default();

        let bt = {
            let _profiler = DhatTester::new("test_behaviortree_no_loop_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, _bt_controller) = AsyncBehaviorTree::from_behavior(
                Behavior::Action(action),
                runner,
                executor.delta().inner().into(),
            );
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

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let inner_delta = runner.delta.clone();

        let action = {
            let _profiler = DhatTester::new("test_behaviortree_loop_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, _bt_controller) = AsyncBehaviorTree::from_behavior(
                Behavior::Loop(Behavior::Action(action).into()),
                runner,
                executor.delta().inner().into(),
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
        assert_eq!(inner_delta.get(), 10.0);

        executor.tick(20.0, None);
        assert_eq!(inner.get(), 3);
        assert_eq!(inner_delta.get(), 20.0);

        // Reset takes place
        executor.tick(30.0, None);
        assert_eq!(inner_delta.get(), 30.0);

        executor.tick(40.0, None);
        assert_eq!(inner.get(), 6);
        assert_eq!(inner_delta.get(), 40.0);

        //Reset takes place
        executor.tick(50.0, None);
        assert_eq!(inner_delta.get(), 50.0);

        executor.tick(60.0, None);
        assert_eq!(inner.get(), 9);
        assert_eq!(inner_delta.get(), 60.0);

        // shutdown gracefully
        cancel.cancel();
        executor.tick(70.0, None);
        assert_eq!(inner_delta.get(), 70.0);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_behaviortree_loop_with_early_shutdown_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let inner_delta = runner.delta.clone();

        let (bt, bt_controller) = {
            let _profiler =
                DhatTester::new("test_behaviortree_loop_with_early_shutdown_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, bt_controller) = AsyncBehaviorTree::from_behavior(
                Behavior::Loop(Behavior::Action(action).into()),
                runner,
                executor.delta().inner().into(),
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
        assert_eq!(inner_delta.get(), 10.0);

        executor.tick(20.0, None);
        assert_eq!(inner.get(), 3);
        assert_eq!(inner_delta.get(), 20.0);

        // Reset takes place
        executor.tick(30.0, None);
        assert_eq!(inner_delta.get(), 30.0);

        executor.tick(40.0, None);
        assert_eq!(inner.get(), 6);
        assert_eq!(inner_delta.get(), 40.0);

        //Reset takes place
        executor.tick(50.0, None);
        assert_eq!(inner_delta.get(), 50.0);

        executor.tick(60.0, None);
        assert_eq!(inner.get(), 9);
        assert_eq!(inner_delta.get(), 60.0);

        // shutdown gracefully
        bt_controller.shutdown();
        executor.tick(70.0, None);
        assert_eq!(inner_delta.get(), 60.0);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_behaviortree_no_loop_with_early_reset_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let inner_delta = runner.delta.clone();

        let (bt, bt_controller) = {
            let _profiler =
                DhatTester::new("test_behaviortree_no_loop_with_early_reset_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, bt_controller) = AsyncBehaviorTree::from_behavior(
                Behavior::Action(action).into(),
                runner,
                executor.delta().inner().into(),
            );
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
        assert_eq!(inner_delta.get(), 10.0);

        bt_controller.reset();

        executor.tick(20.0, None);
        assert_eq!(inner.get(), 0);
        assert_eq!(inner_delta.get(), 20.0);

        bt_controller.reset();

        executor.tick(30.0, None);
        assert_eq!(inner.get(), 0);
        assert_eq!(inner_delta.get(), 30.0);

        executor.tick(40.0, None);
        assert_eq!(inner.get(), 3);
        assert_eq!(inner_delta.get(), 40.0);

        assert_eq!(executor.num_tasks(), 0);
    }
}
