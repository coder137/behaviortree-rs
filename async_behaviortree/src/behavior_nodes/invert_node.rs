use async_trait::async_trait;

use crate::{AsyncAction, AsyncChild};

pub struct AsyncInvertState<S> {
    pub child: AsyncChild<S>,
}

#[async_trait(?Send)]
impl<S> AsyncAction<S> for AsyncInvertState<S> {
    async fn run(&mut self, delta: &mut tokio::sync::watch::Receiver<f64>, shared: &mut S) -> bool {
        !self.child.run(delta, shared).await
    }

    fn reset(&mut self) {
        self.child.reset();
    }
}
