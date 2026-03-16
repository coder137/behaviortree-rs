use crate::{BehaviorTreeAsyncRunner, SafeDeltaType};

pub struct AsyncAction<A> {
    action: A,
    // state
    future: reusable_box_future::ReusableLocalBoxFuture<bool>,
}

impl<A> AsyncAction<A> {
    pub fn new<R>(runner: R, action: A, delta: SafeDeltaType) -> Self
    where
        R: BehaviorTreeAsyncRunner<A> + 'static,
        A: Clone + 'static,
    {
        let future = runner.create_future(action.clone(), delta);
        let future = reusable_box_future::ReusableLocalBoxFuture::new(future);
        Self { action, future }
    }

    pub fn reset<R>(&mut self, mut runner: R, delta: SafeDeltaType)
    where
        R: BehaviorTreeAsyncRunner<A> + 'static,
        A: Clone + 'static,
    {
        runner.reset(&self.action);
        let future = runner.create_future(self.action.clone(), delta);
        self.future.set(future);
    }
}

impl<A> std::future::Future for AsyncAction<A>
where
    A: Unpin,
{
    type Output = bool;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.as_mut().get_mut().future.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        behavior_nodes::AsyncAction,
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    #[test]
    fn test_action_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::new();
        let runner = std::rc::Rc::new(std::cell::RefCell::new(runner));

        let action = {
            let _profiler = DhatTester::new("test_action_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let action = AsyncAction::new(runner.clone(), action, executor.delta().inner().into());
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_action_with_dhat_post");
                let status = action.await;
                assert!(status);
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                })
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
        assert_eq!(runner.borrow().num, 3);
    }

    #[test]
    fn test_action_retry_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();
        let delta = executor.delta().inner();

        let runner = TestOperationRunner::new();
        let runner = std::rc::Rc::new(std::cell::RefCell::new(runner));

        let mut action = {
            let _profiler = DhatTester::new("test_action_retry_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let action = AsyncAction::new(runner.clone(), action, delta.clone().into());
            action
        };

        let runner_clone = runner.clone();
        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_action_retry_with_dhat_post");

                let mut count = 2;

                // Poll and reset the future twice, before returning success
                let status = std::future::poll_fn(|cx| {
                    println!("Count: {}", count);
                    if count == 0 {
                        return std::task::Poll::Ready(true);
                    }

                    let status = std::pin::pin!(&mut action).poll(cx);
                    match status {
                        std::task::Poll::Ready(status) => {
                            count -= 1;
                            assert!(status);
                            action.reset(runner_clone.clone(), delta.clone().into());
                            cx.waker().wake_by_ref();
                            std::task::Poll::Pending
                        }
                        std::task::Poll::Pending => std::task::Poll::Pending,
                    }
                })
                .await;
                assert!(status);
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                })
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);

        executor.tick(16.67, None);
        executor.tick(16.67, None);

        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
        assert_eq!(runner.borrow().num, 6);
    }
}
