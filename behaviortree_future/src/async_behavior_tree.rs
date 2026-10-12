use std::cell::Cell;
use std::rc::Rc;

use crate::ActionName;
use crate::BehaviorActionState;
use crate::BehaviorObserver;
use crate::BehaviorObserverRunner;
use crate::BehaviorObserverTree;
use crate::BehaviorTreeReset;
use crate::IntoBehaviorActionState;
use crate::RootBehavior;
use crate::async_behavior::AsyncBehavior;
use crate::behavior_nodes::AsyncLoop;
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

enum RootAsyncBehavior<AS, O> {
    Once(AsyncBehavior<AS, O>),
    Loop(AsyncLoop<AsyncBehavior<AS, O>>),
}

impl<AS, O> std::future::Future for RootAsyncBehavior<AS, O>
where
    AS: BehaviorActionState,
    O: BehaviorObserverRunner + Unpin,
{
    type Output = bool;

    #[inline(always)]
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.as_mut().get_mut() {
            RootAsyncBehavior::Once(f) => std::pin::Pin::new(f).poll(cx),
            RootAsyncBehavior::Loop(f) => std::pin::Pin::new(f).poll(cx),
        }
    }
}

impl<AS, O> BehaviorTreeReset for RootAsyncBehavior<AS, O>
where
    O: BehaviorObserverRunner + Unpin,
    AS: BehaviorActionState,
{
    #[inline(always)]
    fn reset(&mut self) {
        match self {
            RootAsyncBehavior::Once(r) => r.reset(),
            RootAsyncBehavior::Loop(r) => r.reset(),
        }
    }
}

pub struct AsyncBehaviorTree<AS, O> {
    state: RootAsyncBehavior<AS, O>,
    control: Rc<Cell<Control>>,
}

impl<AS> AsyncBehaviorTree<AS, ()> {
    pub fn from_behavior<A, R>(
        root_behavior: RootBehavior<A>,
        runner: &mut R,
    ) -> (Self, AsyncBehaviorTreeController)
    where
        A: IntoBehaviorActionState<R, State = AS>,
        AS: BehaviorActionState,
    {
        let state = match root_behavior {
            RootBehavior::Once(behavior) => {
                RootAsyncBehavior::Once(AsyncBehavior::new(behavior, runner))
            }
            RootBehavior::Loop(behavior) => {
                RootAsyncBehavior::Loop(AsyncLoop::new(AsyncBehavior::new(behavior, runner)))
            }
        };
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
        root_behavior: RootBehavior<A>,
        runner: &mut R,
        observer: O,
    ) -> (Self, AsyncBehaviorTreeController, BehaviorObserverTree)
    where
        A: IntoBehaviorActionState<R, State = AS> + ActionName,
        AS: BehaviorActionState,
    {
        let (state, tree) = match root_behavior {
            RootBehavior::Once(behavior) => {
                let (state, tree) = AsyncBehavior::new_with_observer(behavior, runner, observer);
                (RootAsyncBehavior::Once(state), tree)
            }
            RootBehavior::Loop(behavior) => {
                let (state, tree) = AsyncBehavior::new_with_observer(behavior, runner, observer);
                (RootAsyncBehavior::Loop(AsyncLoop::new(state)), tree)
            }
        };
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
    use crate::{
        Behavior,
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    use super::*;

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

        let (bt, bt_controller) = {
            let _profiler =
                DhatTester::new("test_behaviortree_loop_with_early_shutdown_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, bt_controller) = AsyncBehaviorTree::from_behavior(
                RootBehavior::Loop(Behavior::Action(action).into()),
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
