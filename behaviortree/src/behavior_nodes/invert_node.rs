use crate::{Action, Behavior, ChildState, State, Status, ToAction};

pub struct InvertState<S> {
    // state
    status: Option<Status>,

    // child state
    current_action: Box<dyn Action<S>>,
    current_action_status: Option<Status>,
}

impl<S> InvertState<S>
where
    S: 'static,
{
    pub fn new<A>(behavior: Behavior<A>) -> Self
    where
        A: ToAction<S> + 'static,
    {
        let current_action: Box<dyn Action<S>> = Box::from(behavior);
        let current_action_status = None;
        Self {
            status: None,
            current_action,
            current_action_status,
        }
    }
}

impl<S> Action<S> for InvertState<S>
where
    S: 'static,
{
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        if let Some(status) = self.status {
            if status != Status::Running {
                return status;
            }
        }

        let child_status = self.current_action.tick(dt, shared);
        self.current_action_status = Some(child_status);
        let status = match child_status {
            Status::Success => Status::Failure,
            Status::Failure => Status::Success,
            Status::Running => Status::Running,
        };
        self.status = Some(status);
        status
    }

    fn halt(&mut self) {
        if let Some(child_status) = self.current_action_status {
            if child_status == Status::Running {
                self.current_action.halt();
                self.current_action_status = None;
            }
        }
        self.status = None;
    }

    fn state(&self) -> State {
        State::Invert(Box::new(ChildState::new(
            self.current_action.state(),
            self.current_action_status,
        )))
    }
}

#[cfg(test)]
mod tests {
    use crate::test_behavior_interface::{TestActions, TestShared};

    use super::*;

    #[test]
    fn test_invert_success() {
        let mut shared = TestShared::default();

        let mut invert = InvertState::new(Behavior::Action(TestActions::Success));
        assert_eq!(
            invert.state(),
            State::Invert(Box::new(ChildState::new(State::NoChild, None)))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        assert_eq!(
            invert.state(),
            State::Invert(Box::new(ChildState::new(
                State::NoChild,
                Some(Status::Success)
            )))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        assert_eq!(
            invert.state(),
            State::Invert(Box::new(ChildState::new(
                State::NoChild,
                Some(Status::Success)
            )))
        );
    }

    #[test]
    fn test_invert_failure() {
        let mut shared = TestShared::default();

        let mut invert = InvertState::new(Behavior::Action(TestActions::Failure));

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        assert_eq!(
            invert.state(),
            State::Invert(Box::new(ChildState::new(
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

        let mut invert = InvertState::new(Behavior::Action(TestActions::Run(1, Status::Failure)));

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        assert_eq!(
            invert.state(),
            State::Invert(Box::new(ChildState::new(
                State::NoChild,
                Some(Status::Running)
            )))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        assert_eq!(
            invert.state(),
            State::Invert(Box::new(ChildState::new(
                State::NoChild,
                Some(Status::Failure)
            )))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        assert_eq!(
            invert.state(),
            State::Invert(Box::new(ChildState::new(
                State::NoChild,
                Some(Status::Failure)
            )))
        );
    }

    #[test]
    fn test_invert_halt() {
        let mut shared = TestShared::default();

        let mut invert = InvertState::new(Behavior::Action(TestActions::Simulate(|mut mob| {
            mob.expect_tick()
                .once()
                .returning(|_dt, _shared| Status::Running);
            mob.expect_halt().return_once(|| {});
            mob.expect_state().returning(|| State::NoChild);
            mob
        })));

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        assert_eq!(
            invert.state(),
            State::Invert(Box::new(ChildState::new(
                State::NoChild,
                Some(Status::Running)
            )))
        );

        invert.halt();
        assert_eq!(invert.status, None);
        assert_eq!(
            invert.state(),
            State::Invert(Box::new(ChildState::new(State::NoChild, None)))
        );
    }
}
