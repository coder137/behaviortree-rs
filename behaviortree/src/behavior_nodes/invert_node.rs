use crate::{Action, Child, Status};

pub struct InvertState<S> {
    child: Child<S>,
    completed: bool,
}

impl<S> InvertState<S> {
    pub fn new(child: Child<S>) -> Self {
        Self {
            child,
            completed: false,
        }
    }
}

impl<S> Action<S> for InvertState<S> {
    fn tick(&mut self, delta: f64, shared: &mut S) -> Status {
        match self.completed {
            true => unreachable!(),
            false => {}
        }

        match self.child.tick(delta, shared) {
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
        self.child.reset();
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

        let behavior = Behavior::Invert(Box::new(Behavior::Action(TestAction::Success)));
        let mut child = Child::from_behavior(behavior);

        let status = child.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_invert_failure() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestAction::Failure);
        let child = Child::from_behavior(behavior);
        let mut invert = InvertState::new(child);

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_invert_running_status() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestAction::FailureAfter { times: 1 });
        let child = Child::from_behavior(behavior);
        let mut invert = InvertState::new(child);

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_invert_reset() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestAction::Success);
        let child = Child::from_behavior(behavior);
        let mut invert = InvertState::new(child);

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);

        invert.reset();

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }
}
