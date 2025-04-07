use crate::{behavior_nodes::*, Behavior, Status};

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

pub struct Child<S> {
    action: Box<dyn Action<S>>,
    status: Option<Status>,
}

impl<S> Child<S> {
    pub fn new(action: Box<dyn Action<S>>) -> Self {
        Self {
            action,
            status: None,
        }
    }

    pub fn from_behavior<A>(behavior: Behavior<A>) -> Self
    where
        A: ToAction<S>,
        S: 'static,
    {
        match behavior {
            Behavior::Action(action) => {
                let action = action.to_action();
                Self::new(action)
            }
            Behavior::Wait(target) => {
                let action = WaitState::new(target);
                Self::new(Box::new(action))
            }
            Behavior::Invert(child) => {
                let child = Child::from_behavior(*child);
                let action = InvertState::new(child);
                Self::new(Box::new(action))
            }
            Behavior::Sequence(children) => {
                let children = children
                    .into_iter()
                    .map(|child| Child::from_behavior(child))
                    .collect::<Vec<_>>();
                let action = SequenceState::new(children);
                Self::new(Box::new(action))
            }
            Behavior::Select(children) => {
                let children = children
                    .into_iter()
                    .map(|child| Child::from_behavior(child))
                    .collect::<Vec<_>>();
                let action = SelectState::new(children);
                Self::new(Box::new(action))
            }
        }
    }

    pub fn tick(&mut self, delta: f64, shared: &mut S) -> Status {
        let status = self.action.tick(delta, shared);
        self.status = Some(status);
        status
    }

    pub fn reset(&mut self) {
        self.action.reset();
        self.status = None;
    }

    pub fn status(&self) -> Option<Status> {
        self.status
    }
}

#[cfg(test)]
pub mod test_behavior_interface {
    use super::*;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
        #[tracing::instrument(level = "trace", name = "GenericTestAction", skip_all, ret)]
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

    #[test]
    fn test_basic_behavior() {
        let _ignore = tracing_subscriber::Registry::default()
            .with(tracing_forest::ForestLayer::default())
            .try_init();

        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Wait(10.0),
            Behavior::Action(TestAction::Success),
            Behavior::Invert(Behavior::Action(TestAction::Failure).into()),
            Behavior::Action(TestAction::Success),
        ]);

        let mut child = Child::from_behavior(behavior);
        let mut shared = TestShared;

        loop {
            let status = child.tick(1.0, &mut shared);
            if status != Status::Running {
                break;
            }
        }
    }
}
