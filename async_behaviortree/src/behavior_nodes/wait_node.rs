use std::marker::PhantomData;

use async_trait::async_trait;

use crate::{AsyncActionRunner, behavior_nodes::AsyncAction};

pub struct AsyncWaitState<A> {
    target: f64,
    inner: PhantomData<A>,
}

impl<A> AsyncWaitState<A> {
    pub fn new(target: f64) -> Self {
        Self {
            target,
            inner: PhantomData::default(),
        }
    }
}

#[async_trait(?Send)]
impl<A, R> AsyncAction<R> for AsyncWaitState<A>
where
    R: AsyncActionRunner<A>,
{
    #[tracing::instrument(level = "trace", name = "Wait::run", skip_all, ret, fields(target = self.target))]
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, runner: &mut R) -> bool {
        runner.wait(delta, self.target).await
    }

    fn reset(&mut self, _runner: &mut R) {}

    fn name(&self) -> &'static str {
        "Wait"
    }
}

#[cfg(test)]
mod tests {
    use ticked_async_executor::TickedAsyncExecutor;

    use super::*;
    use crate::test_async_behavior_interface::{DELTA, TestRunner};

    #[test]
    fn test_wait_success() {
        let mut executor = TickedAsyncExecutor::default();

        let mut wait = AsyncWaitState::new(0.0);

        let delta = executor.tick_channel();
        let mut runner = TestRunner;

        executor
            .spawn_local("WaitFuture", async move {
                wait.run(delta, &mut runner).await;
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_wait_success_with_time() {
        let mut executor = TickedAsyncExecutor::default();

        let mut wait = AsyncWaitState::new(1.0);

        let delta = executor.tick_channel();
        let mut runner = TestRunner;

        executor
            .spawn_local("WaitFuture", async move {
                wait.run(delta, &mut runner).await;
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);

        executor.tick(0.5, None);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(0.5, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_wait_running() {
        let mut executor = TickedAsyncExecutor::default();

        let mut wait: Box<dyn AsyncAction<TestRunner>> = Box::new(AsyncWaitState::new(49.0));

        let delta = executor.tick_channel();
        let mut runner = TestRunner;

        executor
            .spawn_local("WaitFuture", async move {
                wait.run(delta.clone(), &mut runner).await;
                wait.reset(&mut runner);
                wait.run(delta, &mut runner).await;
            })
            .detach();

        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        executor.tick(DELTA, None);

        // reset

        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_executor_drop() {
        let mut executor = TickedAsyncExecutor::default();

        let mut wait = AsyncWaitState::new(50.0);

        let delta = executor.tick_channel();
        let mut runner = TestRunner;

        executor
            .spawn_local("WaitFuture", async move {
                wait.run(delta, &mut runner).await;
            })
            .detach();

        executor.tick(DELTA, None);
        drop(executor);
    }
}
