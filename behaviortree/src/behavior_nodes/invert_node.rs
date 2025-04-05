use crate::{Child, Decorator, Status};

pub struct InvertState {
    completed: bool,
}

impl InvertState {
    pub fn new() -> Self {
        Self { completed: false }
    }
}

impl<S> Decorator<S> for InvertState {
    fn tick(&mut self, child: &mut Child<S>, dt: f64, shared: &mut S) -> Status {
        match self.completed {
            true => unreachable!(),
            false => {}
        }

        match child.tick(dt, shared) {
            Status::Success => {
                self.completed = true;
                Status::Failure
            }
            Status::Failure => {
                self.completed = true;
                Status::Success
            }
            Status::Running => Status::Running,
        }
    }

    fn reset(&mut self) {
        self.completed = false;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        test_behavior_interface::{TestAction, TestShared},
        Behavior,
    };

    use super::*;

    #[test]
    fn test_invert_success() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestAction::Success);
        let mut child = Child::from_behavior(behavior);
        let mut invert = InvertState::new();

        let status = invert.tick(&mut child, 0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_invert_failure() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestAction::Failure);
        let mut child = Child::from_behavior(behavior);
        let mut invert = InvertState::new();

        let status = invert.tick(&mut child, 0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_invert_running_status() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestAction::FailureAfter { times: 1 });
        let mut child = Child::from_behavior(behavior);
        let mut invert = InvertState::new();

        let status = invert.tick(&mut child, 0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = invert.tick(&mut child, 0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_invert_reset() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestAction::Success);
        let mut child = Child::from_behavior(behavior);
        let mut invert = InvertState::new();

        let status = invert.tick(&mut child, 0.1, &mut shared);
        assert_eq!(status, Status::Failure);

        Decorator::<TestShared>::reset(&mut invert);

        let status = invert.tick(&mut child, 0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }
}
