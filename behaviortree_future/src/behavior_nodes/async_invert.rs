use crate::{
    AsyncActionContext, BehaviorTreeAsyncAction, BehaviorTreeReset,
    async_behavior_state::AsyncBehaviorState,
};

pub struct AsyncInvert<A, R> {
    child: Box<AsyncBehaviorState<A, R>>,
}

impl<A, R> AsyncInvert<A, R> {
    pub fn new(child: AsyncBehaviorState<A, R>) -> Self {
        Self {
            child: Box::new(child),
        }
    }
}

impl<A, R> BehaviorTreeReset<R> for AsyncInvert<A, R>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.child.reset(ctx);
    }
}

impl<A, R> std::future::Future for AsyncInvert<A, R>
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
        child.poll(cx).map(|s| !s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        AsyncActionContextOwned,
        behavior_nodes::{AsyncAction, AsyncTimes},
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    #[test]
    fn test_invert_success_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_invert_success_with_dhat_pre");
            let action = AsyncAction::new(TestOperation::Yield(true), ctx.create_ctx());
            let action = AsyncBehaviorState::Action(action);
            let action = AsyncInvert::new(action);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_invert_success_with_dhat_post");
                let status = action.await;
                assert!(!status);
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
    fn test_invert_failure_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_invert_failure_with_dhat_pre");
            let action = AsyncAction::new(TestOperation::Yield(false), ctx.create_ctx());
            let action = AsyncBehaviorState::Action(action);
            let action = AsyncInvert::new(action);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_invert_failure_with_dhat_post");
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
    fn test_invert_reset_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_invert_reset_with_dhat_pre");
            // action
            let action = AsyncAction::new(TestOperation::Yield(true), ctx.create_ctx());
            let action = AsyncBehaviorState::Action(action);
            // invert
            let action = AsyncInvert::new(action);
            let action = AsyncBehaviorState::Invert(action);
            // times
            let action = AsyncTimes::new(action, 2, ctx.create_ctx());
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_invert_reset_with_dhat_post");
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
