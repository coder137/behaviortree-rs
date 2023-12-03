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

        let next_status = self.current_action.tick(dt, shared);
        let new_status = match next_status {
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
                self.current_action_status = Some(next_status);
                next_status
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
        self.status = None;
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
