use reusable_box_future::ReusableLocalBoxFuture;

use crate::{
    AsyncActionContext, AsyncBehaviorActionState, BehaviorTreeAsyncHandler, BehaviorTreeReset,
};

struct CreateReusableLocalBoxFutureHandler;
impl BehaviorTreeAsyncHandler<'static> for CreateReusableLocalBoxFutureHandler {
    type Output = ReusableLocalBoxFuture<bool>;
    fn future(self, future: impl std::future::Future<Output = bool> + 'static) -> Self::Output {
        reusable_box_future::ReusableLocalBoxFuture::new(future)
    }
}

struct UpdateReusableLocalBoxFutureHandler<'a>(&'a mut ReusableLocalBoxFuture<bool>);
impl<'a> BehaviorTreeAsyncHandler<'static> for UpdateReusableLocalBoxFutureHandler<'a> {
    type Output = ();
    fn future(self, future: impl std::future::Future<Output = bool> + 'static) -> Self::Output {
        self.0.set(future);
    }
}

#[pin_project::pin_project]
pub struct AsyncAction<AS> {
    action_state: AS,
    // state
    #[pin]
    future: reusable_box_future::ReusableLocalBoxFuture<bool>,
}

impl<AS> AsyncAction<AS> {
    pub fn new<R>(action_state: AS, ctx: AsyncActionContext<R>) -> Self
    where
        AS: AsyncBehaviorActionState<R>,
    {
        let future = action_state.make_future(ctx, CreateReusableLocalBoxFutureHandler);
        Self {
            action_state,
            future,
        }
    }
}

impl<AS, R> BehaviorTreeReset<R> for AsyncAction<AS>
where
    AS: AsyncBehaviorActionState<R>,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.action_state.reset(ctx);
        self.action_state
            .make_future(ctx, UpdateReusableLocalBoxFutureHandler(&mut self.future));
    }
}

impl<AS> std::future::Future for AsyncAction<AS> {
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
