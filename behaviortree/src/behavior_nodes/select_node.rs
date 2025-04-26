use behaviortree_common::Status;

use crate::{child::Child, SyncAction};

pub struct SelectState<S> {
    children: Vec<Child<S>>,
    index: usize,
    completed: bool,
}

impl<S> SelectState<S> {
    pub fn new(children: Vec<Child<S>>) -> Self {
        assert!(!children.is_empty());
        Self {
            children,
            index: 0,
            completed: false,
        }
    }
}

impl<S> SyncAction<S> for SelectState<S> {
    #[tracing::instrument(level = "trace", name = "Select", skip_all, ret)]
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        match self.completed {
            true => unreachable!(),
            false => {}
        }

        let child = &mut self.children[self.index];
        match child.tick(dt, shared) {
            Status::Failure => {
                self.index += 1;
                match self.children.get(self.index) {
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

    fn reset(&mut self, shared: &mut S) {
        self.children
            .iter_mut()
            .for_each(|child| child.reset(shared));
        self.index = 0;
        self.completed = false;
    }

    fn name(&self) -> &'static str {
        "Select"
    }
}

#[cfg(test)]
mod tests {
    use behaviortree_common::Behavior;

    use crate::test_behavior_interface::{TestAction, TestShared};

    use super::*;

    #[test]
    fn test_select_success() {
        let select = Behavior::Select(vec![Behavior::Action(TestAction::Success)]);
        let mut select = Child::from_behavior(select);

        let mut shared = TestShared::default();

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn test_select_failure() {
        let select = Behavior::Select(vec![Behavior::Action(TestAction::Failure)]);
        let mut select = Child::from_behavior(select);

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_select_run_then_status() {
        let select = Behavior::Select(vec![Behavior::Action(TestAction::FailureAfter {
            times: 2,
        })]);
        let mut select = Child::from_behavior(select);

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
        let select = Behavior::Select(vec![
            Behavior::Action(TestAction::Failure),
            Behavior::Action(TestAction::Failure),
        ]);
        let mut select = Child::from_behavior(select);

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_select_multiple_children_early_reset() {
        let select = Behavior::Select(vec![
            Behavior::Action(TestAction::Failure),
            Behavior::Action(TestAction::Failure),
        ]);
        let mut select = Child::from_behavior(select);

        let mut shared = TestShared::default();

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        select.reset(&mut shared);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn test_select_multiple_children_early_success() {
        let select = Behavior::Select(vec![
            Behavior::Action(TestAction::Failure),
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Failure),
        ]);
        let mut select = Child::from_behavior(select);

        let mut shared = TestShared::default();
        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Running);

        let status = select.tick(0.1, &mut shared);
        assert_eq!(status, Status::Success);
    }
}
