use crate::AsyncActionContextOwned;
use crate::Behavior;
use crate::BehaviorTreeAsyncAction;
use crate::async_behavior_state::AsyncBehaviorState;
use crate::behavior_nodes::AsyncLoop;

pub struct AsyncBehaviorTree<A, R> {
    child: AsyncBehaviorState<A, R>,
    _ctx: AsyncActionContextOwned<R>,
}

impl<A, R> AsyncBehaviorTree<A, R> {
    pub fn from_behavior(
        behavior: Behavior<A>,
        runner: R,
        delta: std::rc::Rc<std::cell::Cell<f64>>,
        should_loop: bool,
    ) -> Self
    where
        A: BehaviorTreeAsyncAction<R> + Clone + 'static,
        R: 'static,
    {
        let ctx = AsyncActionContextOwned::new(runner, delta.get());
        let child = AsyncBehaviorState::from_behavior(behavior, ctx.create_ctx());
        let child = if should_loop {
            let child = AsyncLoop::new(child, ctx.create_ctx());
            AsyncBehaviorState::Loop(child)
        } else {
            child
        };
        Self { child, _ctx: ctx }
    }
}

impl<A, R> std::future::Future for AsyncBehaviorTree<A, R>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    type Output = bool;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();
        let child = std::pin::Pin::new(&mut bt.child);
        child.poll(cx)
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
            let bt = AsyncBehaviorTree::from_behavior(
                Behavior::Action(action),
                runner,
                executor.delta().inner().into(),
                false,
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

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();

        let action = {
            let _profiler = DhatTester::new("test_behaviortree_loop_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let action = AsyncBehaviorTree::from_behavior(
                Behavior::Action(action),
                runner,
                executor.delta().inner().into(),
                true,
            );
            action
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

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        println!("{:?}", inner);
        assert_eq!(inner.get(), 3);

        // Reset takes place
        executor.tick(16.67, None);
        executor.tick(16.67, None);
        println!("{:?}", inner);
        assert_eq!(inner.get(), 6);

        //Reset takes place
        executor.tick(16.67, None);
        executor.tick(16.67, None);
        println!("{:?}", inner);
        assert_eq!(inner.get(), 9);

        // shutdown gracefully
        cancel.cancel();
        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
    }
}
