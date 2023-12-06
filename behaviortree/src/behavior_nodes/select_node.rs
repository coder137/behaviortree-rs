use std::collections::VecDeque;

use crate::{Action, Behavior, Shared, Status, ToAction};

pub struct SelectState<A, S> {
    behaviors: VecDeque<Behavior<A>>,

    // state
    status: Option<Status>,

    // child state
    current_action: Box<dyn Action<S>>,
    current_action_status: Option<Status>,
}

impl<A, S> Action<S> for SelectState<A, S>
where
    A: ToAction<S> + 'static,
    S: Shared + 'static,
{
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        if let Some(status) = self.status {
            if status == Status::Success || status == Status::Failure {
                return status;
            }
        }

        let child_status = self.current_action.tick(dt, shared);
        let new_status = match child_status {
            Status::Failure => {
                // Go to next
                match self.behaviors.pop_front() {
                    Some(b) => {
                        self.current_action = Box::from(b);
                        self.current_action_status = None;
                        Status::Running
                    }
                    None => {
                        //
                        self.current_action_status = None;
                        Status::Failure
                    }
                }
            }
            _ => {
                // Success | Running
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
                // Halt and Reset the child
                // When resuming we tick the child from its good state again!
                self.current_action.halt();
                self.current_action_status = None;
            }
        }
        self.status = None;
        // Current action and index are left untouched for `resume` operation
    }
}

impl<A, S> SelectState<A, S>
where
    A: ToAction<S> + 'static,
    S: Shared + 'static,
{
    pub fn new(behaviors: Vec<Behavior<A>>) -> Self {
        assert!(!behaviors.is_empty());
        let mut behaviors = VecDeque::from(behaviors);
        let current_action = Box::from(behaviors.pop_front().unwrap());
        Self {
            behaviors,
            status: None,
            current_action,
            current_action_status: None,
        }
    }
}
