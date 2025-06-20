use async_trait::async_trait;

use crate::{AsyncAction, util::yield_now};

pub struct AsyncWaitState {
    target: f64,
    elapsed: f64,
}

impl AsyncWaitState {
    pub fn new(target: f64) -> Self {
        Self {
            target,
            elapsed: 0.0,
        }
    }
}

#[async_trait(?Send)]
impl<S> AsyncAction<S> for AsyncWaitState {
    #[tracing::instrument(level = "trace", name = "Wait::run", skip_all, ret)]
    async fn run(&mut self, mut delta: tokio::sync::watch::Receiver<f64>, _shared: &S) -> bool {
        loop {
            let _r = delta.changed().await;
            if _r.is_err() {
                // This means that the executor supplying the delta channel has shutdown
                // We must stop waiting gracefully
                break;
            }
            self.elapsed += *(delta.borrow_and_update());
            if self.elapsed >= self.target {
                break;
            }
            yield_now().await;
        }
        true
    }

    #[tracing::instrument(level = "trace", name = "Wait::reset", skip_all, ret)]
    fn reset(&mut self, _shared: &S) {
        self.elapsed = 0.0;
    }

    fn name(&self) -> &'static str {
        "Wait"
    }
}

#[cfg(test)]
mod tests {
    use ticked_async_executor::TickedAsyncExecutor;

    use super::*;
    use crate::test_async_behavior_interface::{DELTA, TestShared};

    #[test]
    fn test_wait_success() {
        let mut executor = TickedAsyncExecutor::default();

        let mut wait = AsyncWaitState::new(0.0);

        let delta = executor.tick_channel();
        let shared = TestShared;

        executor
            .spawn_local("WaitFuture", async move {
                wait.run(delta, &shared).await;
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
        let shared = TestShared;

        executor
            .spawn_local("WaitFuture", async move {
                wait.run(delta, &shared).await;
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

        let mut wait: Box<dyn AsyncAction<TestShared>> = Box::new(AsyncWaitState::new(49.0));

        let delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("WaitFuture", async move {
                wait.run(delta.clone(), &shared).await;
                wait.reset(&mut shared);
                wait.run(delta, &shared).await;
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
        let shared = TestShared;

        executor
            .spawn_local("WaitFuture", async move {
                wait.run(delta, &shared).await;
            })
            .detach();

        executor.tick(DELTA, None);
        drop(executor);
    }
}
