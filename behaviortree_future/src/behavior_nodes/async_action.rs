use crate::{AsyncActionContext, BehaviorTreeAsyncAction, BehaviorTreeReset};

#[pin_project::pin_project]
pub struct AsyncAction<A> {
    action: A,
    // state
    #[pin]
    future: reusable_box_future::ReusableLocalBoxFuture<bool>,
}

impl<A> AsyncAction<A> {
    pub fn new<R>(action: A, ctx: AsyncActionContext<R>) -> Self
    where
        A: BehaviorTreeAsyncAction<R> + Clone + 'static,
        R: 'static,
    {
        let future = action.clone().create_future(ctx);
        let future = reusable_box_future::ReusableLocalBoxFuture::new(future);
        Self { action, future }
    }
}

impl<A, R> BehaviorTreeReset<R> for AsyncAction<A>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    fn reset(&mut self, mut ctx: AsyncActionContext<R>) {
        self.action.reset(&mut ctx);
        let future = self.action.clone().create_future(ctx);
        self.future.set(future);
    }
}

impl<A> std::future::Future for AsyncAction<A> {
    type Output = bool;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        this.future.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        AsyncActionContextOwned,
        async_behavior_state::AsyncBehaviorState,
        behavior_nodes::{AsyncAction, AsyncTimes},
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    #[test]
    fn test_action_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_action_with_dhat_pre");
            let action = TestOperation::Yield(true);
            let action = AsyncAction::new(action, ctx.create_ctx());
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_action_with_dhat_post");
                let status = action.await;
                assert!(status);
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_action_reset_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();
        let delta = executor.delta().inner();

        let runner = TestOperationRunner::default();
        let ctx = AsyncActionContextOwned::new(runner, delta.get());

        let action = {
            let _profiler = DhatTester::new("test_action_reset_with_dhat_pre");
            let action = AsyncAction::new(TestOperation::Yield(true), ctx.create_ctx());
            let action = AsyncBehaviorState::Action(action);
            let action = AsyncBehaviorState::Times(AsyncTimes::new(action, 2, ctx.create_ctx()));
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_action_reset_with_dhat_post");
                let status = action.await;
                assert!(status);
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);

        executor.tick(16.67, None);
        executor.tick(16.67, None);

        assert_eq!(executor.num_tasks(), 0);
    }
}
