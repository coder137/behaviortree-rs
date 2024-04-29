use std::rc::Rc;

use crate::{Action, Child, ChildState, ChildStateInfo, Status};

pub struct SequenceState<S> {
    children: Vec<Child<S>>,
    index: usize,

    status: Option<Status>,
    state: Rc<[ChildStateInfo]>,
}

impl<S> SequenceState<S> {
    pub fn new(children: Vec<Child<S>>) -> Self {
        assert!(!children.is_empty());
        let state = Rc::from_iter(children.iter().map(|child| child.child_state_info()));
        Self {
            children,
            index: 0,
            status: None,
            state,
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

        let child = match self.children.get_mut(self.index) {
            Some(child) => child,
            None => unreachable!(),
        };
        let new_child_status = child.tick(dt, shared);
        let new_status = match new_child_status {
            Status::Success => {
                self.index += 1;
                match self.children.get(self.index) {
                    Some(_) => Status::Running,
                    None => Status::Success,
                }
            }
            Status::Failure => {
                self.index += 1;
                Status::Failure
            }
            Status::Running => Status::Running,
        };
        self.status = Some(new_status);
        new_status
    }

    fn reset(&mut self) {
        // Reset all ticked children
        self.children
            .iter_mut()
            .filter(|child| child.status().is_some())
            .for_each(|child| {
                child.reset();
            });

        self.index = 0;
        self.status = None;
    }

    fn child_state(&self) -> ChildState {
        ChildState::MultipleChildren(self.state.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::{
        convert_behaviors,
        test_behavior_interface::{TestActions, TestShared},
        Action, Behavior, Status,
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
        matches!(sequence.child_state(), ChildState::MultipleChildren(states) if states.len() == 1 && states[0] == Rc::new(RefCell::new((ChildState::NoChild, Some(Status::Success)))));

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
