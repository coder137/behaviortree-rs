use crate::BehaviorTreeReset;

struct AsyncSequenceOrSelect<C> {
    children: Vec<C>,
    current_index: usize,

    //
    next_check: bool,
}

impl<C> AsyncSequenceOrSelect<C> {
    pub fn new(children: Vec<C>, next_check: bool) -> Self {
        Self {
            children,
            current_index: 0,
            next_check,
        }
    }
}

impl<C> BehaviorTreeReset for AsyncSequenceOrSelect<C>
where
    C: BehaviorTreeReset,
{
    fn reset(&mut self) {
        self.current_index = 0;
        self.children.iter_mut().for_each(|c| {
            c.reset();
        });
    }
}

impl<C> std::future::Future for AsyncSequenceOrSelect<C>
where
    C: std::future::Future<Output = bool> + Unpin,
{
    type Output = C::Output;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();
        loop {
            let current_child = &mut bt.children[bt.current_index];
            let current_child = std::pin::Pin::new(current_child);
            let child_status = current_child.poll(cx);
            let status = match child_status {
                std::task::Poll::Ready(result) => {
                    if result == bt.next_check {
                        // For sequence: true -> try next, or return true
                        // For select: false -> try next, or return false
                        bt.current_index += 1;
                        if bt.children.get(bt.current_index).is_none() {
                            std::task::Poll::Ready(bt.next_check)
                        } else {
                            continue;
                        }
                    } else {
                        // For sequence: false -> return false
                        // For select: true -> return true
                        std::task::Poll::Ready(!bt.next_check)
                    }
                }
                std::task::Poll::Pending => std::task::Poll::Pending,
            };
            break status;
        }
    }
}

pub struct AsyncSequence<C> {
    inner: AsyncSequenceOrSelect<C>,
}

impl<C> AsyncSequence<C> {
    pub fn new(children: Vec<C>) -> Self {
        Self {
            inner: AsyncSequenceOrSelect::new(children, true),
        }
    }
}

impl<C> BehaviorTreeReset for AsyncSequence<C>
where
    C: BehaviorTreeReset,
{
    fn reset(&mut self) {
        self.inner.reset();
    }
}

impl<C> std::future::Future for AsyncSequence<C>
where
    C: std::future::Future<Output = bool> + Unpin,
{
    type Output = C::Output;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();
        std::pin::pin!(&mut bt.inner).poll(cx)
    }
}

pub struct AsyncSelect<C> {
    inner: AsyncSequenceOrSelect<C>,
}

impl<C> AsyncSelect<C> {
    pub fn new(children: Vec<C>) -> Self {
        Self {
            inner: AsyncSequenceOrSelect::new(children, false),
        }
    }
}

impl<C> BehaviorTreeReset for AsyncSelect<C>
where
    C: BehaviorTreeReset,
{
    fn reset(&mut self) {
        self.inner.reset();
    }
}

impl<C> std::future::Future for AsyncSelect<C>
where
    C: std::future::Future<Output = bool> + Unpin,
{
    type Output = C::Output;

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
    use std::rc::Rc;

    use crate::{
        Behavior, Delta,
        async_behavior_state::AsyncBehaviorState,
        behavior_nodes::AsyncTimes,
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    // Sequence

    #[test]
    fn test_sequence_success_simple_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_sequence_success_simple_with_dhat_pre");
            let behavior = Behavior::Sequence(vec![Behavior::Action(TestOperation::Yield(true))]);
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_sequence_success_simple_with_dhat_post");
                let status = action.await;
                assert!(status);
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 0);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 0);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_success_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_sequence_success_with_dhat_pre");
            let behavior = Behavior::Sequence(vec![
                Behavior::Action(TestOperation::Add(1, 2, true, 0)),
                Behavior::Action(TestOperation::Add(1, 2, true, 0)),
            ]);
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_sequence_success_with_dhat_post");
                let status = action.await;
                assert!(status);
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 6);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_running_with_success_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_sequence_running_with_success_with_dhat_pre");
            let behavior = Behavior::Sequence(vec![
                Behavior::Action(TestOperation::Add(1, 2, true, 1)),
                Behavior::Action(TestOperation::Add(1, 2, true, 1)),
            ]);
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler =
                    DhatTester::new("test_sequence_running_with_success_with_dhat_post");
                let status = action.await;
                assert!(status);
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 0);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 3);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 6);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_failure_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_sequence_failure_with_dhat_pre");
            let behavior = Behavior::Sequence(vec![
                Behavior::Action(TestOperation::Add(1, 2, true, 0)),
                Behavior::Action(TestOperation::Add(1, 2, false, 0)),
                Behavior::Action(TestOperation::Add(1, 2, true, 0)),
            ]);
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_sequence_failure_with_dhat_post");
                let status = action.await;
                assert!(!status);
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 6);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_success_reset_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_sequence_success_reset_with_dhat_pre");
            let behavior = Behavior::Sequence(vec![
                Behavior::Action(TestOperation::Add(1, 2, true, 0)),
                Behavior::Action(TestOperation::Yield(true)),
                Behavior::Action(TestOperation::Add(1, 2, true, 0)),
                Behavior::Action(TestOperation::Yield(true)),
            ]);
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
            let action = AsyncTimes::new(action, 2);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_sequence_success_reset_with_dhat_post");
                let status = action.await;
                assert!(status);
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 3);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 6);
        assert_eq!(executor.num_tasks(), 1);

        // Reset happens here
        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 6);
        assert_eq!(executor.num_tasks(), 1);

        // execute
        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 9);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 12);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 12);
        assert_eq!(executor.num_tasks(), 0);
    }

    // Select

    #[test]
    fn test_select_failure_simple_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_select_failure_simple_with_dhat_pre");
            let behavior = Behavior::Select(vec![Behavior::Action(TestOperation::Yield(false))]);
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_select_failure_simple_with_dhat_post");
                let status = action.await;
                assert!(!status);
            })
            .detach();

        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_select_failure_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_select_failure_with_dhat_pre");
            let behavior = Behavior::Select(vec![
                Behavior::Action(TestOperation::Add(1, 2, false, 0)),
                Behavior::Action(TestOperation::Add(1, 2, false, 0)),
            ]);
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
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
        assert_eq!(runner.num.get(), 6);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_select_running_with_failure_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_select_running_with_failure_with_dhat_pre");
            let behavior = Behavior::Select(vec![
                Behavior::Action(TestOperation::Add(1, 2, false, 1)),
                Behavior::Action(TestOperation::Add(1, 2, false, 1)),
            ]);
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_select_running_with_failure_with_dhat_post");
                let status = action.await;
                assert!(!status);
            })
            .detach();

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 0);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 3);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 6);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_select_success_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_select_success_with_dhat_pre");
            let behavior = Behavior::Select(vec![
                Behavior::Action(TestOperation::Add(1, 2, false, 0)),
                Behavior::Action(TestOperation::Add(1, 2, true, 0)),
                Behavior::Action(TestOperation::Add(1, 2, false, 0)),
            ]);
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_select_success_with_dhat_post");
                let status = action.await;
                assert!(status);
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 6);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_select_failure_reset_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_select_failure_reset_with_dhat_pre");
            let behavior = Behavior::Select(vec![
                Behavior::Action(TestOperation::Add(1, 2, false, 0)),
                Behavior::Action(TestOperation::Yield(false)),
                Behavior::Action(TestOperation::Add(1, 2, false, 0)),
                Behavior::Action(TestOperation::Yield(false)),
            ]);
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
            let action = AsyncTimes::new(action, 2);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_select_failure_reset_with_dhat_post");
                let status = action.await;
                assert!(status);
            })
            .detach();

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 3);

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 6);

        // reset
        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 6);

        //
        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 9);

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 12);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 12);
        assert_eq!(executor.num_tasks(), 0);
    }
}
