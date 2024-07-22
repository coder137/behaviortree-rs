use crate::{Action, Child, ChildState, Status};

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
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        match self.completed {
            true => unreachable!(),
            false => {}
        }

        match self.child.tick(dt, shared) {
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

    fn child_state(&self) -> ChildState {
        ChildState::SingleChild(self.child.inner_state())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        test_behavior_interface::{TestAction, TestShared},
        Behavior, ChildStateInfo,
    };

    use super::*;

    #[test]
    fn test_invert_success() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestAction::Success);
        let mut invert = InvertState::new(Child::from(behavior));
        assert_eq!(
            invert.child_state(),
            ChildState::SingleChild(ChildStateInfo::from((ChildState::NoChild, None)))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        assert_eq!(
            invert.child_state(),
            ChildState::SingleChild(ChildStateInfo::from((
                ChildState::NoChild,
                Some(Status::Success)
            )))
        );
    }

    #[test]
    fn test_invert_failure() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestAction::Failure);
        let mut invert = InvertState::new(Child::from(behavior));

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        assert_eq!(
            invert.child_state(),
            ChildState::SingleChild(ChildStateInfo::from((
                ChildState::NoChild,
                Some(Status::Failure)
            )))
        );
    }

    #[test]
    fn test_invert_running_status() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestAction::FailureAfter { times: 1 });
        let mut invert = InvertState::new(Child::from(behavior));

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        assert_eq!(
            invert.child_state(),
            ChildState::SingleChild(ChildStateInfo::from((
                ChildState::NoChild,
                Some(Status::Running)
            )))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        assert_eq!(
            invert.child_state(),
            ChildState::SingleChild(ChildStateInfo::from((
                ChildState::NoChild,
                Some(Status::Failure)
            )))
        );
    }

    #[test]
    fn test_invert_reset() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestAction::Success);
        let mut invert = InvertState::new(Child::from(behavior));

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);

        invert.reset();

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }
}
