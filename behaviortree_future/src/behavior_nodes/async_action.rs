use crate::{BehaviorTreeAsyncRunner, SafeDeltaType};

pub struct AsyncAction<A> {
    action: A,
    // state
    result: Option<bool>,
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
        Self {
            action,
            result: None,
            future,
        }
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
        if let Some(result) = self.result {
            return std::task::Poll::Ready(result);
        }
        let status = self.as_mut().get_mut().future.poll(cx);
        match status {
            std::task::Poll::Ready(result) => {
                self.get_mut().result = Some(result);
            }
            std::task::Poll::Pending => {}
        }
        status
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        behavior_nodes::AsyncAction,
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    #[test]
    fn test_action_operation_add_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::new();
        let runner = std::rc::Rc::new(std::cell::RefCell::new(runner));
        std::thread::sleep(Duration::from_millis(50));

        let action = {
            let _profiler = DhatTester::new("test_action_operation_add_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 10);
            let action = AsyncAction::new(runner.clone(), action, executor.delta().inner().into());
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_action_operation_add_with_dhat_post");
                let status = action.await;
                assert!(status);
                let stats = dhat::HeapStats::get();
                println!("Stats: {stats:?}");
            })
            .detach();

        executor.wait_till_completed(16.67);
        assert_eq!(executor.num_tasks(), 0);
        assert_eq!(runner.borrow().num, 3);
    }
}
