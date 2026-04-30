use crate::{
    AsyncActionContext, BehaviorTreeAsyncAction, BehaviorTreeReset,
    async_behavior_state::AsyncBehaviorState,
};

pub struct AsyncInvert<A> {
    child: Box<AsyncBehaviorState<A>>,
}

impl<A> AsyncInvert<A> {
    pub fn new(child: AsyncBehaviorState<A>) -> Self {
        Self {
            child: Box::new(child),
        }
    }
}

impl<A, R> BehaviorTreeReset<R> for AsyncInvert<A>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.child.reset(ctx);
    }
}

impl<A> std::future::Future for AsyncInvert<A> {
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
        behavior_nodes::AsyncAction,
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    #[test]
    fn test_invert_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_invert_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let action = AsyncAction::new(action, ctx.create_ctx());
            let action = AsyncInvert::new(AsyncBehaviorState::Action(action));
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_invert_with_dhat_post");
                let status = action.await;
                assert!(!status);
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
        assert_eq!(inner.get(), 3);
    }
}
