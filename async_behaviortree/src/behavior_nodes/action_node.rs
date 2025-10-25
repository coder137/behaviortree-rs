use async_trait::async_trait;

use crate::{AsyncAction, AsyncActionName, AsyncBehaviorRunner};

pub struct AsyncActionState<A> {
    action: A,
}

impl<A> AsyncActionState<A> {
    pub fn new(action: A) -> Self {
        Self { action }
    }
}

#[async_trait(?Send)]
impl<A, S> AsyncAction<S> for AsyncActionState<A>
where
    A: AsyncActionName,
    S: AsyncBehaviorRunner<A>,
{
    #[tracing::instrument(level = "trace", name = "Action::run", skip_all, ret)]
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, shared: &mut S) -> bool {
        shared.run(delta, &self.action).await
    }

    #[tracing::instrument(level = "trace", name = "Action::reset", skip_all, ret)]
    fn reset(&mut self, shared: &mut S) {
        shared.reset(&self.action);
    }

    fn name(&self) -> &'static str {
        self.action.name()
    }
}
