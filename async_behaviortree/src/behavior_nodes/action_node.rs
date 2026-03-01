use async_trait::async_trait;

use crate::{AsyncActionName, AsyncActionRunner, behavior_nodes::AsyncAction};

pub struct AsyncActionState<A> {
    action: A,
}

impl<A> AsyncActionState<A> {
    pub fn new(action: A) -> Self {
        Self { action }
    }
}

#[async_trait(?Send)]
impl<A, R> AsyncAction<R> for AsyncActionState<A>
where
    A: AsyncActionName,
    R: AsyncActionRunner<A>,
{
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, runner: &mut R) -> bool {
        runner.run(delta, &self.action).await
    }

    fn reset(&mut self, runner: &mut R) {
        runner.reset(&self.action);
    }

    fn name(&self) -> &'static str {
        self.action.name()
    }
}
