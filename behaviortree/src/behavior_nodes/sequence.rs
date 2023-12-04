use crate::{Action, Behavior, Shared, Status, ToAction};

pub struct SequenceState<A, S> {
    // originial
    behaviors: Vec<Behavior<A>>,
    // state
    status: Option<Status>,

    // state for child actions
    index: usize,
    current_action: Box<dyn Action<S>>,
    current_action_status: Option<Status>,
}

impl<A, S> Action<S> for SequenceState<A, S>
where
    A: ToAction<S> + Clone + 'static,
    S: Shared + 'static,
{
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        // Once sequence is complete return the completed status
        if let Some(status) = self.status {
            if status == Status::Success || status == Status::Failure {
                return status;
            }
        }

        let child_status = self.current_action.tick(dt, shared);
        let new_status = match child_status {
            Status::Success => {
                let next_index = self.index + 1;
                match self.behaviors.get(next_index) {
                    Some(b) => {
                        self.index = next_index;
                        self.current_action = Box::from(b.clone());
                        self.current_action_status = None;
                        Status::Running
                    }
                    None => {
                        // current_action `cannot run`
                        // No actions left to tick, success since sequence is completed
                        self.current_action_status = None;
                        Status::Success
                    }
                }
            }
            _ => {
                // Failure | Running
                self.current_action_status = Some(child_status);
                child_status
            }
        };
        self.status = Some(new_status);
        new_status
    }

    fn halt(&mut self) {
        if let Some(status) = self.current_action_status {
            if status == Status::Running {
                self.current_action.halt();
            }
        }
    }
}

impl<A, S> SequenceState<A, S>
where
    A: ToAction<S> + Clone + 'static,
    S: Shared + 'static,
{
    pub fn new(behaviors: Vec<Behavior<A>>) -> Self {
        assert!(!behaviors.is_empty());
        let current_action = Box::from(behaviors[0].clone());
        Self {
            behaviors,
            status: None,
            index: 0,
            current_action,
            current_action_status: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_behavior_interface::{TestActions, TestShared},
        Action, Behavior, Status,
    };

    #[test]
    fn test_sequence_success() {
        let mut sequence = SequenceState::new(vec![Behavior::Action(TestActions::Success)]);
        assert_eq!(sequence.status, None);

        let mut shared = TestShared::default();
        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_sequence_failure() {
        let mut sequence = SequenceState::new(vec![Behavior::Action(TestActions::Failure)]);
        assert_eq!(sequence.status, None);

        let mut shared = TestShared::default();
        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_sequence_run_then_status() {
        let mut sequence =
            SequenceState::new(vec![Behavior::Action(TestActions::Run(2, Status::Failure))]);
        assert_eq!(sequence.status, None);

        let mut shared = TestShared::default();
        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = sequence.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_sequence_run_then_halt() {}
}
