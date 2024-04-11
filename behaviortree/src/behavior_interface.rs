use crate::{
    behavior_nodes::{InvertState, SequenceState, WaitState},
    Behavior, State, Status,
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
    fn state(&self) -> State {
        State::NoChild
    }
}

pub trait ToAction<S> {
    fn to_action(self) -> Box<dyn Action<S>>;
}

impl<A, S> From<Behavior<A>> for Box<dyn Action<S>>
where
    A: ToAction<S> + 'static,
    S: 'static,
{
    fn from(behavior: Behavior<A>) -> Self {
        match behavior {
            Behavior::Action(action) => action.to_action(),
            Behavior::Wait(target) => Box::new(WaitState::new(target)),
            Behavior::Sequence(mut behaviors) => {
                let behaviors = behaviors
                    .drain(..)
                    .map(|b| {
                        let action = Self::from(b);
                        Child::new(action)
                    })
                    .collect::<Vec<Child<S>>>();
                Box::new(SequenceState::new(behaviors))
            }
            Behavior::Select(_) => todo!(),
            // Behavior::Select(behaviors) => Box::new(SelectState::new(behaviors)),
            Behavior::Invert(behavior) => {
                let action = Self::from(*behavior);
                Box::new(InvertState::new(Child::new(action)))
            }
        }
    }
}

/// Tracking Child action, status and state
///
/// Decorator and Control nodes need to track 1 or more children
/// This wrapper makes it easier to work with child nodes
/// Bundles
/// - Action: Boxed action trait (converted from Behavior)
/// - Status: Running child status
/// - Child State
pub struct Child<S> {
    action: Box<dyn Action<S>>,
    status: Option<Status>,
}

impl<S> Child<S> {
    pub fn new(action: Box<dyn Action<S>>) -> Self {
        let status = None;
        Self { action, status }
    }

    pub fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        let status = self.action.tick(dt, shared);
        self.status = Some(status);
        status
    }

    pub fn child_state(&self) -> (State, Option<Status>) {
        (self.action.state(), self.status)
    }

    pub fn reset(&mut self) {
        self.action.reset();
        self.status = None;
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
                    mock.expect_state().returning(|| State::NoChild);
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
                    mock.expect_state().returning(|| State::NoChild);
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
                    mock.expect_state().returning(|| State::NoChild);
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
