use crate::{Action, ChildState, Children, Status};

pub struct SelectState<S> {
    children: Children<S>,
    completed: bool,
}

impl<S> SelectState<S> {
    pub fn new(children: Children<S>) -> Self {
        Self {
            children,
            completed: false,
        }
    }
}

impl<S> Action<S> for SelectState<S> {
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
            Status::Failure => {
                self.children.next();
                match self.children.current_child() {
                    Some(_) => Status::Running,
                    None => {
                        self.completed = true;
                        Status::Failure
                    }
                }
            }
            Status::Success => {
                self.completed = true;
                Status::Success
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
    use crate::{
        test_behavior_interface::{TestAction, TestShared},
        Behavior, ChildStateInfo,
    };

    use super::*;

    #[test]
    fn test_select_success() {
        let mut select =
            SelectState::new(Children::from(vec![Behavior::Action(TestAction::Success)]));

        let mut shared = TestShared::default();

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
        matches!(select.child_state(), ChildState::MultipleChildren(states) if states.len() == 1 && states[0] == ChildStateInfo::from((ChildState::NoChild, Some(Status::Success))));
    }

    #[test]
    fn test_select_failure() {
        let mut select =
            SelectState::new(Children::from(vec![Behavior::Action(TestAction::Failure)]));

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_select_run_then_status() {
        let mut select = SelectState::new(Children::from(vec![Behavior::Action(
            TestAction::FailureAfter { times: 2 },
        )]));

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
        let mut select = SelectState::new(Children::from(vec![
            Behavior::Action(TestAction::Failure),
            Behavior::Action(TestAction::Failure),
        ]));

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_select_multiple_children_early_reset() {
        let mut select = SelectState::new(Children::from(vec![
            Behavior::Action(TestAction::Failure),
            Behavior::Action(TestAction::Failure),
        ]));

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
        let mut select = SelectState::new(Children::from(vec![
            Behavior::Action(TestAction::Failure),
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Failure),
        ]));

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }
}
