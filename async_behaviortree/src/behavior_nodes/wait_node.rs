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
            async_std::task::yield_now().await;
        }
        true
    }

    fn reset(&mut self) {
        self.elapsed = 0.0;
    }
}
