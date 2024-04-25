use crate::{Action, Status};

pub struct WaitState {
    target: f64,
    elapsed: f64,
}

impl<S> Action<S> for WaitState {
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

    fn reset(&mut self) {
        self.elapsed = 0.0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_behavior_interface::TestShared;

    #[test]
    fn test_tick_simple() {
        let mut shared = TestShared::default();

        let mut wait = WaitState::new(2.0);
        let wait_ref_mut: &mut dyn Action<TestShared> = &mut wait;

        let status = wait_ref_mut.tick(1.0, &mut shared);
        assert_eq!(status, Status::Running);

        let status = wait_ref_mut.tick(1.0, &mut shared);
        assert_eq!(status, Status::Success);

        let status = wait_ref_mut.tick(1.0, &mut shared);
        assert_eq!(status, Status::Success);

        // Reset
        wait_ref_mut.reset();

        let status = wait_ref_mut.tick(1.0, &mut shared);
        assert_eq!(status, Status::Running);

        let status = wait_ref_mut.tick(1.0, &mut shared);
        assert_eq!(status, Status::Success);
    }
}
