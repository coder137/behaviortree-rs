use crate::{
    behavior_nodes::{InvertState, WaitState},
    Behavior, Status,
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
    fn tick(&mut self, delta: f64, shared: &mut S) -> Status;

    /// Resets the current action to its initial/newly created state
    fn reset(&mut self);
}

pub trait ToAction<S> {
    fn to_action(self) -> Box<dyn Action<S>>;
}

pub trait Decorator<S> {
    fn tick(&mut self, child: &mut Child<S>, delta: f64, shared: &mut S) -> Status;

    /// Resets the current action to its initial/newly created state
    fn reset(&mut self);
}

pub trait Control<S> {
    fn tick(&mut self, children: &mut [Child<S>], delta: f64, shared: &mut S) -> Status;

    /// Resets the current action to its initial/newly created state
    fn reset(&mut self);
}

pub enum Child<S> {
    Action(Box<dyn Action<S>>, Option<Status>),
    Decorator(Box<dyn Decorator<S>>, Box<Child<S>>, Option<Status>),
    Control(Box<dyn Control<S>>, Vec<Child<S>>, Option<Status>),
}

impl<S> Child<S> {
    pub fn from_behavior<A>(behavior: Behavior<A>) -> Self
    where
        A: ToAction<S>,
    {
        match behavior {
            Behavior::Action(action) => {
                let action = action.to_action();
                Self::Action(action, None)
            }
            Behavior::Wait(target) => {
                let action = WaitState::new(target);
                Self::Action(Box::new(action), None)
            }
            Behavior::Invert(child) => {
                let child = Child::from_behavior(*child);
                Self::Decorator(Box::new(InvertState::new()), Box::new(child), None)
            }
            Behavior::Sequence(_children) => todo!(),
            Behavior::Select(_children) => todo!(),
        }
    }

    pub fn tick(&mut self, delta: f64, shared: &mut S) -> Status {
        match self {
            Child::Action(action, status) => {
                let current_status = action.tick(delta, shared);
                *status = Some(current_status);
                current_status
            }
            Child::Decorator(decorator, child, status) => {
                //
                let current_status = decorator.tick(child, delta, shared);
                *status = Some(current_status);
                current_status
            }
            Child::Control(control, child, status) => todo!(),
        }
    }

    pub fn reset(&mut self) {
        match self {
            Child::Action(action, status) => {
                action.reset();
                *status = None;
            }
            Child::Decorator(decorator, child, status) => {
                child.reset();
                decorator.reset();
                *status = None;
            }
            Child::Control(control, children, status) => {
                children.iter_mut().for_each(|child| {
                    child.reset();
                });
                control.reset();
                *status = None;
            }
        }
    }

    pub fn status(&self) -> Option<Status> {
        match self {
            Child::Action(_, status) => *status,
            Child::Decorator(_, _, status) => *status,
            Child::Control(_, _, status) => *status,
        }
    }
}

#[cfg(test)]
pub mod test_behavior_interface {
    use super::*;

    #[derive(Default)]
    pub struct TestShared;

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
