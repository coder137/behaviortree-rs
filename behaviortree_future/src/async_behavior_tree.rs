use crate::AsyncActionContextOwned;
use crate::Behavior;
use crate::BehaviorTreeAsyncAction;
use crate::BehaviorTreeReset;
use crate::async_behavior_state::AsyncBehaviorState;

pub struct AsyncBehaviorTree<A, R> {
    ctx: AsyncActionContextOwned<R>,
    should_loop: bool,

    // state
    child: AsyncBehaviorState<A>,
    result: Option<bool>,
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
        let ctx_ref = ctx.create_ctx();
        let child = AsyncBehaviorState::from_behavior(behavior, ctx_ref);
        Self::new(child, ctx, should_loop)
    }

    pub(crate) fn new(
        child: AsyncBehaviorState<A>,
        ctx: AsyncActionContextOwned<R>,
        should_loop: bool,
    ) -> Self {
        Self {
            child,
            ctx,
            should_loop,
            result: None,
        }
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
        if bt.result.is_some() && bt.should_loop {
            bt.result = None;
            bt.child.reset(bt.ctx.create_ctx());
        }

        let child_status = std::pin::pin!(&mut bt.child).poll(cx);
        match child_status {
            std::task::Poll::Ready(result) => {
                bt.result = Some(result);
                if bt.should_loop {
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                } else {
                    std::task::Poll::Ready(result)
                }
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
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
