use crate::{Action, Child, State, Status};

pub struct SelectState<S> {
    children: Vec<Child<S>>,
    index: usize,

    // state
    status: Option<Status>,
}

impl<S> Action<S> for SelectState<S>
where
    S: 'static,
{
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
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
            Status::Failure => {
                self.index += 1;
                match self.children.get_mut(self.index) {
                    Some(_) => Status::Running,
                    None => Status::Failure,
                }
            }
            Status::Success => {
                self.index += 1;
                Status::Success
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

    fn state(&self) -> State {
        let child_states = self
            .children
            .iter()
            .take(self.index + 1)
            .map(|child| child.child_state())
            .collect();
        State::MultipleChildren(child_states)
    }
}

impl<S> SelectState<S>
where
    S: 'static,
{
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
    use crate::{
        convert_behaviors,
        test_behavior_interface::{TestActions, TestShared},
        Behavior,
    };

    use super::*;

    #[test]
    fn test_select_success() {
        let mut select = SelectState::new(convert_behaviors(vec![Behavior::Action(
            TestActions::SuccessTimes { ticks: 1 },
        )]));
        assert_eq!(select.status, None);

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        matches!(select.state(), State::MultipleChildren(states) if states.len() == 1 && states[0] == (State::NoChild, Some(Status::Success)));

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_select_failure() {
        let mut select = SelectState::new(convert_behaviors(vec![Behavior::Action(
            TestActions::FailureTimes { ticks: 1 },
        )]));
        assert_eq!(select.status, None);

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_select_run_then_status() {
        let mut select = SelectState::new(convert_behaviors(vec![Behavior::Action(
            TestActions::Run {
                times: 2,
                output: Status::Failure,
            },
        )]));
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
    fn test_select_multiple_children() {
        let mut select = SelectState::new(convert_behaviors(vec![
            Behavior::Action(TestActions::FailureTimes { ticks: 1 }),
            Behavior::Action(TestActions::FailureTimes { ticks: 1 }),
        ]));
        assert_eq!(select.status, None);

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_select_multiple_children_early_success() {
        let mut select = SelectState::new(convert_behaviors(vec![
            Behavior::Action(TestActions::FailureTimes { ticks: 1 }),
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
            Behavior::Action(TestActions::FailureTimes { ticks: 0 }),
        ]));
        assert_eq!(select.status, None);

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }
}
