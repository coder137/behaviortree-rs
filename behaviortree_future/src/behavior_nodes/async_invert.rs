use crate::BehaviorTreeReset;

pub struct AsyncInvert<C> {
    child: Box<C>,
}

impl<C> AsyncInvert<C> {
    pub fn new(child: C) -> Self {
        Self {
            child: child.into(),
        }
    }
}

impl<C> BehaviorTreeReset for AsyncInvert<C>
where
    C: BehaviorTreeReset,
{
    fn reset(&mut self) {
        self.child.reset();
    }
}

impl<C> std::future::Future for AsyncInvert<C>
where
    C: std::future::Future<Output = bool> + Unpin,
{
    type Output = C::Output;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();
        let child = std::pin::Pin::new(&mut bt.child);
        child.poll(cx).map(|s| !s)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;

    use crate::{
        ActionToActionState, Behavior, Delta,
        async_behavior_state::AsyncBehaviorState,
        behavior_nodes::{AsyncAction, AsyncTimes},
        test_nodes::{DhatTester, TestOperation, TestOperationRunner},
    };

    #[test]
    fn test_invert_success_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_invert_success_with_dhat_pre");
            let behavior = Behavior::Invert(Behavior::Action(TestOperation::Yield(true)).into());
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_invert_success_with_dhat_post");
                let status = action.await;
                assert!(!status);
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
    fn test_invert_failure_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_invert_failure_with_dhat_pre");
            let behavior = Behavior::Invert(Behavior::Action(TestOperation::Yield(false)).into());
            let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_invert_failure_with_dhat_post");
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
    fn test_invert_reset_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();
        let delta = Rc::new(Delta::default());

        let action = {
            let _profiler = DhatTester::new("test_invert_reset_with_dhat_pre");
            // action
            let action = TestOperation::Yield(true);
            let action = action.to_state(delta, &mut runner);
            let action = AsyncAction::new(action);
            let action = AsyncBehaviorState::Action(action);
            // invert
            let action = AsyncInvert::new(action.into());
            let action = AsyncBehaviorState::Invert(action);
            // times
            let action = AsyncTimes::new(action, 2);
            action
        };

        executor
            .spawn_local("_", async move {
                let _profiler = DhatTester::new("test_invert_reset_with_dhat_post");
                let status = action.await;
                assert!(status);
                DhatTester::stats(|stats| {
                    assert_eq!(stats.total_bytes, 0);
                });
            })
            .detach();

        executor.tick(16.67, None);
        executor.tick(16.67, None);

        executor.tick(16.67, None);
        executor.tick(16.67, None);
        assert_eq!(executor.num_tasks(), 0);
    }
}
