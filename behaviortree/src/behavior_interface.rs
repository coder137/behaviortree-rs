use behaviortree_common::Status;

/// Modelled after the `std::future::Future` trait
pub trait SyncAction<S> {
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
    fn reset(&mut self, shared: &mut S);

    /// Identify your action
    fn name(&self) -> &'static str;
}

pub trait ToAction<S> {
    fn to_action(self) -> Box<dyn SyncAction<S>>;
}

#[cfg(test)]
pub mod test_behavior_interface {
    use super::*;

    #[derive(Default)]
    pub struct TestShared;

    struct GenericTestAction {
        name: &'static str,
        status: bool,
        times: usize,
        elapsed: usize,
    }

    impl GenericTestAction {
        fn new(name: String, status: bool, times: usize) -> Self {
            Self {
                name: Box::new(name).leak(),
                status,
                times,
                elapsed: 0,
            }
        }
    }

    impl<S> SyncAction<S> for GenericTestAction {
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

        fn reset(&mut self, _shared: &mut S) {
            self.elapsed = 0;
        }

        fn name(&self) -> &'static str {
            self.name
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
        fn to_action(self) -> Box<dyn SyncAction<TestShared>> {
            match self {
                TestAction::Success => Box::new(GenericTestAction::new("Success".into(), true, 1)),
                TestAction::Failure => Box::new(GenericTestAction::new("Failure".into(), false, 1)),
                TestAction::SuccessAfter { times } => {
                    assert!(times >= 1);
                    Box::new(GenericTestAction::new(
                        format!("SuccessAfter{}", times),
                        true,
                        times + 1,
                    ))
                }
                TestAction::FailureAfter { times } => {
                    assert!(times >= 1);
                    Box::new(GenericTestAction::new(
                        format!("FailureAfter{}", times),
                        false,
                        times + 1,
                    ))
                }
            }
        }
    }
}
