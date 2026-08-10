use crate::BehaviorTreeReset;

pub struct AsyncLoop<C> {
    child: Box<C>,
    completed: bool,
}

impl<C> AsyncLoop<C> {
    pub fn new(child: C) -> Self {
        Self {
            child: Box::new(child),
            completed: false,
        }
    }
}

impl<C> BehaviorTreeReset for AsyncLoop<C>
where
    C: BehaviorTreeReset,
{
    fn reset(&mut self) {
        self.completed = false;
        self.child.reset();
    }
}

impl<C> std::future::Future for AsyncLoop<C>
where
    C: std::future::Future<Output = bool> + BehaviorTreeReset + Unpin,
{
    type Output = C::Output;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();
        if bt.completed {
            bt.completed = false;
            bt.child.reset();
        }

        let child = std::pin::Pin::new(&mut bt.child);
        match child.poll(cx) {
            std::task::Poll::Ready(_s) => {
                bt.completed = true;
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use tokio_util::sync::CancellationToken;

    use crate::{
        Behavior, BehaviorTreeReset, Delta,
        async_behavior_state::AsyncBehaviorState,
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    #[test]
    fn test_async_loop_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_async_loop_with_dhat_pre");
            Behavior::Action(TestOperation::Add(1, 2, true, 1));
            let behavior =
                Behavior::Loop(Behavior::Action(TestOperation::Add(1, 2, true, 1)).into());
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
            action
        };

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_async_loop_with_dhat_post");
                let status = cancel_clone.run_until_cancelled_owned(action).await;
                println!("Status: {status:?}");
                assert!(status.is_none());
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 3);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 6);

        cancel.cancel();
        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_async_loop_reset_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let reset = std::rc::Rc::new(std::cell::Cell::new(false));

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let mut action = {
            let _profiler = DhatTester::new("test_async_loop_reset_with_dhat_pre");
            Behavior::Action(TestOperation::Add(1, 2, true, 1));
            let behavior =
                Behavior::Loop(Behavior::Action(TestOperation::Add(1, 2, true, 1)).into());
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
            action
        };

        let reset_clone = reset.clone();
        let future = std::future::poll_fn(move |cx| {
            let mut action = std::pin::Pin::new(&mut action);
            if reset_clone.get() {
                reset_clone.set(false);
                action.reset();
            }
            action.poll(cx)
        });

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_async_loop_reset_with_dhat_post");
                let status = cancel_clone.run_until_cancelled_owned(future).await;
                println!("Status: {status:?}");
                assert!(status.is_none());
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 3);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 6);

        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 6);
        reset.set(true);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 9);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(runner.num.get(), 12);

        cancel.cancel();
        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
    }
}
