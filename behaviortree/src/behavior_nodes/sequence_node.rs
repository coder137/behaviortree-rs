use crate::{Action, Child, ChildState, State, Status};

pub struct SequenceState<S> {
    children: Vec<Child<S>>,
    index: usize,

    // state
    status: Option<Status>,
}

impl<S> Action<S> for SequenceState<S> {
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        // Once sequence is complete return the completed status
        if let Some(status) = self.status {
            if status != Status::Running {
                return status;
            }
        }

        let child = &mut self.children[self.index];
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
        // Reset all children
        for i in 0..=self.index {
            self.children[i].reset();
        }

        self.index = 0;
        self.status = None;
    }

    fn state(&self) -> State {
        let child_states = self
            .children
            .iter()
            .take(self.index + 1)
            .map(|child| child.child_state())
            .collect::<Vec<ChildState>>();
        State::MultipleChildren(child_states)
    }
}

impl<S> SequenceState<S> {
    pub fn new(children: Vec<Child<S>>) -> Self
    where
        S: 'static,
    {
        assert!(!children.is_empty());
        Self {
            children,
            index: 0,
            status: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_behavior_interface::{TestActions, TestShared},
        Action, Behavior, Status, ToAction,
    };

    fn convert_behavior<A, S>(mut behaviors: Vec<Behavior<A>>) -> Vec<Child<S>>
    where
        A: ToAction<S> + 'static,
        S: 'static,
    {
        let behaviors = behaviors
            .drain(..)
            .map(|b| {
                let action = Box::from(b);
                Child::new(action)
            })
            .collect::<Vec<Child<S>>>();
        behaviors
    }

    #[test]
    fn test_sequence_success() {
        let mut shared = TestShared::default();
        let mut sequence = SequenceState::new(convert_behavior(vec![Behavior::Action(
            TestActions::SuccessTimes { ticks: 1 },
        )]));
        assert_eq!(sequence.status, None);

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        assert_eq!(sequence.status, Some(Status::Success));
        matches!(sequence.state(), State::MultipleChildren(states) if states.len() == 1 && states[0] == ChildState::new(State::NoChild, Some(Status::Success)));

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_sequence_failure() {
        let mut shared = TestShared::default();
        let mut sequence = SequenceState::new(convert_behavior(vec![Behavior::Action(
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
        let mut sequence =
            SequenceState::new(convert_behavior(vec![Behavior::Action(TestActions::Run {
                times: 2,
                output: Status::Failure,
            })]));
        assert_eq!(sequence.status, None);

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        println!("State: {:?}", sequence.state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        println!("State: {:?}", sequence.state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        println!("State: {:?}", sequence.state());
    }

    #[test]
    fn test_sequence_multiple_children() {
        let mut shared = TestShared::default();
        let mut sequence = SequenceState::new(convert_behavior(vec![
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
        ]));
        assert_eq!(sequence.status, None);
        println!("State: {:?}", sequence.state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        println!("State: {:?}", sequence.state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        println!("State: {:?}", sequence.state());
    }

    #[test]
    fn test_sequence_multiple_children_early_failure() {
        let mut shared = TestShared::default();
        let mut sequence = SequenceState::new(convert_behavior(vec![
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
            Behavior::Action(TestActions::FailureTimes { ticks: 1 }),
            Behavior::Action(TestActions::SuccessTimes { ticks: 0 }), // This never executes
        ]));

        assert_eq!(sequence.status, None);
        println!("State: {:?}", sequence.state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);
        println!("State: {:?}", sequence.state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        println!("State: {:?}", sequence.state());

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
        println!("State: {:?}", sequence.state());
    }
}
