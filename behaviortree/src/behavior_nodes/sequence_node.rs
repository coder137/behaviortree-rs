use crate::{Action, ChildState, Children, Status};

pub struct SequenceState<S> {
    children: Children<S>,
    completed: bool,
}

impl<S> SequenceState<S> {
    pub fn new(children: Children<S>) -> Self {
        Self {
            children,
            completed: false,
        }
    }
}

impl<S> Action<S> for SequenceState<S> {
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        match self.completed {
            true => unreachable!(),
            false => {}
        }

        let child = match self.children.current_child() {
            Some(child) => child,
            None => unreachable!(),
        };
        match child.tick(dt, shared) {
            Status::Success => {
                self.children.next();
                match self.children.current_child() {
                    Some(_) => Status::Running,
                    None => {
                        self.completed = true;
                        Status::Success
                    }
                }
            }
            Status::Failure => {
                self.completed = true;
                Status::Failure
            }
            Status::Running => Status::Running,
        }
    }

    fn reset(&mut self) {
        self.children.reset();
        self.completed = false;
    }

    fn child_state(&self) -> ChildState {
        ChildState::MultipleChildren(self.children.inner_state())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_behavior_interface::{TestAction, TestShared},
        Action, Behavior, ChildStateInfo, Status,
    };

    #[test]
    fn test_sequence_success() {
        let mut shared = TestShared::default();
        let mut sequence =
            SequenceState::new(Children::from(vec![Behavior::Action(TestAction::Success)]));

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        matches!(sequence.child_state(), ChildState::MultipleChildren(states) if states.len() == 1 && states[0] == ChildStateInfo::from((ChildState::NoChild, Some(Status::Success))));
    }

    #[test]
    fn test_sequence_failure() {
        let mut shared = TestShared::default();
        let mut sequence =
            SequenceState::new(Children::from(vec![Behavior::Action(TestAction::Failure)]));

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_sequence_run_then_status() {
        let mut shared = TestShared::default();
        let mut sequence = SequenceState::new(Children::from(vec![Behavior::Action(
            TestAction::FailureAfter { times: 2 },
        )]));

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
        let mut sequence = SequenceState::new(Children::from(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]));

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
        let mut sequence = SequenceState::new(Children::from(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Failure),
            Behavior::Action(TestAction::Success), // This never executes
        ]));

        println!("State: {:?}", sequence.child_state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        println!("State: {:?}", sequence.child_state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        println!("State: {:?}", sequence.child_state());
    }
}
