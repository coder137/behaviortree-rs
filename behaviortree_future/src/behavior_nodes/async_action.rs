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
        A: BehaviorTreeAsyncAction<R>,
    {
        let future = action.create_future(ctx);
        Self { action, future }
    }
}

impl<A, R> BehaviorTreeReset<R> for AsyncAction<A>
where
    A: BehaviorTreeAsyncAction<R>,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.action.reset_future(ctx, &mut self.future);
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
            let action = AsyncBehaviorState::<_, _, ()>::Action(action, None);
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

    #[ignore = "random test"]
    #[tokio::test]
    async fn test_channel_memory_with_dhat() {
        // broadcast channel
        {
            // 168 bytes, 2 blocks
            let (tx, mut rx) = {
                let _profiler = DhatTester::new("test_channel_memory_broadcast");
                let (tx, rx) = tokio::sync::broadcast::channel::<u32>(1);
                (tx, rx)
            };

            // 0 bytes send
            {
                let _profiler = DhatTester::new("test_channel_memory_broadcast_tx");
                // tx.send(10).await.unwrap();
                tx.send(10).unwrap();
            };

            // 0 bytes recv
            {
                let _profiler = DhatTester::new("test_channel_memory_broadcast_rx");
                let d = rx.recv().await.unwrap();
                assert_eq!(d, 10);
            };
        }

        // mpsc channel
        {
            // 672 bytes, 2 blocks
            let (tx, mut rx) = {
                let _profiler = DhatTester::new("test_channel_memory_mpsc");
                // let (tx, rx) = tokio::sync::mpsc::channel::<u32>(1);
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<usize>();
                (tx, rx)
            };

            // 0 bytes send
            const LEN: usize = 33;
            {
                let _profiler = DhatTester::new("test_channel_memory_mpsc_tx");
                for i in 0..LEN {
                    // println!("{}", i * 10);
                    tx.send(i * 10).unwrap();
                }
            };

            // 0 bytes recv
            {
                let _profiler = DhatTester::new("test_channel_memory_mpsc_rx");
                for i in 0..LEN {
                    let d = rx.recv().await.unwrap();
                    assert_eq!(d, i * 10);
                }
            };
        }

        // watch channel
        {
            // 344 bytes, 1 block
            let (tx, mut rx) = {
                let _profiler = DhatTester::new("test_channel_memory_watch");
                let (tx, rx) = tokio::sync::watch::channel::<u32>(10);
                (tx, rx)
            };

            // 0 bytes send
            {
                let _profiler = DhatTester::new("test_channel_memory_watch_tx");
                tx.send_replace(10);
            };

            // 0 bytes recv
            {
                let _profiler = DhatTester::new("test_channel_memory_watch_rx");
                let d = *rx.borrow_and_update();
                assert_eq!(d, 10);
            };
        }

        // oneshot channel
        {
            // 64 bytes, 1 block
            let (tx, rx) = {
                let _profiler = DhatTester::new("test_channel_memory_oneshot");
                let (tx, rx) = tokio::sync::oneshot::channel::<u32>();
                (tx, rx)
            };

            // 0 bytes send
            {
                let _profiler = DhatTester::new("test_channel_memory_oneshot_tx");
                tx.send(10).unwrap();
            };

            // 0 bytes recv
            {
                let _profiler = DhatTester::new("test_channel_memory_oneshot_rx");
                let d = rx.await.unwrap();
                assert_eq!(d, 10);
            };
        }
    }
}
