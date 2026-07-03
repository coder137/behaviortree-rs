use std::cell::Cell;
use std::rc::Rc;

use crate::ActionToActionState;
use crate::AsyncActionContextOwned;
use crate::AsyncBehaviorActionState;
use crate::AsyncBehaviorStateTree;
use crate::Behavior;
use crate::BehaviorTreeObserver;
use crate::BehaviorTreeReset;
use crate::Status;
use crate::async_behavior_state::AsyncBehaviorState;
use crate::async_behavior_state_with_observer::AsyncBehaviorStateWithObserver;

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

#[pin_project::pin_project(project = AsyncBehaviorTreeStateProj)]
enum AsyncBehaviorTreeState<AS, R, O> {
    Default(#[pin] AsyncBehaviorState<AS, R>),
    Observer(#[pin] AsyncBehaviorStateWithObserver<AS, R, O>),
}

impl<AS, R, O> BehaviorTreeReset<R> for AsyncBehaviorTreeState<AS, R, O>
where
    AS: AsyncBehaviorActionState<R>,
    O: BehaviorTreeObserver<AS>,
{
    fn reset(&mut self, ctx: crate::AsyncActionContext<R>) {
        let r: &mut dyn BehaviorTreeReset<R> = match self {
            AsyncBehaviorTreeState::Default(a) => a,
            AsyncBehaviorTreeState::Observer(a) => a,
        };
        r.reset(ctx);
    }
}

impl<AS, R, O> std::future::Future for AsyncBehaviorTreeState<AS, R, O>
where
    AS: AsyncBehaviorActionState<R>,
    O: BehaviorTreeObserver<AS>,
{
    type Output = bool;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let future: std::pin::Pin<&mut dyn std::future::Future<Output = bool>> = match this {
            AsyncBehaviorTreeStateProj::Default(a) => a,
            AsyncBehaviorTreeStateProj::Observer(a) => a,
        };
        future.poll(cx)
    }
}

pub struct AsyncBehaviorTree<AS, R, O> {
    state: AsyncBehaviorTreeState<AS, R, O>,
    ctx: AsyncActionContextOwned<R>,
    delta: Rc<Cell<f64>>,

    // control
    control: Rc<Cell<Control>>,
}

impl<AS, R, O> AsyncBehaviorTree<AS, R, O> {
    pub fn from_behavior_with_observer<A>(
        behavior: Behavior<A>,
        runner: R,
        delta: std::rc::Rc<std::cell::Cell<f64>>,
        observer: Rc<O>,
    ) -> (Self, AsyncBehaviorTreeController, AsyncBehaviorStateTree)
    where
        A: ActionToActionState<AS>,
        AS: AsyncBehaviorActionState<R>,
        O: BehaviorTreeObserver<AS>,
    {
        let ctx = AsyncActionContextOwned::new(runner, delta.get());
        let mut id = 0;
        let (state, state_tree) = AsyncBehaviorStateWithObserver::from_behavior(
            behavior,
            ctx.create_ctx(),
            observer.clone(),
            &mut id,
        );
        observer.init(id);
        let control = Rc::new(Cell::new(Control::None));
        let behaviortree = Self {
            state: AsyncBehaviorTreeState::Observer(state),
            ctx,
            delta,
            control: control.clone(),
        };
        let behaviortree_controller = AsyncBehaviorTreeController { control };
        (behaviortree, behaviortree_controller, state_tree)
    }
}

impl<AS, R> AsyncBehaviorTree<AS, R, ()> {
    pub fn from_behavior<A>(
        behavior: Behavior<A>,
        runner: R,
        delta: std::rc::Rc<std::cell::Cell<f64>>,
    ) -> (Self, AsyncBehaviorTreeController)
    where
        A: ActionToActionState<AS>,
        AS: AsyncBehaviorActionState<R>,
    {
        let ctx = AsyncActionContextOwned::new(runner, delta.get());
        let state = AsyncBehaviorState::from_behavior(behavior, ctx.create_ctx());
        let control = Rc::new(Cell::new(Control::None));
        let behaviortree = Self {
            state: AsyncBehaviorTreeState::Default(state),
            ctx,
            delta,
            control: control.clone(),
        };
        let behaviortree_controller = AsyncBehaviorTreeController { control };
        (behaviortree, behaviortree_controller)
    }
}

impl<AS, R, O> std::future::Future for AsyncBehaviorTree<AS, R, O>
where
    AS: AsyncBehaviorActionState<R>,
    O: BehaviorTreeObserver<AS>,
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

impl<AS> BehaviorTreeObserver<AS> for () {
    fn action_name(_action: &AS) -> &'static str {
        ""
    }
    fn init(&self, _capacity: usize) {}
    fn update(&self, _id: usize, _status: Option<Status>) {}
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
