use behaviortree_common::{ImmediateAction, Status};

use crate::SyncAction;

pub enum ActionType<S> {
    Immediate(Box<dyn ImmediateAction<S>>),
    Sync(Box<dyn SyncAction<S>>),
}

impl<S> ActionType<S> {
    pub fn tick(&mut self, delta: f64, shared: &mut S) -> Status {
        match self {
            ActionType::Immediate(immediate_action) => {
                let status = immediate_action.run(delta, shared);
                if status {
                    Status::Success
                } else {
                    Status::Failure
                }
            }
            ActionType::Sync(sync_action) => sync_action.tick(delta, shared),
        }
    }

    pub fn reset(&mut self, shared: &mut S) {
        match self {
            ActionType::Immediate(immediate_action) => immediate_action.reset(shared),
            ActionType::Sync(sync_action) => sync_action.reset(shared),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ActionType::Immediate(immediate_action) => immediate_action.name(),
            ActionType::Sync(sync_action) => sync_action.name(),
        }
    }
}
