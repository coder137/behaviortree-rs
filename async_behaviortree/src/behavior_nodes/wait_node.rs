use async_trait::async_trait;

use crate::AsyncAction;

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
    async fn run(
        &mut self,
        delta: &mut tokio::sync::watch::Receiver<f64>,
        _shared: &mut S,
    ) -> bool {
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
            tokio::task::yield_now().await;
        }
        true
    }

    fn reset(&mut self, _shared: &mut S) {
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
        let executor = TickedAsyncExecutor::default();

        let mut wait = AsyncWaitState::new(0.0);

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("WaitFuture", async move {
                wait.run(&mut delta, &mut shared).await;
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_wait_running() {
        let executor = TickedAsyncExecutor::default();

        let mut wait: Box<dyn AsyncAction<TestShared>> = Box::new(AsyncWaitState::new(49.0));

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("WaitFuture", async move {
                wait.run(&mut delta, &mut shared).await;
                wait.reset(&mut shared);
                wait.run(&mut delta, &mut shared).await;
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
        let executor = TickedAsyncExecutor::default();

        let mut wait = AsyncWaitState::new(50.0);

        let mut delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("WaitFuture", async move {
                wait.run(&mut delta, &mut shared).await;
            })
            .detach();

        executor.tick(DELTA, None);
        let mut r = executor.tick_channel();
        let mut r1 = r.clone();
        executor.tick(DELTA, None);
        assert!(r.has_changed().unwrap());
        assert!(r1.has_changed().unwrap());
        r.borrow_and_update();
        assert!(!r.has_changed().unwrap());
        assert!(r1.has_changed().unwrap());
        r1.borrow_and_update();
        assert!(!r.has_changed().unwrap());
        assert!(!r1.has_changed().unwrap());
        drop(executor);
    }
}
