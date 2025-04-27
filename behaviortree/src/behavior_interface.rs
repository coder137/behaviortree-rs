use behaviortree_common::Status;

pub trait ImmediateAction<S> {
    /// Runs the action in a single tick
    ///
    /// Cannot return `Status::Running`
    /// true == `Status::Success`
    /// false == `Status::Failure`
    fn run(&mut self, delta: f64, shared: &mut S) -> bool;

    /// Resets the current action to its initial/newly created state
    fn reset(&mut self, shared: &mut S);

    /// Identify your action
    fn name(&self) -> &'static str;
}

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

// TODO, Shift this also
#[cfg(test)]
pub mod test_behavior_interface {
    use crate::action_type::ActionType;

    use super::*;

    #[derive(Default)]
    pub struct TestShared;

    struct GenericTestImmediateAction {
        name: &'static str,
        status: bool,
    }

    impl<S> ImmediateAction<S> for GenericTestImmediateAction {
        fn run(&mut self, _delta: f64, _shared: &mut S) -> bool {
            self.status
        }

        fn reset(&mut self, _shared: &mut S) {}

        fn name(&self) -> &'static str {
            self.name
        }
    }

    struct GenericTestSyncAction {
        name: &'static str,
        status: bool,
        times: usize,
        elapsed: usize,
    }

    impl GenericTestSyncAction {
        fn new(name: String, status: bool, times: usize) -> Self {
            Self {
                name: Box::new(name).leak(),
                status,
                times,
                elapsed: 0,
            }
        }
    }

    impl<S> SyncAction<S> for GenericTestSyncAction {
        #[tracing::instrument(level = "trace", name = "GenericTestSyncAction", skip_all, ret)]
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

    impl Into<ActionType<TestShared>> for TestAction {
        fn into(self) -> ActionType<TestShared> {
            match self {
                TestAction::Success => {
                    // let action = Box::new(GenericTestSyncAction::new("Success".into(), true, 1));
                    // ActionType::Sync()
                    let action = Box::new(GenericTestImmediateAction {
                        name: "Success",
                        status: true,
                    });
                    ActionType::Immediate(action)
                }
                TestAction::Failure => {
                    let action = Box::new(GenericTestImmediateAction {
                        name: "Failure",
                        status: false,
                    });
                    ActionType::Immediate(action)
                }
                TestAction::SuccessAfter { times } => {
                    assert!(times >= 1);
                    let action = Box::new(GenericTestSyncAction::new(
                        format!("SuccessAfter{}", times),
                        true,
                        times + 1,
                    ));
                    ActionType::Sync(action)
                }
                TestAction::FailureAfter { times } => {
                    assert!(times >= 1);
                    let action = Box::new(GenericTestSyncAction::new(
                        format!("FailureAfter{}", times),
                        false,
                        times + 1,
                    ));
                    ActionType::Sync(action)
                }
            }
        }
    }
}
