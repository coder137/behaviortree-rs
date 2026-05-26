use crate::{AsyncActionContext, BehaviorTreeReset};

pub struct AsyncTimes<C, R> {
    child: Box<C>,
    current_times: u64,
    reset: bool,

    times: u64,
    ctx: AsyncActionContext<R>,
}

impl<C, R> AsyncTimes<C, R> {
    pub fn new(child: C, times: u64, ctx: AsyncActionContext<R>) -> Self {
        Self {
            child: Box::new(child),
            current_times: 0,
            reset: false,
            times,
            ctx,
        }
    }
}

impl<C, R> BehaviorTreeReset<R> for AsyncTimes<C, R>
where
    C: BehaviorTreeReset<R>,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.current_times = 0;
        self.reset = false;
        self.child.reset(ctx);
    }
}

impl<C, R> std::future::Future for AsyncTimes<C, R>
where
    C: std::future::Future<Output = bool> + BehaviorTreeReset<R> + Unpin,
{
    type Output = C::Output;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();

        if bt.current_times == bt.times {
            return std::task::Poll::Ready(true);
        }

        if bt.reset {
            bt.reset = false;
            bt.child.reset(bt.ctx);
        }

        let child = std::pin::Pin::new(&mut bt.child);
        match child.poll(cx) {
            std::task::Poll::Ready(_s) => {
                bt.current_times += 1;
                if bt.current_times == bt.times {
                    std::task::Poll::Ready(true)
                } else {
                    bt.reset = true;
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                }
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use ticked_async_executor::TickedAsyncExecutor;

    use crate::{
        AsyncActionContextOwned,
        async_behavior_state::AsyncBehaviorState,
        behavior_nodes::{AsyncAction, AsyncTimes},
        test_nodes::{TestOperation, TestOperationRunner},
    };

    #[test]
    fn test_times_0() {
        let runner = TestOperationRunner::default();
        let ctx = AsyncActionContextOwned::new(runner, 10.0);

        let action = AsyncAction::new(TestOperation::Yield(true), ctx.create_ctx());
        let action = AsyncBehaviorState::Action(action);
        let future = AsyncTimes::new(action, 0, ctx.create_ctx());

        let mut executor = TickedAsyncExecutor::default();
        executor
            .spawn_local((), async move {
                let status = future.await;
                assert!(status);
            })
            .detach();

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_times_1() {
        let runner = TestOperationRunner::default();
        let ctx = AsyncActionContextOwned::new(runner, 10.0);

        let action = AsyncAction::new(TestOperation::Yield(true), ctx.create_ctx());
        let action = AsyncBehaviorState::Action(action);
        let future = AsyncTimes::new(action, 1, ctx.create_ctx());

        let mut executor = TickedAsyncExecutor::default();
        executor
            .spawn_local((), async move {
                let status = future.await;
                assert!(status);
            })
            .detach();

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 1);
        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_times_2() {
        let runner = TestOperationRunner::default();
        let ctx = AsyncActionContextOwned::new(runner, 10.0);

        let action = AsyncAction::new(TestOperation::Yield(true), ctx.create_ctx());
        let action = AsyncBehaviorState::Action(action);
        let future = AsyncTimes::new(action, 2, ctx.create_ctx());

        let mut executor = TickedAsyncExecutor::default();
        executor
            .spawn_local((), async move {
                let status = future.await;
                assert!(status);
            })
            .detach();

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 1);
        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 1);
        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_times_reset() {
        let runner = TestOperationRunner::default();
        let ctx = AsyncActionContextOwned::new(runner, 10.0);

        let action = AsyncBehaviorState::Action(AsyncAction::new(
            TestOperation::Yield(true),
            ctx.create_ctx(),
        ));
        let action = AsyncBehaviorState::Times(AsyncTimes::new(action, 1, ctx.create_ctx()));

        let future = AsyncBehaviorState::Times(AsyncTimes::new(action, 2, ctx.create_ctx()));

        let mut executor = TickedAsyncExecutor::default();
        executor
            .spawn_local((), async move {
                let status = future.await;
                assert!(status);
            })
            .detach();

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 1);
        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 1);
        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 0);
    }
}
