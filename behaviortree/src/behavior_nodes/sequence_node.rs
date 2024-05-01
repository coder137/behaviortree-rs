use crate::{Action, Child, ChildState, Children, Status};

pub struct SequenceState<S> {
    children: Children<S>,
    status: Option<Status>,
}

impl<S> SequenceState<S> {
    pub fn new(children: Vec<Child<S>>) -> Self {
        assert!(!children.is_empty());
        let children = Children::from(children);
        Self {
            children,
            status: None,
        }
    }
}

impl<S> Action<S> for SequenceState<S> {
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        // Once sequence is complete return the completed status
        if let Some(status) = self.status {
            if status != Status::Running {
                return status;
            }
        }

        let child = match self.children.current_child() {
            Some(child) => child,
            None => unreachable!(),
        };
        let new_child_status = child.tick(dt, shared);
        let new_status = match new_child_status {
            Status::Success => {
                self.children.next();
                match self.children.current_child() {
                    Some(_) => Status::Running,
                    None => Status::Success,
                }
            }
            Status::Failure => {
                self.children.next();
                Status::Failure
            }
            Status::Running => Status::Running,
        };
        self.status = Some(new_status);
        new_status
    }

    fn reset(&mut self) {
        self.children.reset();
        self.status = None;
    }

    fn child_state(&self) -> ChildState {
        ChildState::MultipleChildren(self.children.inner_state())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        convert_behaviors,
        test_behavior_interface::{TestActions, TestShared},
        Action, Behavior, ChildStateInfo, Status,
    };

    #[test]
    fn test_sequence_success() {
        let mut shared = TestShared::default();
        let mut sequence = SequenceState::new(convert_behaviors(vec![Behavior::Action(
            TestActions::SuccessTimes { ticks: 1 },
        )]));
        assert_eq!(sequence.status, None);

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        assert_eq!(sequence.status, Some(Status::Success));
        matches!(sequence.child_state(), ChildState::MultipleChildren(states) if states.len() == 1 && states[0] == ChildStateInfo::from((ChildState::NoChild, Some(Status::Success))));

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_sequence_failure() {
        let mut shared = TestShared::default();
        let mut sequence = SequenceState::new(convert_behaviors(vec![Behavior::Action(
            TestActions::FailureTimes { ticks: 1 },
        )]));
        assert_eq!(sequence.status, None);

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        assert_eq!(sequence.status, Some(Status::Failure));

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_sequence_run_then_status() {
        let mut shared = TestShared::default();
        let mut sequence = SequenceState::new(convert_behaviors(vec![Behavior::Action(
            TestActions::Run {
                times: 2,
                output: Status::Failure,
            },
        )]));
        assert_eq!(sequence.status, None);

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        println!("State: {:?}", sequence.child_state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        println!("State: {:?}", sequence.child_state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        println!("State: {:?}", sequence.child_state());
    }

    #[test]
    fn test_sequence_multiple_children() {
        let mut shared = TestShared::default();
        let mut sequence = SequenceState::new(convert_behaviors(vec![
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
        ]));
        assert_eq!(sequence.status, None);
        println!("State: {:?}", sequence.child_state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        println!("State: {:?}", sequence.child_state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        println!("State: {:?}", sequence.child_state());
    }

    #[test]
    fn test_sequence_multiple_children_early_failure() {
        let mut shared = TestShared::default();
        let mut sequence = SequenceState::new(convert_behaviors(vec![
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
            Behavior::Action(TestActions::FailureTimes { ticks: 1 }),
            Behavior::Action(TestActions::SuccessTimes { ticks: 0 }), // This never executes
        ]));

        assert_eq!(sequence.status, None);
        println!("State: {:?}", sequence.child_state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        println!("State: {:?}", sequence.child_state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        println!("State: {:?}", sequence.child_state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        println!("State: {:?}", sequence.child_state());
    }
}
