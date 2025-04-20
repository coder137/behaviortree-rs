use crate::{child::Child, Action, Status};

pub struct SequenceState<S> {
    children: Vec<Child<S>>,
    index: usize,
    completed: bool,
}

impl<S> SequenceState<S> {
    pub fn new(children: Vec<Child<S>>) -> Self {
        assert!(!children.is_empty());
        Self {
            children,
            index: 0,
            completed: false,
        }
    }
}

impl<S> Action<S> for SequenceState<S> {
    #[tracing::instrument(level = "trace", name = "Sequence", skip_all, ret)]
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        match self.completed {
            true => unreachable!(),
            false => {}
        }

        let child = &mut self.children[self.index];
        match child.tick(dt, shared) {
            Status::Success => {
                self.index += 1;
                match self.children.get(self.index) {
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

    fn reset(&mut self, shared: &mut S) {
        self.children
            .iter_mut()
            .for_each(|child| child.reset(shared));
        self.index = 0;
        self.completed = false;
    }

    fn name(&self) -> &'static str {
        "Sequence"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_behavior_interface::{TestAction, TestShared},
        Behavior, Status,
    };

    #[test]
    fn test_sequence_success() {
        let mut shared = TestShared::default();
        let mut sequence = Child::from_behavior(Behavior::Sequence(vec![Behavior::Action(
            TestAction::Success,
        )]));

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_sequence_failure() {
        let mut shared = TestShared::default();
        let mut sequence = Child::from_behavior(Behavior::Sequence(vec![Behavior::Action(
            TestAction::Failure,
        )]));

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_sequence_run_then_status() {
        let mut shared = TestShared::default();
        let mut sequence = Child::from_behavior(Behavior::Sequence(vec![Behavior::Action(
            TestAction::FailureAfter { times: 2 },
        )]));

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_sequence_multiple_children() {
        let mut shared = TestShared::default();
        let mut sequence = Child::from_behavior(Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]));

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_sequence_multiple_children_early_failure() {
        let mut shared = TestShared::default();
        let mut sequence = Child::from_behavior(Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Failure),
            Behavior::Action(TestAction::Success), // This never executes
        ]));

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }
}
