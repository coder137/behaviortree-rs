use std::{cell::RefCell, rc::Rc};

use crate::{
    behavior_nodes::{InvertState, SelectState, SequenceState, WaitState},
    Behavior, ChildState, ChildStateInfo, ChildStateInfoInner, Status,
};

/// Modelled after the `std::future::Future` trait
pub trait Action<S> {
    /// Ticks the action once.
    ///
    /// User implementation must ensure that calls to `tick` are non-blocking.
    /// Should return `Status::Running` if action has not completed.
    ///
    /// Can be called multiple times.
    /// Once `tick` has completed i.e returns `Status::Success`/`Status::Failure`,
    /// clients should `reset` before `tick`ing.
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status;

    /// Resets the current action to its initial/newly created state
    fn reset(&mut self);

    /// Decorator and Control type nodes need to know the state of its child(ren)
    ///
    /// User defined Action nodes do not need to override this function
    fn child_state(&self) -> ChildState {
        ChildState::NoChild
    }
}

pub trait ToAction<S> {
    fn to_action(self) -> Box<dyn Action<S>>;
}

/// Tracking Child action, status and state
///
/// Decorator and Control nodes need to track 1 or more children
/// This wrapper makes it easier to work with child nodes
/// Bundles
/// - Action: Boxed action trait (converted from Behavior)
/// - Child State
pub struct Child<S> {
    action: Box<dyn Action<S>>,
    state: ChildStateInfoInner,
}

impl<S> Child<S> {
    pub fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        let status = self.action.tick(dt, shared);
        {
            let mut b = self.state.borrow_mut();
            b.0 = self.action.child_state();
            b.1 = Some(status);
        }
        status
    }

    pub fn reset(&mut self) {
        if self.status().is_none() {
            return;
        }
        self.action.reset();
        {
            let mut b = self.state.borrow_mut();
            b.0 = self.action.child_state();
            b.1 = None;
        }
    }

    pub fn inner_state(&self) -> ChildStateInfo {
        ChildStateInfo::from(self.state.clone())
    }

    pub fn child_state(&self) -> ChildState {
        self.state.borrow().0.clone()
    }

    pub fn status(&self) -> Option<Status> {
        self.state.borrow().1
    }
}

impl<S> From<Box<dyn Action<S>>> for Child<S> {
    fn from(action: Box<dyn Action<S>>) -> Self {
        let state = Rc::new(RefCell::new((action.child_state(), None)));
        Self { action, state }
    }
}

impl<A, S> From<Behavior<A>> for Child<S>
where
    A: ToAction<S>,
    S: 'static,
{
    fn from(behavior: Behavior<A>) -> Self {
        let action = match behavior {
            Behavior::Action(action) => action.to_action(),
            Behavior::Wait(target) => Box::new(WaitState::new(target)),
            Behavior::Invert(behavior) => {
                let child = Self::from(*behavior);
                Box::new(InvertState::new(child))
            }
            Behavior::Sequence(behaviors) => {
                let children = Children::from(behaviors);
                Box::new(SequenceState::new(children))
            }
            Behavior::Select(behaviors) => {
                let children = Children::from(behaviors);
                Box::new(SelectState::new(children))
            }
        };
        Self::from(action)
    }
}

pub struct Children<S> {
    children: Vec<Child<S>>,
    index: usize,

    //
    state: Rc<[ChildStateInfo]>,
}

impl<S> Children<S> {
    pub fn current_child(&mut self) -> Option<&mut Child<S>> {
        self.children.get_mut(self.index)
    }

    pub fn next(&mut self) {
        self.index += 1;
    }

    pub fn reset(&mut self) {
        self.children.iter_mut().for_each(|child| {
            child.reset();
        });
        self.index = 0;
    }

    pub fn inner_state(&self) -> Rc<[ChildStateInfo]> {
        self.state.clone()
    }
}

impl<S> From<Vec<Child<S>>> for Children<S> {
    fn from(children: Vec<Child<S>>) -> Self {
        let state = Rc::from_iter(children.iter().map(|child| child.inner_state()));
        Self {
            children,
            index: 0,
            state,
        }
    }
}

impl<A, S> From<Vec<Behavior<A>>> for Children<S>
where
    A: ToAction<S>,
    S: 'static,
{
    fn from(mut behaviors: Vec<Behavior<A>>) -> Self {
        let children = behaviors
            .drain(..)
            .map(Child::from)
            .collect::<Vec<Child<S>>>();
        Self::from(children)
    }
}

#[cfg(test)]
pub mod test_behavior_interface {
    use super::*;

    #[derive(Default)]
    pub struct TestShared {}

    struct GenericTestAction {
        status: bool,
        times: usize,
        elapsed: usize,
    }

    impl GenericTestAction {
        fn new(status: bool, times: usize) -> Self {
            Self {
                status,
                times,
                elapsed: 0,
            }
        }
    }

    impl<S> Action<S> for GenericTestAction {
        fn tick(&mut self, _dt: f64, _shared: &mut S) -> Status {
            let mut status = if self.status {
                Status::Success
            } else {
                Status::Failure
            };
            self.elapsed += 1;
            if self.elapsed < self.times {
                status = Status::Running;
            }
            status
        }

        fn reset(&mut self) {
            self.elapsed = 0;
        }
    }

    #[derive(Clone, Copy)]
    pub enum TestAction {
        Success,
        Failure,
        SuccessAfter { times: usize },
        FailureAfter { times: usize },
    }

    impl ToAction<TestShared> for TestAction {
        fn to_action(self) -> Box<dyn Action<TestShared>> {
            match self {
                TestAction::Success => Box::new(GenericTestAction::new(true, 1)),
                TestAction::Failure => Box::new(GenericTestAction::new(false, 1)),
                TestAction::SuccessAfter { times } => {
                    assert!(times >= 1);
                    Box::new(GenericTestAction::new(true, times + 1))
                }
                TestAction::FailureAfter { times } => {
                    assert!(times >= 1);
                    Box::new(GenericTestAction::new(false, times + 1))
                }
            }
        }
    }
}
