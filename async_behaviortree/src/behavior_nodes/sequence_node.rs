use async_trait::async_trait;

use crate::{AsyncAction, AsyncChild};

pub struct AsyncSequenceState<S> {
    pub children: Vec<AsyncChild<S>>,
}

#[async_trait(?Send)]
impl<S> AsyncAction<S> for AsyncSequenceState<S> {
    async fn run(&mut self, delta: &mut tokio::sync::watch::Receiver<f64>, shared: &mut S) -> bool {
        let last = self.children.len() - 1;
        for (index, child) in self.children.iter_mut().enumerate() {
            let child_status = child.run(delta, shared).await;
            if !child_status {
                return false;
            }
            // Only one child should be run per tick
            // This means that if they are more children after the current child,
            // we must yield back to the executor
            if index != last {
                async_std::task::yield_now().await;
            }
        }
        true
    }

    fn reset(&mut self) {
        self.children.iter_mut().for_each(|child| {
            child.reset();
        });
    }
}
