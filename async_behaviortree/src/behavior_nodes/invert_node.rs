use async_trait::async_trait;

use crate::{AsyncChild, AsyncDecorator};

pub struct AsyncInvertState {
    completed: bool,
}

impl AsyncInvertState {
    pub fn new() -> Self {
        Self { completed: false }
    }
}

#[async_trait(?Send)]
impl<S> AsyncDecorator<S> for AsyncInvertState {
    async fn run(
        &mut self,
        child: &mut AsyncChild<S>,
        delta: &mut tokio::sync::watch::Receiver<f64>,
        shared: &mut S,
    ) -> bool {
        match self.completed {
            true => {
                unreachable!()
            }
            false => {}
        }
        let status = !child.run(delta, shared).await;
        self.completed = true;
        status
    }

    fn reset(&mut self) {
        self.completed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_async_behavior_interface::{TestAction, TestShared, DELTA},
        AsyncChild,
    };
    use behaviortree_common::Behavior;
    use ticked_async_executor::TickedAsyncExecutor;

    #[test]
    fn test_invert_success() {
        let behavior = Behavior::Action(TestAction::Success);
        let mut child = AsyncChild::from_behavior(behavior);

        let mut invert = AsyncInvertState::new();

        let executor = TickedAsyncExecutor::default();

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("InvertFuture", async move {
                let status = invert.run(&mut child, &mut delta, &mut shared).await;
                assert!(!status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_invert_failure() {
        let behavior = Behavior::Action(TestAction::Failure);
        let mut child = AsyncChild::from_behavior(behavior);

        let mut invert = AsyncInvertState::new();

        let executor = TickedAsyncExecutor::default();

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("InvertFuture", async move {
                let status = invert.run(&mut child, &mut delta, &mut shared).await;
                assert!(status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_invert_running_with_reset() {
        let behavior = Behavior::Action(TestAction::SuccessAfter { times: 2 });
        let mut child = AsyncChild::from_behavior(behavior);

        let mut invert: Box<dyn AsyncDecorator<TestShared>> = Box::new(AsyncInvertState::new());

        let executor = TickedAsyncExecutor::default();

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("InvertFuture", async move {
                let status = invert.run(&mut child, &mut delta, &mut shared).await;
                assert!(!status);
                invert.reset();
                let status = invert.run(&mut child, &mut delta, &mut shared).await;
                assert!(!status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA);
        executor.tick(DELTA);
        executor.tick(DELTA);

        // Reset here

        executor.tick(DELTA);
        executor.tick(DELTA);
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }
}
