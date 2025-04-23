use async_trait::async_trait;

use crate::{AsyncAction, async_child::AsyncChild};

pub struct AsyncInvertState<S> {
    child: AsyncChild<S>,
    completed: bool,
}

impl<S> AsyncInvertState<S> {
    pub fn new(child: AsyncChild<S>) -> Self {
        Self {
            child,
            completed: false,
        }
    }
}

#[async_trait(?Send)]
impl<S> AsyncAction<S> for AsyncInvertState<S> {
    async fn run(&mut self, delta: &mut tokio::sync::watch::Receiver<f64>, shared: &mut S) -> bool {
        match self.completed {
            true => {
                unreachable!()
            }
            false => {}
        }
        let status = !self.child.run(delta, shared).await;
        self.completed = true;
        status
    }

    fn reset(&mut self, shared: &mut S) {
        self.child.reset(shared);
        self.completed = false;
    }

    fn name(&self) -> &'static str {
        "Invert"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_async_behavior_interface::{DELTA, TestAction, TestShared};
    use behaviortree_common::Behavior;
    use ticked_async_executor::TickedAsyncExecutor;

    #[test]
    fn test_invert_success() {
        let behavior = Behavior::Invert(Behavior::Action(TestAction::Success).into());
        let mut invert = AsyncChild::from_behavior(behavior);

        let executor = TickedAsyncExecutor::default();

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("InvertFuture", async move {
                let status = invert.run(&mut delta, &mut shared).await;
                assert!(!status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_invert_failure() {
        let behavior = Behavior::Invert(Behavior::Action(TestAction::Failure).into());
        let mut invert = AsyncChild::from_behavior(behavior);

        let executor = TickedAsyncExecutor::default();

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("InvertFuture", async move {
                let status = invert.run(&mut delta, &mut shared).await;
                assert!(status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_invert_running_with_reset() {
        let behavior =
            Behavior::Invert(Behavior::Action(TestAction::SuccessAfter { times: 2 }).into());
        let mut invert = AsyncChild::from_behavior(behavior);

        let executor = TickedAsyncExecutor::default();

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("InvertFuture", async move {
                let status = invert.run(&mut delta, &mut shared).await;
                assert!(!status);
                invert.reset(&mut shared);
                let status = invert.run(&mut delta, &mut shared).await;
                assert!(!status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        executor.tick(DELTA, None);

        // Reset here

        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }
}
