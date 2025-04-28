use crate::{AsyncAction, async_behavior_interface::ImmediateAction};

pub enum AsyncActionType<S> {
    Immediate(Box<dyn ImmediateAction<S>>),
    Async(Box<dyn AsyncAction<S>>),
}

impl<S> AsyncActionType<S> {
    pub async fn run(&mut self, mut delta: tokio::sync::watch::Receiver<f64>, shared: &S) -> bool {
        match self {
            AsyncActionType::Immediate(immediate_action) => {
                let dt = *delta.borrow_and_update();
                immediate_action.run(dt, shared)
            }
            AsyncActionType::Async(async_action) => async_action.run(delta, shared).await,
        }
    }

    pub fn reset(&mut self, shared: &mut S) {
        match self {
            AsyncActionType::Immediate(immediate_action) => immediate_action.reset(shared),
            AsyncActionType::Async(async_action) => async_action.reset(shared),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            AsyncActionType::Immediate(immediate_action) => immediate_action.name(),
            AsyncActionType::Async(async_action) => async_action.name(),
        }
    }
}
