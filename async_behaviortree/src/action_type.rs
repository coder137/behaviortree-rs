use crate::{AsyncAction, ImmediateAction};

pub enum ActionType<S> {
    Immediate(Box<dyn ImmediateAction<S>>),
    Async(Box<dyn AsyncAction<S>>),
}

impl<S> ActionType<S> {
    pub async fn run(
        &mut self,
        delta: &mut tokio::sync::watch::Receiver<f64>,
        shared: &mut S,
    ) -> bool {
        match self {
            ActionType::Immediate(immediate_action) => immediate_action.run(shared),
            ActionType::Async(async_action) => async_action.run(delta, shared).await,
        }
    }

    pub fn reset(&mut self, shared: &mut S) {
        match self {
            ActionType::Immediate(immediate_action) => immediate_action.reset(shared),
            ActionType::Async(async_action) => async_action.reset(shared),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ActionType::Immediate(immediate_action) => immediate_action.name(),
            ActionType::Async(async_action) => async_action.name(),
        }
    }
}
