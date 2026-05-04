use crate::{
    AsyncActionContext, BehaviorTreeAsyncAction, BehaviorTreeReset,
    async_behavior_state::AsyncBehaviorState,
};

pub struct AsyncLoop<A, R> {
    child: Box<AsyncBehaviorState<A, R>>,
    completed: bool,
    ctx: AsyncActionContext<R>,
}

impl<A, R> AsyncLoop<A, R> {
    pub fn new(child: AsyncBehaviorState<A, R>, ctx: AsyncActionContext<R>) -> Self {
        Self {
            child: Box::new(child),
            completed: false,
            ctx,
        }
    }
}

impl<A, R> BehaviorTreeReset<R> for AsyncLoop<A, R>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.completed = false;
        self.child.reset(ctx);
    }
}

impl<A, R> std::future::Future for AsyncLoop<A, R>
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
        if bt.completed {
            bt.completed = false;
            bt.child.reset(bt.ctx);
        }

        let child = std::pin::Pin::new(&mut bt.child);
        match child.poll(cx) {
            std::task::Poll::Ready(_s) => {
                bt.completed = true;
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio_util::sync::CancellationToken;

    use crate::{
        AsyncActionContextOwned, BehaviorTreeReset,
        async_behavior_state::AsyncBehaviorState,
        behavior_nodes::{AsyncAction, AsyncLoop},
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    #[test]
    fn test_async_loop_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_async_loop_with_dhat_pre");
            let action = AsyncAction::new(TestOperation::Add(1, 2, true, 1), ctx.create_ctx());
            let action = AsyncBehaviorState::Action(action);
            let action = AsyncLoop::new(action, ctx.create_ctx());
            action
        };

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_async_loop_with_dhat_post");
                let status = cancel_clone.run_until_cancelled_owned(action).await;
                println!("Status: {status:?}");
                assert!(status.is_none());
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 3);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 6);

        cancel.cancel();
        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_async_loop_reset_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let reset = std::rc::Rc::new(std::cell::Cell::new(false));

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let mut action = {
            let _profiler = DhatTester::new("test_async_loop_reset_with_dhat_pre");
            let action = AsyncAction::new(TestOperation::Add(1, 2, true, 1), ctx.create_ctx());
            let action = AsyncBehaviorState::Action(action);
            let action = AsyncLoop::new(action, ctx.create_ctx());
            let action = AsyncBehaviorState::Loop(action);
            action
        };

        let reset_clone = reset.clone();
        let future = std::future::poll_fn(move |cx| {
            let mut action = std::pin::Pin::new(&mut action);
            if reset_clone.get() {
                reset_clone.set(false);
                action.reset(ctx.create_ctx());
            }
            action.poll(cx)
        });

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_async_loop_reset_with_dhat_post");
                let status = cancel_clone.run_until_cancelled_owned(future).await;
                println!("Status: {status:?}");
                assert!(status.is_none());
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 3);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 6);

        executor.tick(16.67, None);
        assert_eq!(inner.get(), 6);
        reset.set(true);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 9);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 12);

        cancel.cancel();
        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
    }
}
