use crate::{Action, Shared, Status};

pub struct WaitState {
    target: f64,
    elapsed: f64,
}

impl<S> Action<S> for WaitState
where
    S: Shared,
{
    fn tick(&mut self, dt: f64, _shared: &mut S) -> Status {
        if self.elapsed >= self.target {
            return Status::Success;
        }

        self.elapsed += dt;
        if self.elapsed >= self.target {
            Status::Success
        } else {
            Status::Running
        }
    }
}

impl WaitState {
    pub fn new(target: f64) -> Self {
        Self {
            target,
            elapsed: 0.0,
        }
    }
}
