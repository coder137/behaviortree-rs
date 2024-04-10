use crate::{
    behavior_nodes::{SequenceState, WaitState},
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

    /// Resets the current action
    ///
    /// Functionally equal to calling `Self::new(..)`
    fn reset(&mut self);

    /// Halts the current action and `status` will be reported as None
    ///
    /// Function is only invoked when action is in `Status::Running` state
    ///
    /// Ticking after halting == `resume` operation
    /// TODO, Do we need this?
    /// * It might be good to have in the case of halt and resume operations
    fn halt(&mut self) {}

    // TODO, on_start, on_end, on_halt, on_reset

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
            Behavior::Sequence(behaviors) => Box::new(SequenceState::new(behaviors)),
            Behavior::Select(_) => todo!(),
            Behavior::Invert(_) => todo!(),
            // Behavior::Select(behaviors) => Box::new(SelectState::new(behaviors)),
            // Behavior::Invert(behavior) => Box::new(InvertState::new(*behavior)),
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
