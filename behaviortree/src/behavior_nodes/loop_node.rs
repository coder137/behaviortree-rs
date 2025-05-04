use behaviortree_common::Status;

use crate::{SyncAction, child::Child};

pub struct LoopState<S> {
    child: Child<S>,
}

impl<S> LoopState<S> {
    pub fn new(child: Child<S>) -> Self {
        Self { child }
    }
}

impl<S> SyncAction<S> for LoopState<S> {
    fn tick(&mut self, delta: f64, shared: &mut S) -> Status {
        let child_status = self.child.status();
        if let Some(child_status) = child_status {
            if child_status != Status::Running {
                self.child.reset(shared);
            }
        }

        self.child.tick(delta, shared)
    }

    fn reset(&mut self, shared: &mut S) {
        self.child.reset(shared);
    }

    fn name(&self) -> &'static str {
        "Loop"
    }
}
