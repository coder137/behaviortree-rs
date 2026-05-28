use crate::{AsyncActionContext, BehaviorTreeReset};

struct AsyncSequenceOrSelect<C, R> {
    children: Vec<C>,
    current_index: usize,

    //
    next_check: bool,
    ctx: AsyncActionContext<R>,
}

impl<C, R> AsyncSequenceOrSelect<C, R> {
    pub fn new(children: Vec<C>, next_check: bool, ctx: AsyncActionContext<R>) -> Self {
        Self {
            children,
            current_index: 0,
            next_check,
            ctx,
        }
    }
}

impl<C, R> BehaviorTreeReset<R> for AsyncSequenceOrSelect<C, R>
where
    C: BehaviorTreeReset<R>,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.current_index = 0;
        self.children.iter_mut().for_each(|c| {
            c.reset(ctx);
        });
    }
}

impl<C, R> std::future::Future for AsyncSequenceOrSelect<C, R>
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
        }
    }
}

pub struct AsyncSequence<C, R> {
    inner: AsyncSequenceOrSelect<C, R>,
}

impl<C, R> AsyncSequence<C, R> {
    pub fn new(children: Vec<C>, ctx: AsyncActionContext<R>) -> Self {
        Self {
            inner: AsyncSequenceOrSelect::new(children, true, ctx),
        }
    }
}

impl<C, R> BehaviorTreeReset<R> for AsyncSequence<C, R>
where
    C: BehaviorTreeReset<R>,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.inner.reset(ctx);
    }
}

impl<C, R> std::future::Future for AsyncSequence<C, R>
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

pub struct AsyncSelect<C, R> {
    inner: AsyncSequenceOrSelect<C, R>,
}

impl<C, R> AsyncSelect<C, R> {
    pub fn new(children: Vec<C>, ctx: AsyncActionContext<R>) -> Self {
        Self {
            inner: AsyncSequenceOrSelect::new(children, false, ctx),
        }
    }
}

impl<C, R> BehaviorTreeReset<R> for AsyncSelect<C, R>
where
    C: BehaviorTreeReset<R>,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.inner.reset(ctx);
    }
}

impl<C, R> std::future::Future for AsyncSelect<C, R>
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
    use super::*;

    use crate::{
        AsyncActionContextOwned,
        async_behavior_state::AsyncBehaviorState,
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
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Yield(true), ctx.create_ctx()),
                        None,
                    ),
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Add(1, 2, true, 0), ctx.create_ctx()),
                        None,
                    ),
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Yield(true), ctx.create_ctx()),
                        None,
                    ),
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Add(1, 2, true, 0), ctx.create_ctx()),
                        None,
                    ),
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
                    AsyncBehaviorState::<_, _, ()>::Action(action1, None),
                    AsyncBehaviorState::<_, _, ()>::Action(action2, None),
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
                    AsyncBehaviorState::<_, _, ()>::Action(action1, None),
                    AsyncBehaviorState::<_, _, ()>::Action(action2, None),
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
                    AsyncBehaviorState::<_, _, ()>::Action(action1, None),
                    AsyncBehaviorState::<_, _, ()>::Action(action2, None),
                    AsyncBehaviorState::<_, _, ()>::Action(action3, None),
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
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Add(1, 2, true, 0), ctx.create_ctx()),
                        None,
                    ),
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Yield(true), ctx.create_ctx()),
                        None,
                    ),
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Add(1, 2, true, 0), ctx.create_ctx()),
                        None,
                    ),
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Yield(true), ctx.create_ctx()),
                        None,
                    ),
                ],
                ctx.create_ctx(),
            );
            let action = AsyncBehaviorState::Sequence(sequence, None);

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
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Yield(false), ctx.create_ctx()),
                        None,
                    ),
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Add(1, 2, false, 0), ctx.create_ctx()),
                        None,
                    ),
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Yield(false), ctx.create_ctx()),
                        None,
                    ),
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Add(1, 2, false, 0), ctx.create_ctx()),
                        None,
                    ),
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
                    AsyncBehaviorState::<_, _, ()>::Action(action1, None),
                    AsyncBehaviorState::<_, _, ()>::Action(action2, None),
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
                    AsyncBehaviorState::<_, _, ()>::Action(action1, None),
                    AsyncBehaviorState::<_, _, ()>::Action(action2, None),
                    AsyncBehaviorState::<_, _, ()>::Action(action3, None),
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
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Add(1, 2, false, 0), ctx.create_ctx()),
                        None,
                    ),
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Yield(false), ctx.create_ctx()),
                        None,
                    ),
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Add(1, 2, false, 0), ctx.create_ctx()),
                        None,
                    ),
                    AsyncBehaviorState::<_, _, ()>::Action(
                        AsyncAction::new(TestOperation::Yield(false), ctx.create_ctx()),
                        None,
                    ),
                ],
                ctx.create_ctx(),
            );
            let action = AsyncBehaviorState::Select::<_, _, ()>(action, None);
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
