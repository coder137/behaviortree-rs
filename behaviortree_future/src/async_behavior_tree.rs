use std::cell::Cell;
use std::rc::Rc;

use crate::ActionToActionState;
use crate::AsyncBehaviorActionState;
use crate::AsyncBehaviorStateTree;
use crate::Behavior;
use crate::BehaviorTreeObserver;
use crate::BehaviorTreeReset;
use crate::Delta;
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
enum AsyncBehaviorTreeState<AS, O> {
    Default(#[pin] AsyncBehaviorState<AS>),
    Observer(#[pin] AsyncBehaviorStateWithObserver<AS, O>),
}

impl<AS, O> BehaviorTreeReset for AsyncBehaviorTreeState<AS, O>
where
    AS: AsyncBehaviorActionState,
    O: BehaviorTreeObserver<AS>,
{
    fn reset(&mut self) {
        let r: &mut dyn BehaviorTreeReset = match self {
            AsyncBehaviorTreeState::Default(a) => a,
            AsyncBehaviorTreeState::Observer(a) => a,
        };
        r.reset();
    }
}

impl<AS, O> std::future::Future for AsyncBehaviorTreeState<AS, O>
where
    AS: AsyncBehaviorActionState,
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

pub struct AsyncBehaviorTree<AS, O> {
    state: AsyncBehaviorTreeState<AS, O>,
    delta: Rc<Cell<f64>>,
    current_delta: Rc<Delta>,

    // control
    control: Rc<Cell<Control>>,
}

impl<AS, O> AsyncBehaviorTree<AS, O> {
    pub fn from_behavior_with_observer<A, R>(
        behavior: Behavior<A>,
        runner: &mut R,
        delta: std::rc::Rc<std::cell::Cell<f64>>,
        observer: Rc<O>,
    ) -> (Self, AsyncBehaviorTreeController, AsyncBehaviorStateTree)
    where
        A: ActionToActionState<AS, R>,
        AS: AsyncBehaviorActionState,
        O: BehaviorTreeObserver<AS>,
    {
        let mut id = 0;
        let current_delta = Rc::new(Delta::default());
        let (state, state_tree) = AsyncBehaviorStateWithObserver::from_behavior(
            behavior,
            current_delta.clone(),
            runner,
            observer.clone(),
            &mut id,
        );
        observer.init(id);
        let control = Rc::new(Cell::new(Control::None));
        let behaviortree = Self {
            state: AsyncBehaviorTreeState::Observer(state),
            delta,
            current_delta,
            control: control.clone(),
        };
        let behaviortree_controller = AsyncBehaviorTreeController { control };
        (behaviortree, behaviortree_controller, state_tree)
    }
}

impl<AS> AsyncBehaviorTree<AS, ()> {
    pub fn from_behavior<A, R>(
        behavior: Behavior<A>,
        runner: &mut R,
        delta: std::rc::Rc<std::cell::Cell<f64>>,
    ) -> (Self, AsyncBehaviorTreeController)
    where
        A: ActionToActionState<AS, R>,
        AS: AsyncBehaviorActionState,
    {
        let current_delta = Rc::new(Delta::default());
        let state = AsyncBehaviorState::from_behavior(behavior, current_delta.clone(), runner);
        let control = Rc::new(Cell::new(Control::None));
        let behaviortree = Self {
            state: AsyncBehaviorTreeState::Default(state),
            delta,
            current_delta,
            control: control.clone(),
        };
        let behaviortree_controller = AsyncBehaviorTreeController { control };
        (behaviortree, behaviortree_controller)
    }
}

impl<AS, O> std::future::Future for AsyncBehaviorTree<AS, O>
where
    AS: AsyncBehaviorActionState,
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
                bt.state.reset();
            }
            Control::Shutdown => {
                return std::task::Poll::Ready(None);
            }
        }
        let current_delta = bt.delta.get();
        bt.current_delta.update(current_delta);
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

        let mut runner = TestOperationRunner::default();

        let bt = {
            let _profiler = DhatTester::new("test_behaviortree_no_loop_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, _bt_controller) = AsyncBehaviorTree::from_behavior(
                Behavior::Action(action),
                &mut runner,
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

        let mut runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let inner_delta = runner.delta.clone();

        let action = {
            let _profiler = DhatTester::new("test_behaviortree_loop_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, _bt_controller) = AsyncBehaviorTree::from_behavior(
                Behavior::Loop(Behavior::Action(action).into()),
                &mut runner,
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

        let mut runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let inner_delta = runner.delta.clone();

        let (bt, bt_controller) = {
            let _profiler =
                DhatTester::new("test_behaviortree_loop_with_early_shutdown_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, bt_controller) = AsyncBehaviorTree::from_behavior(
                Behavior::Loop(Behavior::Action(action).into()),
                &mut runner,
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

        let mut runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let inner_delta = runner.delta.clone();

        let (bt, bt_controller) = {
            let _profiler =
                DhatTester::new("test_behaviortree_no_loop_with_early_reset_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let (bt, bt_controller) = AsyncBehaviorTree::from_behavior(
                Behavior::Action(action).into(),
                &mut runner,
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
