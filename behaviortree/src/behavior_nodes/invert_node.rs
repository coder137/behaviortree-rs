use crate::{Action, Child, ChildState, Status};

pub struct InvertState<S> {
    //
    child: Child<S>,
    status: Option<Status>,
}

impl<S> InvertState<S> {
    pub fn new(child: Child<S>) -> Self {
        Self {
            child,
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

    fn child_state(&self) -> ChildState {
        ChildState::SingleChild(self.child.child_state_info())
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        test_behavior_interface::{TestActions, TestShared},
        Behavior,
    };

    use super::*;

    #[test]
    fn test_invert_success() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestActions::SuccessTimes { ticks: 1 });
        let mut invert = InvertState::new(Child::new(Box::from(behavior)));
        assert_eq!(
            invert.child_state(),
            ChildState::SingleChild(Rc::new(RefCell::new((ChildState::NoChild, None))))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        assert_eq!(
            invert.child_state(),
            ChildState::SingleChild(Rc::new(RefCell::new((
                ChildState::NoChild,
                Some(Status::Success)
            ))))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        assert_eq!(
            invert.child_state(),
            ChildState::SingleChild(Rc::new(RefCell::new((
                ChildState::NoChild,
                Some(Status::Success)
            ))))
        );
    }

    #[test]
    fn test_invert_failure() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestActions::FailureTimes { ticks: 1 });
        let mut invert = InvertState::new(Child::new(Box::from(behavior)));

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        assert_eq!(
            invert.child_state(),
            ChildState::SingleChild(Rc::new(RefCell::new((
                ChildState::NoChild,
                Some(Status::Failure)
            ))))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_invert_running_status() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestActions::Run {
            times: 1,
            output: Status::Failure,
        });
        let mut invert = InvertState::new(Child::new(Box::from(behavior)));

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        assert_eq!(
            invert.child_state(),
            ChildState::SingleChild(Rc::new(RefCell::new((
                ChildState::NoChild,
                Some(Status::Running)
            ))))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        assert_eq!(
            invert.child_state(),
            ChildState::SingleChild(Rc::new(RefCell::new((
                ChildState::NoChild,
                Some(Status::Failure)
            ))))
        );

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        assert_eq!(
            invert.child_state(),
            ChildState::SingleChild(Rc::new(RefCell::new((
                ChildState::NoChild,
                Some(Status::Failure)
            ))))
        );
    }

    #[test]
    fn test_invert_reset() {
        let mut shared = TestShared::default();

        let behavior = Behavior::Action(TestActions::SuccessWithCb {
            ticks: 2,
            cb: |mut m| {
                //
                m.expect_reset().times(1).returning(|| {});
                m
            },
        });
        let mut invert = InvertState::new(Child::new(Box::from(behavior)));

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);

        invert.reset();

        let status = invert.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }
}
