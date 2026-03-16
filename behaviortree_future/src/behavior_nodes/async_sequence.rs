use crate::{BehaviorTreeAsyncRunner, SafeDeltaType, async_behavior_state::AsyncBehaviorState};

pub struct AsyncSequenceOrSelect<A> {
    children: Vec<AsyncBehaviorState<A>>,
    current_index: usize,

    //
    next_check: bool,
}

impl<A> AsyncSequenceOrSelect<A> {
    pub fn new(children: Vec<AsyncBehaviorState<A>>, next_check: bool) -> Self {
        Self {
            children,
            current_index: 0,
            next_check,
        }
    }

    pub fn reset<R>(&mut self, runner: R, delta: SafeDeltaType)
    where
        R: BehaviorTreeAsyncRunner<A> + 'static,
        A: Clone + 'static,
    {
        self.current_index = 0;
        self.children.iter_mut().for_each(|c| {
            c.reset(runner.clone(), delta.clone());
        });
    }
}

impl<A> std::future::Future for AsyncSequenceOrSelect<A>
where
    A: Unpin,
{
    type Output = bool;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();

        let current_child = &mut bt.children[bt.current_index];
        let child_status = std::pin::pin!(current_child).poll(cx);
        match child_status {
            std::task::Poll::Ready(result) => {
                if result == bt.next_check {
                    // For sequence: true -> try next, or return true
                    // For select: false -> try next, or return false
                    bt.current_index += 1;
                    if bt.children.get(bt.current_index).is_none() {
                        std::task::Poll::Ready(bt.next_check)
                    } else {
                        // Tick again to poll the next child
                        cx.waker().wake_by_ref();
                        std::task::Poll::Pending
                    }
                } else {
                    // For sequence: false -> return false
                    // For select: true -> return true
                    std::task::Poll::Ready(!bt.next_check)
                }
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

pub struct AsyncSequence<A> {
    children: Vec<AsyncBehaviorState<A>>,
    current_index: usize,
}

impl<A> AsyncSequence<A> {
    pub fn new(children: Vec<AsyncBehaviorState<A>>) -> Self {
        Self {
            children,
            current_index: 0,
        }
    }

    pub fn reset<R>(&mut self, runner: R, delta: SafeDeltaType)
    where
        R: BehaviorTreeAsyncRunner<A> + 'static,
        A: Clone + 'static,
    {
        self.current_index = 0;
        self.children.iter_mut().for_each(|c| {
            c.reset(runner.clone(), delta.clone());
        });
    }
}

impl<A> std::future::Future for AsyncSequence<A>
where
    A: Unpin,
{
    type Output = bool;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();

        let current_child = &mut bt.children[bt.current_index];
        let child_status = std::pin::pin!(current_child).poll(cx);
        match child_status {
            std::task::Poll::Ready(result) => {
                match result {
                    true => {
                        bt.current_index += 1;
                        if bt.children.get(bt.current_index).is_none() {
                            std::task::Poll::Ready(true)
                        } else {
                            // Tick again to poll the next child
                            cx.waker().wake_by_ref();
                            std::task::Poll::Pending
                        }
                    }
                    false => std::task::Poll::Ready(false),
                }
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

pub struct AsyncSelect<A> {
    inner: AsyncSequenceOrSelect<A>,
}

impl<A> AsyncSelect<A> {
    pub fn new(children: Vec<AsyncBehaviorState<A>>) -> Self {
        Self {
            inner: AsyncSequenceOrSelect::new(children, false),
        }
    }

    pub fn reset<R>(&mut self, runner: R, delta: SafeDeltaType)
    where
        R: BehaviorTreeAsyncRunner<A> + 'static,
        A: Clone + 'static,
    {
        self.inner.reset(runner, delta);
    }
}

impl<A> std::future::Future for AsyncSelect<A>
where
    A: Unpin,
{
    type Output = bool;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();
        std::pin::pin!(&mut bt.inner).poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        behavior_nodes::AsyncAction,
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    // Sequence

    #[test]
    fn test_sequence_success_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::new();
        let runner = std::rc::Rc::new(std::cell::RefCell::new(runner));

        let action = {
            let _profiler = DhatTester::new("test_sequence_success_with_dhat_pre");
            let action1 = TestOperation::Add(1, 2, true, 1);
            let action2 = TestOperation::Add(1, 2, true, 1);
            let action1 =
                AsyncAction::new(runner.clone(), action1, executor.delta().inner().into());
            let action2 =
                AsyncAction::new(runner.clone(), action2, executor.delta().inner().into());
            let action = AsyncSequence::new(vec![
                AsyncBehaviorState::Action(action1),
                AsyncBehaviorState::Action(action2),
            ]);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_sequence_success_with_dhat_post");
                let status = action.await;
                assert!(status);
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
        assert_eq!(runner.borrow().num, 6);
    }

    #[test]
    fn test_sequence_failure_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::new();
        let runner = std::rc::Rc::new(std::cell::RefCell::new(runner));

        let action = {
            let _profiler = DhatTester::new("test_sequence_failure_with_dhat_pre");
            let action1 = TestOperation::Add(1, 2, true, 1);
            let action2 = TestOperation::Add(1, 2, false, 1);
            let action3 = TestOperation::Add(1, 2, true, 1);
            let action1 =
                AsyncAction::new(runner.clone(), action1, executor.delta().inner().into());
            let action2 =
                AsyncAction::new(runner.clone(), action2, executor.delta().inner().into());
            let action3 =
                AsyncAction::new(runner.clone(), action3, executor.delta().inner().into());
            let action = AsyncSequence::new(vec![
                AsyncBehaviorState::Action(action1),
                AsyncBehaviorState::Action(action2),
                AsyncBehaviorState::Action(action3),
            ]);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_sequence_failure_with_dhat_post");
                let status = action.await;
                assert!(!status);
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
        assert_eq!(runner.borrow().num, 6);
    }

    // Select

    #[test]
    fn test_select_failure_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::new();
        let runner = std::rc::Rc::new(std::cell::RefCell::new(runner));

        let action = {
            let _profiler = DhatTester::new("test_select_failure_with_dhat_pre");
            let action1 = TestOperation::Add(1, 2, false, 1);
            let action2 = TestOperation::Add(1, 2, false, 1);
            let action1 =
                AsyncAction::new(runner.clone(), action1, executor.delta().inner().into());
            let action2 =
                AsyncAction::new(runner.clone(), action2, executor.delta().inner().into());
            let action = AsyncSelect::new(vec![
                AsyncBehaviorState::Action(action1),
                AsyncBehaviorState::Action(action2),
            ]);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_select_failure_with_dhat_post");
                let status = action.await;
                assert!(!status);
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
        assert_eq!(runner.borrow().num, 6);
    }

    #[test]
    fn test_select_success_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::new();
        let runner = std::rc::Rc::new(std::cell::RefCell::new(runner));

        let action = {
            let _profiler = DhatTester::new("test_select_success_with_dhat_pre");
            let action1 = TestOperation::Add(1, 2, false, 1);
            let action2 = TestOperation::Add(1, 2, true, 1);
            let action3 = TestOperation::Add(1, 2, false, 1);
            let action1 =
                AsyncAction::new(runner.clone(), action1, executor.delta().inner().into());
            let action2 =
                AsyncAction::new(runner.clone(), action2, executor.delta().inner().into());
            let action3 =
                AsyncAction::new(runner.clone(), action3, executor.delta().inner().into());
            let action = AsyncSelect::new(vec![
                AsyncBehaviorState::Action(action1),
                AsyncBehaviorState::Action(action2),
                AsyncBehaviorState::Action(action3),
            ]);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_select_success_with_dhat_post");
                let status = action.await;
                assert!(status);
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
        assert_eq!(runner.borrow().num, 6);
    }
}
