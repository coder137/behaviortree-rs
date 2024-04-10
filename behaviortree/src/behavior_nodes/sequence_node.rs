use crate::{Action, Behavior, ChildState, State, Status, ToAction};

pub struct SequenceState<S> {
    // actions
    actions: Vec<(Box<dyn Action<S>>, Option<Status>, ChildState)>,
    index: usize,

    // state
    status: Option<Status>,
}

impl<S> Action<S> for SequenceState<S>
where
    S: 'static,
{
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        // Once sequence is complete return the completed status
        if let Some(status) = self.status {
            if status != Status::Running {
                return status;
            }
        }

        let (current_action, current_action_status, current_action_state) =
            &mut self.actions[self.index];
        let new_child_status = current_action.tick(dt, shared);
        let new_child_state = current_action.state();

        *current_action_status = Some(new_child_status);
        current_action_state.child_state = new_child_state;
        current_action_state.child_status = Some(new_child_status);

        let new_status = match new_child_status {
            Status::Success => {
                self.index += 1;
                match self.actions.get(self.index) {
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
            let (action, status, state) = &mut self.actions[i];
            action.reset();
            *status = None;
            state.child_state = action.state();
            state.child_status = *status;
        }

        self.index = 0;
        self.status = None;
    }

    fn state(&self) -> State {
        let child_states = self
            .actions
            .iter()
            .take(self.index + 1)
            .map(|(_, _, cs)| cs.clone())
            .collect::<Vec<ChildState>>();
        State::MultipleChildren(child_states)
    }
}

impl<S> SequenceState<S>
where
    S: 'static,
{
    pub fn new<A>(mut behaviors: Vec<Behavior<A>>) -> Self
    where
        A: ToAction<S> + 'static,
    {
        assert!(!behaviors.is_empty());
        let actions = behaviors
            .drain(..)
            .map(|behavior| {
                let behavior: Box<dyn Action<S>> = Box::from(behavior);
                let status = None;
                let child_state = ChildState::new(behavior.state(), status);
                (behavior, status, child_state)
            })
            .collect::<Vec<(Box<dyn Action<S>>, Option<Status>, ChildState)>>();
        let index = 0;
        Self {
            actions,
            index,
            status: None,
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
        let mut shared = TestShared::default();
        let mut sequence = SequenceState::new(vec![Behavior::Action(TestActions::SuccessTimes {
            ticks: 1,
        })]);
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
        let mut sequence = SequenceState::new(vec![Behavior::Action(TestActions::FailureTimes {
            ticks: 1,
        })]);
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
        let mut sequence = SequenceState::new(vec![Behavior::Action(TestActions::Run {
            times: 2,
            output: Status::Failure,
        })]);
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
        let mut sequence = SequenceState::new(vec![
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
        ]);
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
        let mut sequence = SequenceState::new(vec![
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
            Behavior::Action(TestActions::FailureTimes { ticks: 1 }),
            Behavior::Action(TestActions::SuccessTimes { ticks: 0 }), // This never executes
        ]);

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
