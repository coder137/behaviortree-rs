use crate::{Action, Behavior, Child, State, Status, ToAction};

pub struct InvertState<S> {
    //
    child: Child<S>,

    // state
    status: Option<Status>,
}

impl<S> InvertState<S> {
    pub fn new<A>(behavior: Behavior<A>) -> Self
    where
        A: ToAction<S> + 'static,
        S: 'static,
    {
        let action: Box<dyn Action<S>> = Box::from(behavior);
        Self {
            child: Child::new(action, None),
            status: None,
        }
    }
}

impl<S> Action<S> for InvertState<S> {
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        if let Some(status) = self.status {
            if status != Status::Running {
                return status;
            }
        }

        let child_status = self.child.tick(dt, shared);
        let status = match child_status {
            Status::Success => Status::Failure,
            Status::Failure => Status::Success,
            Status::Running => Status::Running,
        };
        self.status = Some(status);
        status
    }

    fn reset(&mut self) {
        self.child.reset();
        self.status = None;
    }

    fn state(&self) -> State {
        State::SingleChild(Box::new(self.child.child_state()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        test_behavior_interface::{TestActions, TestShared},
        ChildState,
    };

    use super::*;

    #[test]
    fn test_invert_success() {
        let mut shared = TestShared::default();

        let mut invert = InvertState::new(Behavior::Action(TestActions::SuccessTimes { ticks: 1 }));
        assert_eq!(
            invert.state(),
            State::SingleChild(Box::new(ChildState::new(State::NoChild, None)))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        assert_eq!(
            invert.state(),
            State::SingleChild(Box::new(ChildState::new(
                State::NoChild,
                Some(Status::Success)
            )))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        assert_eq!(
            invert.state(),
            State::SingleChild(Box::new(ChildState::new(
                State::NoChild,
                Some(Status::Success)
            )))
        );
    }

    #[test]
    fn test_invert_failure() {
        let mut shared = TestShared::default();

        let mut invert = InvertState::new(Behavior::Action(TestActions::FailureTimes { ticks: 1 }));

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        assert_eq!(
            invert.state(),
            State::SingleChild(Box::new(ChildState::new(
                State::NoChild,
                Some(Status::Failure)
            )))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_invert_running_status() {
        let mut shared = TestShared::default();

        let mut invert = InvertState::new(Behavior::Action(TestActions::Run {
            times: 1,
            output: Status::Failure,
        }));

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        assert_eq!(
            invert.state(),
            State::SingleChild(Box::new(ChildState::new(
                State::NoChild,
                Some(Status::Running)
            )))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        assert_eq!(
            invert.state(),
            State::SingleChild(Box::new(ChildState::new(
                State::NoChild,
                Some(Status::Failure)
            )))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        assert_eq!(
            invert.state(),
            State::SingleChild(Box::new(ChildState::new(
                State::NoChild,
                Some(Status::Failure)
            )))
        );
    }
}
