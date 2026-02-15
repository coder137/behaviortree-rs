use crate::Behavior;
use crate::BehaviorTreeAsyncRunner;
use crate::SafeDeltaType;
use crate::async_behavior_state::AsyncBehaviorState;

pub struct AsyncBehaviorTree<A, R> {
    runner: R,
    delta: SafeDeltaType,
    should_loop: bool,

    // state
    child: AsyncBehaviorState<A>,
    result: Option<bool>,
}

impl<A, R> AsyncBehaviorTree<A, R> {
    pub fn from_behavior(
        behavior: Behavior<A>,
        runner: R,
        delta: SafeDeltaType,
        should_loop: bool,
    ) -> Self
    where
        R: BehaviorTreeAsyncRunner<A> + 'static,
        A: Clone + 'static,
    {
        let child = AsyncBehaviorState::from_behavior(behavior, runner.clone(), delta.clone());
        Self::new(child, runner, delta, should_loop)
    }

    pub(crate) fn new(
        child: AsyncBehaviorState<A>,
        runner: R,
        delta: SafeDeltaType,
        should_loop: bool,
    ) -> Self {
        Self {
            child,
            runner,
            delta,
            should_loop,
            result: None,
        }
    }
}

impl<A, R> std::future::Future for AsyncBehaviorTree<A, R>
where
    R: BehaviorTreeAsyncRunner<A> + Unpin + 'static,
    A: Clone + Unpin + 'static,
{
    type Output = bool;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();
        if bt.result.is_some() && bt.should_loop {
            bt.result = None;
            bt.child.reset(bt.runner.clone(), bt.delta.clone());
        }

        let child_status = std::pin::pin!(&mut bt.child).poll(cx);
        match child_status {
            std::task::Poll::Ready(result) => {
                bt.result = Some(result);
                if bt.should_loop {
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                } else {
                    std::task::Poll::Ready(result)
                }
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_nodes::{DhatTester, TestOperation, TestOperationRunner};

    use super::*;

    #[test]
    fn test_behaviortree_no_loop_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::new();
        let runner = std::rc::Rc::new(std::cell::RefCell::new(runner));

        let action = {
            let _profiler = DhatTester::new("test_behaviortree_no_loop_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let action = AsyncBehaviorTree::from_behavior(
                Behavior::Action(action),
                runner,
                executor.delta().inner().into(),
                false,
            );
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_behaviortree_no_loop_with_dhat_post");
                let status = action.await;
                assert!(status);
            })
            .detach();

        executor.wait_till_completed(16.67);
    }

    #[test]
    fn test_behaviortree_loop_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::new();
        let runner = std::rc::Rc::new(std::cell::RefCell::new(runner));

        let action = {
            let _profiler = DhatTester::new("test_behaviortree_loop_with_dhat_pre");
            let action = TestOperation::Add(1, 2, true, 1);
            let action = AsyncBehaviorTree::from_behavior(
                Behavior::Action(action),
                runner.clone(),
                executor.delta().inner().into(),
                true,
            );
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_behaviortree_loop_with_dhat_post");
                let status = action.await;
                assert!(status);
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(runner.borrow().num, 3);
        // Reset takes place
        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(runner.borrow().num, 6);
        //Reset takes place
        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(runner.borrow().num, 9);
        drop(executor);
    }
}
