use std::collections::VecDeque;

use crate::{Action, Behavior, ChildState, State, Status, ToAction};

pub struct SelectState<A, S> {
    behaviors: VecDeque<Behavior<A>>,

    // state
    status: Option<Status>,

    // child state
    current_action: Box<dyn Action<S>>,
    current_action_status: Option<Status>,
    child_states: Vec<ChildState>,
}

impl<A, S> Action<S> for SelectState<A, S>
where
    A: ToAction<S> + 'static,
    S: 'static,
{
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        if let Some(status) = self.status {
            if status != Status::Running {
                return status;
            }
        }

        // Tick and get child status and state
        let child_status = self.current_action.tick(dt, shared);
        let child_state = self.current_action.state();
        *self.child_states.last_mut().unwrap() = ChildState::new(child_state, Some(child_status));
        let new_status = match child_status {
            Status::Failure => {
                // Go to next
                match self.behaviors.pop_front() {
                    Some(b) => {
                        self.current_action = Box::from(b);
                        self.current_action_status = None;
                        self.child_states.push(ChildState::new(
                            self.current_action.state(),
                            self.current_action_status,
                        ));
                        Status::Running
                    }
                    None => {
                        //
                        self.current_action_status = Some(child_status);
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
                *self.child_states.last_mut().unwrap() =
                    ChildState::new(self.current_action.state(), self.current_action_status);
            }
        }
        self.status = None;
        // Current action is left untouched for `resume` operation
    }

    fn state(&self) -> State {
        State::Select(self.child_states.clone())
    }
}

impl<A, S> SelectState<A, S>
where
    A: ToAction<S> + 'static,
    S: 'static,
{
    pub fn new(behaviors: Vec<Behavior<A>>) -> Self {
        assert!(!behaviors.is_empty());
        let mut behaviors = VecDeque::from(behaviors);
        let current_action: Box<dyn Action<S>> = Box::from(behaviors.pop_front().unwrap());
        let current_action_status = None;
        let child_states = vec![ChildState::new(
            current_action.state(),
            current_action_status,
        )];
        Self {
            behaviors,
            status: None,
            current_action,
            current_action_status,
            child_states,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_behavior_interface::{TestActions, TestShared};

    use super::*;

    #[test]
    fn test_select_success() {
        let mut select = SelectState::new(vec![Behavior::Action(TestActions::Success)]);
        assert_eq!(select.status, None);

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        matches!(select.state(), State::Sequence(states) if states.len() == 1 && states[0] == ChildState::new(State::NoChild, Some(Status::Success)));

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_select_failure() {
        let mut select = SelectState::new(vec![Behavior::Action(TestActions::Failure)]);
        assert_eq!(select.status, None);

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_select_run_then_status() {
        let mut select =
            SelectState::new(vec![Behavior::Action(TestActions::Run(2, Status::Failure))]);
        assert_eq!(select.status, None);

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_select_run_then_halt() {
        let custom_action1 = TestActions::Simulate(|mut mock| {
            mock.expect_tick()
                .once()
                .returning(|_dt, _shared| Status::Running);
            mock.expect_halt().once().returning(|| {});
            mock.expect_state().returning(|| State::NoChild);
            mock
        });
        let mut select = SelectState::new(vec![Behavior::Action(custom_action1)]);
        assert_eq!(select.status, None);

        let mut shared = TestShared::default();

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        select.halt();
        assert_eq!(select.status, None);

        // * When `resuming` this current action needs to restart
        // TODO, What happens after `halt` -> `resume`
    }

    #[test]
    fn test_select_multiple_children() {
        let mut select = SelectState::new(vec![
            Behavior::Action(TestActions::Failure),
            Behavior::Action(TestActions::Failure),
        ]);
        assert_eq!(select.status, None);

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_select_multiple_children_early_success() {
        let mut select = SelectState::new(vec![
            Behavior::Action(TestActions::Failure),
            Behavior::Action(TestActions::Success),
            Behavior::Action(TestActions::Failure),
        ]);
        assert_eq!(select.status, None);

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }
}
