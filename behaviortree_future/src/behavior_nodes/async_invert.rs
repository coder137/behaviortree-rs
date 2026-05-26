use crate::{AsyncActionContext, BehaviorTreeReset};

pub struct AsyncInvert<C> {
    child: Box<C>,
}

impl<C> AsyncInvert<C> {
    pub fn new(child: C) -> Self {
        Self {
            child: child.into(),
        }
    }
}

impl<C, R> BehaviorTreeReset<R> for AsyncInvert<C>
where
    C: BehaviorTreeReset<R>,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.child.reset(ctx);
    }
}

impl<C> std::future::Future for AsyncInvert<C>
where
    C: std::future::Future<Output = bool> + Unpin,
{
    type Output = C::Output;
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
        async_behavior_state::AsyncBehaviorState,
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
            let action = AsyncBehaviorState::<_, _, ()>::Action(action, None);
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
            let action = AsyncBehaviorState::<_, _, ()>::Action(action, None);
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
            let action = AsyncBehaviorState::<_, _, ()>::Action(action, None);
            // invert
            let action = AsyncInvert::new(action.into());
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
