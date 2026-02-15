use crate::Behavior;
use crate::BehaviorTreeAsyncRunner;
use crate::SafeDeltaType;
use crate::async_nodes::AsyncActionType;

pub struct AsyncBehaviorTree<A, R> {
    runner: R,
    delta: SafeDeltaType,
    should_loop: bool,

    // state
    child: AsyncActionType<A>,
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
        let child = AsyncActionType::from_behavior(behavior, runner.clone(), delta.clone());
        Self::new(child, runner, delta, should_loop)
    }

    pub(crate) fn new(
        child: AsyncActionType<A>,
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

        println!("AsyncBehaviorTree::poll -> Start: {:?}", bt.result);
        if bt.result.is_some() && bt.should_loop {
            bt.result = None;
            bt.child.reset(bt.runner.clone(), bt.delta.clone());
        }

        let status = std::pin::pin!(&mut bt.child).poll(cx);
        let status = match status {
            std::task::Poll::Ready(result) => {
                bt.result = Some(result);
                if bt.should_loop {
                    std::task::Poll::Pending
                } else {
                    std::task::Poll::Ready(result)
                }
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        };
        println!("AsyncBehaviorTree::poll -> End: {:?}", status);
        println!("------");
        status
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::test_nodes::{TestOperation, TestOperationRunner};

    use super::*;

    #[test]
    fn test_behaviortree_operation_async() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::new();
        let runner = std::rc::Rc::new(std::cell::RefCell::new(runner));
        std::thread::sleep(Duration::from_millis(50));

        let action = {
            let action = TestOperation::Add(1, 2, true, 1);
            let action = AsyncBehaviorTree::from_behavior(
                Behavior::Action(action),
                runner,
                executor.delta().inner().into(),
                false,
            );
            // let stats = dhat::HeapStats::get();
            // println!("Stats: {stats:?}");
            action
        };

        executor
            .spawn_local("_", async move {
                // let _profiler = dhat::Profiler::builder()
                //     .file_name(format!("heap-post-action-operation-async.json"))
                //     .build();
                let status = action.await;
                assert!(status);
                // let stats = dhat::HeapStats::get();
                // println!("Stats: {stats:?}");
            })
            .detach();

        executor.wait_till_completed(16.67);
    }
}
