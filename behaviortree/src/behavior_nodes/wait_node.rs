use behaviortree_common::Status;

use crate::SyncAction;

pub struct WaitState {
    target: f64,
    elapsed: f64,
}

impl<S> SyncAction<S> for WaitState {
    #[tracing::instrument(level = "trace", name = "Wait", skip_all, ret)]
    fn tick(&mut self, dt: f64, _shared: &mut S) -> Status {
        match self.elapsed >= self.target {
            true => unreachable!(),
            false => {}
        }

        self.elapsed += dt;
        if self.elapsed >= self.target {
            Status::Success
        } else {
            Status::Running
        }
    }

    fn reset(&mut self, _shared: &mut S) {
        self.elapsed = 0.0;
    }

    fn name(&self) -> &'static str {
        "Wait"
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
    use behaviortree_common::Behavior;

    use super::*;
    use crate::{
        child::Child,
        test_behavior_interface::{TestAction, TestShared},
    };

    #[test]
    fn test_wait() {
        let mut shared = TestShared::default();

        let mut wait = WaitState::new(2.0);
        let wait_ref_mut: &mut dyn SyncAction<TestShared> = &mut wait;

        let status = wait_ref_mut.tick(1.0, &mut shared);
        assert_eq!(status, Status::Running);

        let status = wait_ref_mut.tick(1.0, &mut shared);
        assert_eq!(status, Status::Success);

        // Reset
        wait_ref_mut.reset(&mut shared);

        let status = wait_ref_mut.tick(1.0, &mut shared);
        assert_eq!(status, Status::Running);

        let status = wait_ref_mut.tick(1.0, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_wait_from_behavior() {
        let mut shared = TestShared::default();

        let mut wait = Child::from_behavior::<TestAction>(Behavior::Wait(2.0));

        let status = wait.tick(1.0, &mut shared);
        assert_eq!(status, Status::Running);

        let status = wait.tick(1.0, &mut shared);
        assert_eq!(status, Status::Success);

        // Reset
        wait.reset(&mut shared);

        let status = wait.tick(1.0, &mut shared);
        assert_eq!(status, Status::Running);

        let status = wait.tick(1.0, &mut shared);
        assert_eq!(status, Status::Success);
    }
}
