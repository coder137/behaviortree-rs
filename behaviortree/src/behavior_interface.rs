use crate::{
    behavior_nodes::{InvertState, SelectState, SequenceState, WaitState},
    Behavior, Blackboard, Input, Output, State, Status,
};

#[cfg(test)]
use mockall::automock;

pub trait Shared {
    fn read_ref<'a, T>(&'a self, input: &'a Input<T>) -> Option<&T>
    where
        T: 'static,
    {
        match input {
            Input::Literal(data) => Some(data),
            Input::Blackboard(key) => self.get_local_blackboard().read_ref(key),
        }
    }

    // TODO, Add a read_ref_mut version here!

    fn read<T>(&self, input: Input<T>) -> Option<T>
    where
        T: Clone + 'static,
    {
        self.read_ref(&input).cloned()
    }

    fn write<T>(&mut self, output: Output, data: T)
    where
        T: 'static,
    {
        match output {
            Output::Blackboard(key) => {
                self.get_mut_local_blackboard().write(key, data);
            }
        }
    }

    fn get_local_blackboard(&self) -> &Blackboard;
    fn get_mut_local_blackboard(&mut self) -> &mut Blackboard;
}

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

    /// Halts the current action and `status` will be reported as None
    ///
    /// Function is only invoked when action is in `Status::Running` state
    ///
    /// Ticking after halting == `resume` operation
    fn halt(&mut self) {}

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
            Behavior::Select(behaviors) => Box::new(SelectState::new(behaviors)),
            Behavior::Invert(behavior) => Box::new(InvertState::new(*behavior)),
        }
    }
}

#[cfg(test)]
pub mod test_behavior_interface {
    use super::*;

    #[derive(Default)]
    pub struct TestShared {
        blackboard: Blackboard,
    }

    #[derive(Clone)]
    pub enum TestActions {
        /// Action returns success immediately
        Success,
        /// Action returns failure immediately
        Failure,
        /// Action runs for `usize` ticks and returns `status` in the next tick
        ///
        /// Runs for a total of `usize + 1` ticks
        Run(usize, Status),
        /// Provides a user defined callback to simulate more complex scenarios
        Simulate(fn(MockAction<TestShared>) -> MockAction<TestShared>),
    }

    impl ToAction<TestShared> for TestActions {
        fn to_action(self) -> Box<dyn Action<TestShared>> {
            let mut mock = MockAction::new();
            match self {
                TestActions::Success => {
                    mock.expect_tick()
                        .once()
                        .returning(|_dt, _shared| Status::Success);
                    mock.expect_state().returning(|| State::NoChild);
                }
                TestActions::Failure => {
                    mock.expect_tick()
                        .once()
                        .returning(|_dt, _shared| Status::Failure);
                    mock.expect_state().returning(|| State::NoChild);
                }
                TestActions::Run(times, status) => {
                    mock.expect_tick()
                        .times(times)
                        .returning(|_dt, _shared| Status::Running);
                    mock.expect_tick().return_once(move |_dt, _shared| status);
                    mock.expect_state().returning(|| State::NoChild);
                }
                TestActions::Simulate(cb) => {
                    mock = cb(mock);
                }
            }
            Box::new(mock)
        }
    }
}
