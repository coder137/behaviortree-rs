use crate::{Blackboard, Input, Output, Status};

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
pub trait Action<S>
where
    S: Shared,
{
    /// Function is invoked as long as `Status::Running` is returned by the action.
    ///
    /// No longer invoked after `Status::Success` or `Status::Failure` is returned,
    /// unless reset
    ///
    /// NOTE: See `BehaviorTree` implementation. User is not expected to invoke this manually
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status;

    /// Function is only invoked when a `Status::Running` action is halted.
    fn halt(&mut self) {}
}

pub trait ToAction<S> {
    fn to_action(self) -> Box<dyn Action<S>>;
}

#[cfg(test)]
pub mod test_behavior_interface {
    use super::*;

    #[derive(Default)]
    pub struct TestShared {
        blackboard: Blackboard,
    }

    impl Shared for TestShared {
        fn get_local_blackboard(&self) -> &crate::Blackboard {
            &self.blackboard
        }

        fn get_mut_local_blackboard(&mut self) -> &mut crate::Blackboard {
            &mut self.blackboard
        }
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
                }
                TestActions::Failure => {
                    mock.expect_tick()
                        .once()
                        .returning(|_dt, _shared| Status::Failure);
                }
                TestActions::Run(times, status) => {
                    mock.expect_tick()
                        .times(times)
                        .returning(|_dt, _shared| Status::Running);
                    mock.expect_tick().return_once(move |_dt, _shared| status);
                }
                TestActions::Simulate(cb) => {
                    mock = cb(mock);
                }
            }
            Box::new(mock)
        }
    }
}
