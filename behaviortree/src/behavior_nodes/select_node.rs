use std::rc::Rc;

use crate::{Action, Child, ChildState, ChildStateInfo, Status};

pub struct SelectState<S> {
    children: Vec<Child<S>>,
    index: usize,

    // state
    status: Option<Status>,
    state: Rc<[ChildStateInfo]>,
}

impl<S> SelectState<S> {
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

impl<S> Action<S> for SelectState<S> {
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

    fn child_state(&self) -> ChildState {
        ChildState::MultipleChildren(self.state.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

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
        matches!(select.child_state(), ChildState::MultipleChildren(states) if states.len() == 1 && states[0] == Rc::new(RefCell::new((ChildState::NoChild, Some(Status::Success)))));

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
    fn test_select_multiple_children_early_reset() {
        let mut select = SelectState::new(convert_behaviors(vec![
            Behavior::Action(TestActions::FailureWithCb {
                ticks: 2,
                cb: |mut m| {
                    m.expect_reset().times(1).returning(|| {});
                    m
                },
            }),
            Behavior::Action(TestActions::FailureTimes { ticks: 1 }),
        ]));
        assert_eq!(select.status, None);

        let mut shared = TestShared::default();

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        select.reset();

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
