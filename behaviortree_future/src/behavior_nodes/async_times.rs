use crate::BehaviorTreeReset;

pub struct AsyncTimes<C> {
    child: Box<C>,
    current_times: u64,
    reset: bool,

    times: u64,
}

impl<C> AsyncTimes<C> {
    pub fn new(child: C, times: u64) -> Self {
        Self {
            child: Box::new(child),
            current_times: 0,
            reset: false,
            times,
        }
    }
}

impl<C> BehaviorTreeReset for AsyncTimes<C>
where
    C: BehaviorTreeReset,
{
    fn reset(&mut self) {
        self.current_times = 0;
        self.reset = false;
        self.child.reset();
    }
}

impl<C> std::future::Future for AsyncTimes<C>
where
    C: std::future::Future<Output = bool> + BehaviorTreeReset + Unpin,
{
    type Output = C::Output;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();

        if bt.current_times == bt.times {
            return std::task::Poll::Ready(true);
        }

        if bt.reset {
            bt.reset = false;
            bt.child.reset();
        }

        let child = std::pin::Pin::new(&mut bt.child);
        match child.poll(cx) {
            std::task::Poll::Ready(_s) => {
                bt.current_times += 1;
                if bt.current_times == bt.times {
                    std::task::Poll::Ready(true)
                } else {
                    bt.reset = true;
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                }
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use ticked_async_executor::TickedAsyncExecutor;

    use crate::{
        Behavior, Delta,
        async_behavior_state::AsyncBehaviorState,
        behavior_nodes::AsyncTimes,
        test_nodes::{TestOperation, TestOperationRunner},
    };

    #[test]
    fn test_times_0() {
        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let behavior = Behavior::Action(TestOperation::Add(1, 2, true, 0));
        let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
        let future = AsyncTimes::new(action, 0);

        let mut executor: TickedAsyncExecutor<fn(ticked_async_executor::TaskState)> =
            TickedAsyncExecutor::default();
        executor
            .spawn_local((), async move {
                let status = future.await;
                assert!(status);
            })
            .detach();

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 0);
        assert_eq!(runner.num.get(), 0);
    }

    #[test]
    fn test_times_1() {
        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let behavior = Behavior::Action(TestOperation::Add(1, 2, true, 0));
        let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
        let future = AsyncTimes::new(action, 1);

        let mut executor = TickedAsyncExecutor::default();
        executor
            .spawn_local((), async move {
                let status = future.await;
                assert!(status);
            })
            .detach();

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 0);
        assert_eq!(runner.num.get(), 3);
    }

    #[test]
    fn test_times_2() {
        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let behavior = Behavior::Action(TestOperation::Add(1, 2, true, 0));
        let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
        let future = AsyncTimes::new(action, 2);

        let mut executor = TickedAsyncExecutor::default();
        executor
            .spawn_local((), async move {
                let status = future.await;
                assert!(status);
            })
            .detach();

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 1);
        assert_eq!(runner.num.get(), 3);

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 0);
        assert_eq!(runner.num.get(), 6);
    }

    #[test]
    fn test_times_reset() {
        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let behavior = Behavior::Action(TestOperation::Add(1, 2, true, 1));
        let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
        let action = AsyncBehaviorState::Times(AsyncTimes::new(action, 1));

        let future = AsyncBehaviorState::Times(AsyncTimes::new(action, 2));

        let mut executor = TickedAsyncExecutor::default();
        executor
            .spawn_local((), async move {
                let status = future.await;
                assert!(status);
            })
            .detach();

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 1);
        assert_eq!(runner.num.get(), 0);

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 1);
        assert_eq!(runner.num.get(), 3);

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 1);
        assert_eq!(runner.num.get(), 3);

        executor.tick(10.0, None);
        assert_eq!(executor.num_tasks(), 0);
        assert_eq!(runner.num.get(), 6);
    }
}
