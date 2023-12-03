use crate::{Blackboard, Input, Output, Status};

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
