use crate::{BehaviorTreeAsyncRunner, SafeDeltaType, async_nodes::AsyncActionType};

pub struct AsyncInvert<A> {
    child: AsyncActionType<A>,

    // state
    result: Option<bool>,
}

impl<A> AsyncInvert<A> {
    pub fn new(child: AsyncActionType<A>) -> Self {
        Self {
            child,
            result: None,
        }
    }

    pub fn reset<R>(&mut self, runner: R, delta: SafeDeltaType)
    where
        R: BehaviorTreeAsyncRunner<A> + Clone + 'static,
        A: Clone + 'static,
    {
        self.result = None;
        self.child.reset(runner, delta);
    }
}

impl<A> std::future::Future for AsyncInvert<A>
where
    A: Unpin,
{
    type Output = bool;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        println!("AsyncInvert::poll -> Start: {:?}", self.result);
        if let Some(result) = self.result {
            return std::task::Poll::Ready(result);
        }

        let child = &mut self.as_mut().get_mut().child;
        let status = std::pin::pin!(child).poll(cx).map(|s| !s);
        match status {
            std::task::Poll::Ready(result) => {
                self.get_mut().result = Some(result);
            }
            std::task::Poll::Pending => {}
        }
        println!("AsyncInvert::poll -> End: {:?}", status);
        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    use crate::{
        behavior_nodes::AsyncAction,
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    #[test]
    fn test_invert_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::new();
        let runner = std::rc::Rc::new(std::cell::RefCell::new(runner));
        std::thread::sleep(Duration::from_millis(50));

        let action = {
            let _profiler = DhatTester::new("test_invert_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let action = AsyncAction::new(runner.clone(), action, executor.delta().inner().into());
            let action = AsyncInvert::new(AsyncActionType::Action(action));
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
        assert_eq!(runner.borrow().num, 3);
        drop(runner);
    }
}
