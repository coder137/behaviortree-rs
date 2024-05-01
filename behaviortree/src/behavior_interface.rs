use std::{cell::RefCell, rc::Rc};

use crate::{
    behavior_nodes::{InvertState, SelectState, SequenceState, WaitState},
    Behavior, ChildState, ChildStateInfo, ChildStateInfoInner, Status,
};

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait Action<S> {
    /// Ticks the action
    ///
    /// "Work" is done as long as `Status::Running` is returned by the action.
    ///
    /// `Status::Success` or `Status::Failure` indicates whether the work was a success/failure
    ///
    /// Invoking `tick` after action has return ed`Status::Success` or `Status::Failure` should
    /// return the same value without actually doing any "work"
    ///
    /// NOTE: See `BehaviorTree` implementation. User is not expected to invoke this manually
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status;

    /// Resets the current action to its initial/newly created state
    ///
    /// Decorator and Control nodes need to also reset their ticked children
    fn reset(&mut self);

    /// Decorator and Control type nodes need to know the state of its child(ren)
    /// User defined Action nodes do not need to override this function
    fn child_state(&self) -> ChildState {
        ChildState::NoChild
    }
}

pub trait ToAction<S> {
    fn to_action(self) -> Box<dyn Action<S>>;
}

pub fn convert_behaviors<A, S>(mut behaviors: Vec<Behavior<A>>) -> Vec<Child<S>>
where
    A: ToAction<S>,
    S: 'static,
{
    behaviors
        .drain(..)
        .map(Child::from)
        .collect::<Vec<Child<S>>>()
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
            Behavior::Sequence(behaviors) => {
                let children = convert_behaviors(behaviors);
                Box::new(SequenceState::new(children))
            }
            Behavior::Select(behaviors) => {
                let children = convert_behaviors(behaviors);
                Box::new(SelectState::new(children))
            }
            Behavior::Invert(behavior) => {
                //
                Box::new(InvertState::new(Self::from(*behavior)))
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
        self.children
            .iter_mut()
            .take_while(|child| child.status().is_some())
            .for_each(|child| {
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

#[cfg(test)]
pub mod test_behavior_interface {
    use super::*;

    #[derive(Default)]
    pub struct TestShared {}

    #[derive(Clone)]
    pub enum TestActions {
        /// Action returns success immediately
        SuccessTimes { ticks: usize },
        SuccessWithCb {
            ticks: usize,
            cb: fn(MockAction<TestShared>) -> MockAction<TestShared>,
        },
        /// Action returns failure immediately
        FailureTimes { ticks: usize },
        FailureWithCb {
            ticks: usize,
            cb: fn(MockAction<TestShared>) -> MockAction<TestShared>,
        },
        /// Action runs for `usize` ticks as Status::Running and returns `status` in the next tick
        ///
        /// Runs for a total of `usize + 1` ticks
        Run { times: usize, output: Status },
        RunWithCb {
            times: usize,
            output: Status,
            cb: fn(MockAction<TestShared>) -> MockAction<TestShared>,
        },
        /// Provides a user defined callback to simulate more complex scenarios
        Simulate(fn(MockAction<TestShared>) -> MockAction<TestShared>),
    }

    impl ToAction<TestShared> for TestActions {
        fn to_action(self) -> Box<dyn Action<TestShared>> {
            match self {
                TestActions::SuccessTimes { ticks } => {
                    TestActions::SuccessWithCb { ticks, cb: |m| m }.to_action()
                }
                TestActions::SuccessWithCb { ticks, cb } => {
                    let mut mock = MockAction::new();
                    mock.expect_tick()
                        .times(ticks)
                        .returning(|_, _| Status::Success);
                    mock.expect_child_state().returning(|| ChildState::NoChild);
                    mock = cb(mock);
                    Box::new(mock)
                }
                TestActions::FailureTimes { ticks } => {
                    TestActions::FailureWithCb { ticks, cb: |m| m }.to_action()
                }
                TestActions::FailureWithCb { ticks, cb } => {
                    let mut mock = MockAction::new();
                    mock.expect_tick()
                        .times(ticks)
                        .returning(|_dt, _shared| Status::Failure);
                    mock.expect_child_state().returning(|| ChildState::NoChild);
                    mock = cb(mock);
                    Box::new(mock)
                }
                TestActions::Run { times, output } => TestActions::RunWithCb {
                    times,
                    output,
                    cb: |m| m,
                }
                .to_action(),
                TestActions::RunWithCb { times, output, cb } => {
                    let mut mock = MockAction::new();
                    mock.expect_tick()
                        .times(times)
                        .returning(|_dt, _shared| Status::Running);
                    mock.expect_tick().return_once(move |_dt, _shared| output);
                    mock.expect_child_state().returning(|| ChildState::NoChild);
                    mock = cb(mock);
                    Box::new(mock)
                }
                TestActions::Simulate(cb) => {
                    let mut mock = MockAction::new();
                    mock = cb(mock);
                    Box::new(mock)
                }
            }
        }
    }
}
