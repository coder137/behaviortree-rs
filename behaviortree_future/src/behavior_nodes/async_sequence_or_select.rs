use crate::{
    AsyncActionContext, BehaviorTreeAsyncAction, BehaviorTreeReset,
    async_behavior_state::AsyncBehaviorState,
};

struct AsyncSequenceOrSelect<A, R> {
    children: Vec<AsyncBehaviorState<A, R>>,
    current_index: usize,

    //
    next_check: bool,
    ctx: AsyncActionContext<R>,
}

impl<A, R> AsyncSequenceOrSelect<A, R> {
    pub fn new(
        children: Vec<AsyncBehaviorState<A, R>>,
        next_check: bool,
        ctx: AsyncActionContext<R>,
    ) -> Self {
        Self {
            children,
            current_index: 0,
            next_check,
            ctx,
        }
    }
}

impl<A, R> BehaviorTreeReset<R> for AsyncSequenceOrSelect<A, R>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.current_index = 0;
        self.children.iter_mut().for_each(|c| {
            c.reset(ctx);
        });
    }
}

impl<A, R> std::future::Future for AsyncSequenceOrSelect<A, R>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    type Output = bool;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();

        let status = loop {
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
                            if bt.ctx.peek_delta() != 0.0 {
                                // if delta is not consumed, immediately tick the next child
                                continue;
                            } else {
                                // Tick again to poll the next child
                                cx.waker().wake_by_ref();
                                std::task::Poll::Pending
                            }
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
        };
        status
    }
}

pub struct AsyncSequence<A, R> {
    inner: AsyncSequenceOrSelect<A, R>,
}

impl<A, R> AsyncSequence<A, R> {
    pub fn new(children: Vec<AsyncBehaviorState<A, R>>, ctx: AsyncActionContext<R>) -> Self {
        Self {
            inner: AsyncSequenceOrSelect::new(children, true, ctx),
        }
    }
}

impl<A, R> BehaviorTreeReset<R> for AsyncSequence<A, R>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.inner.reset(ctx);
    }
}

impl<A, R> std::future::Future for AsyncSequence<A, R>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
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

pub struct AsyncSelect<A, R> {
    inner: AsyncSequenceOrSelect<A, R>,
}

impl<A, R> AsyncSelect<A, R> {
    pub fn new(children: Vec<AsyncBehaviorState<A, R>>, ctx: AsyncActionContext<R>) -> Self {
        Self {
            inner: AsyncSequenceOrSelect::new(children, false, ctx),
        }
    }
}

impl<A, R> BehaviorTreeReset<R> for AsyncSelect<A, R>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.inner.reset(ctx);
    }
}

impl<A, R> std::future::Future for AsyncSelect<A, R>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
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
        AsyncActionContextOwned,
        behavior_nodes::{AsyncAction, AsyncTimes},
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    // Sequence

    #[test]
    fn test_sequence_success_no_consume_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_sequence_success_no_consume_with_dhat_pre");
            let action = AsyncSequence::new(
                vec![
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Yield(true),
                        ctx.create_ctx(),
                    )),
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Add(1, 2, true, 0),
                        ctx.create_ctx(),
                    )),
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Yield(true),
                        ctx.create_ctx(),
                    )),
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Add(1, 2, true, 0),
                        ctx.create_ctx(),
                    )),
                ],
                ctx.create_ctx(),
            );
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_sequence_success_no_consume_with_dhat_post");
                let status = action.await;
                assert!(status);
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 3);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 6);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_success_consume_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_sequence_success_consume_with_dhat_pre");
            let action1 = AsyncAction::new(TestOperation::ConsumeDelta(true), ctx.create_ctx());
            let action2 = AsyncAction::new(TestOperation::ConsumeDelta(true), ctx.create_ctx());
            let action = AsyncSequence::new(
                vec![
                    AsyncBehaviorState::Action(action1),
                    AsyncBehaviorState::Action(action2),
                ],
                ctx.create_ctx(),
            );
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_sequence_success_consume_with_dhat_post");
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
    fn test_sequence_success_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_sequence_success_with_dhat_pre");
            let action1 = TestOperation::Add(1, 2, true, 1);
            let action2 = TestOperation::Add(1, 2, true, 1);
            let action1 = AsyncAction::new(action1, ctx.create_ctx());
            let action2 = AsyncAction::new(action2, ctx.create_ctx());
            let action = AsyncSequence::new(
                vec![
                    AsyncBehaviorState::Action(action1),
                    AsyncBehaviorState::Action(action2),
                ],
                ctx.create_ctx(),
            );
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
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 3);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 6);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_failure_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_sequence_failure_with_dhat_pre");
            let action1 = TestOperation::Add(1, 2, true, 1);
            let action2 = TestOperation::Add(1, 2, false, 1);
            let action3 = TestOperation::Add(1, 2, true, 1);
            let action1 = AsyncAction::new(action1, ctx.create_ctx());
            let action2 = AsyncAction::new(action2, ctx.create_ctx());
            let action3 = AsyncAction::new(action3, ctx.create_ctx());
            let action = AsyncSequence::new(
                vec![
                    AsyncBehaviorState::Action(action1),
                    AsyncBehaviorState::Action(action2),
                    AsyncBehaviorState::Action(action3),
                ],
                ctx.create_ctx(),
            );
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
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 3);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 6);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_sequence_success_reset_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_sequence_success_reset_with_dhat_pre");
            let sequence = AsyncSequence::new(
                vec![
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Add(1, 2, true, 0),
                        ctx.create_ctx(),
                    )),
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Yield(true),
                        ctx.create_ctx(),
                    )),
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Add(1, 2, true, 0),
                        ctx.create_ctx(),
                    )),
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Yield(true),
                        ctx.create_ctx(),
                    )),
                ],
                ctx.create_ctx(),
            );
            let action = AsyncBehaviorState::Sequence(sequence);

            let action = AsyncTimes::new(action, 2, ctx.create_ctx());
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
        assert_eq!(inner.get(), 3);

        executor.tick(16.67, None);
        assert_eq!(inner.get(), 6);

        // Reset happens here
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 6);

        // execute
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 9);

        executor.tick(16.67, None);
        assert_eq!(inner.get(), 12);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(16.67, None);
        assert_eq!(inner.get(), 12);
        assert_eq!(executor.num_tasks(), 0);
    }

    // Select

    #[test]
    fn test_select_failure_no_consume_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_select_failure_no_consume_with_dhat_pre");
            let action = AsyncSelect::new(
                vec![
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Yield(false),
                        ctx.create_ctx(),
                    )),
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Add(1, 2, false, 0),
                        ctx.create_ctx(),
                    )),
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Yield(false),
                        ctx.create_ctx(),
                    )),
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Add(1, 2, false, 0),
                        ctx.create_ctx(),
                    )),
                ],
                ctx.create_ctx(),
            );
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_select_failure_no_consume_with_dhat_post");
                let status = action.await;
                assert!(!status);
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 3);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 6);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_select_failure_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_select_failure_with_dhat_pre");
            let action1 = TestOperation::Add(1, 2, false, 1);
            let action2 = TestOperation::Add(1, 2, false, 1);
            let action1 = AsyncAction::new(action1, ctx.create_ctx());
            let action2 = AsyncAction::new(action2, ctx.create_ctx());
            let action = AsyncSelect::new(
                vec![
                    AsyncBehaviorState::Action(action1),
                    AsyncBehaviorState::Action(action2),
                ],
                ctx.create_ctx(),
            );
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
        assert_eq!(inner.get(), 3);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 6);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_select_success_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_select_success_with_dhat_pre");
            let action1 = TestOperation::Add(1, 2, false, 1);
            let action2 = TestOperation::Add(1, 2, true, 1);
            let action3 = TestOperation::Add(1, 2, false, 1);
            let action1 = AsyncAction::new(action1, ctx.create_ctx());
            let action2 = AsyncAction::new(action2, ctx.create_ctx());
            let action3 = AsyncAction::new(action3, ctx.create_ctx());
            let action = AsyncSelect::new(
                vec![
                    AsyncBehaviorState::Action(action1),
                    AsyncBehaviorState::Action(action2),
                    AsyncBehaviorState::Action(action3),
                ],
                ctx.create_ctx(),
            );
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
        assert_eq!(inner.get(), 3);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 6);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_select_failure_reset_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let runner = TestOperationRunner::default();
        let inner = runner.num.clone();
        let ctx = AsyncActionContextOwned::new(runner, 16.67);

        let action = {
            let _profiler = DhatTester::new("test_select_failure_reset_with_dhat_pre");
            let action = AsyncSelect::new(
                vec![
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Add(1, 2, false, 0),
                        ctx.create_ctx(),
                    )),
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Yield(false),
                        ctx.create_ctx(),
                    )),
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Add(1, 2, false, 0),
                        ctx.create_ctx(),
                    )),
                    AsyncBehaviorState::Action(AsyncAction::new(
                        TestOperation::Yield(false),
                        ctx.create_ctx(),
                    )),
                ],
                ctx.create_ctx(),
            );
            let action = AsyncBehaviorState::Select(action);
            let action = AsyncTimes::new(action, 2, ctx.create_ctx());
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
        assert_eq!(inner.get(), 3);

        executor.tick(16.67, None);
        assert_eq!(inner.get(), 6);

        // reset
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 6);

        //
        executor.tick(16.67, None);
        assert_eq!(inner.get(), 9);

        executor.tick(16.67, None);
        assert_eq!(inner.get(), 12);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(16.67, None);
        assert_eq!(inner.get(), 12);
        assert_eq!(executor.num_tasks(), 0);
    }
}
