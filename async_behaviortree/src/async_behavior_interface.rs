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

#[async_trait::async_trait(?Send)]
pub trait AsyncAction<S> {
    /// Asynchronously runs the action till completion
    ///
    /// User implementation must ensure that `run` is non-blocking.
    /// - Should `.await` internally if action has not completed.
    /// - Nodes with child(ren) internally must also ensure that only one child is run
    /// before yielding back to the executor.
    ///
    /// Once `run` has completed i.e returns `true`/`false`,
    /// clients should `reset` before `run`ning.
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, shared: &mut S) -> bool;

    /// Resets the current action to its initial/newly created state
    fn reset(&mut self, shared: &mut S);

    /// Identify your action
    fn name(&self) -> &'static str;
}

// TODO, Shift this also
#[cfg(test)]
pub mod test_async_behavior_interface {
    use crate::{async_action_type::AsyncActionType, util::yield_now};

    use super::*;

    pub const DELTA: f64 = 1000.0 / 60.0;

    #[derive(Debug, Default)]
    pub struct TestShared;

    struct GenericTestImmediateAction {
        name: &'static str,
        status: bool,
    }

    impl<S> ImmediateAction<S> for GenericTestImmediateAction {
        #[tracing::instrument(level = "trace", name = "GenericTestImmediateAction::run", fields(name=<GenericTestImmediateAction as ImmediateAction<S>>::name(self)), skip_all, ret)]
        fn run(&mut self, _delta: f64, _shared: &mut S) -> bool {
            self.status
        }

        #[tracing::instrument(level = "trace", name = "GenericTestImmediateAction::reset", fields(name=<GenericTestImmediateAction as ImmediateAction<S>>::name(self)), skip_all)]
        fn reset(&mut self, _shared: &mut S) {}

        fn name(&self) -> &'static str {
            self.name
        }
    }

    struct GenericTestAsyncAction {
        name: &'static str,
        status: bool,
        times: usize,
        elapsed: usize,
    }

    impl GenericTestAsyncAction {
        fn new(name: String, status: bool, times: usize) -> Self {
            Self {
                name: Box::new(name).leak(),
                status,
                times,
                elapsed: 0,
            }
        }
    }

    #[async_trait::async_trait(?Send)]
    impl<S> AsyncAction<S> for GenericTestAsyncAction {
        #[tracing::instrument(level = "trace", name = "GenericTestAsyncAction::run", fields(name=<GenericTestAsyncAction as AsyncAction<S>>::name(self)), skip_all, ret)]
        async fn run(
            &mut self,
            mut delta: tokio::sync::watch::Receiver<f64>,
            _shared: &mut S,
        ) -> bool {
            loop {
                let _r = delta.changed().await.unwrap();
                let _dt = *delta.borrow_and_update();
                self.elapsed += 1;
                if self.elapsed < self.times {
                    yield_now().await;
                } else {
                    break;
                }
            }
            self.status
        }

        #[tracing::instrument(level = "trace", name = "GenericTestAsyncAction::reset", fields(name=<GenericTestAsyncAction as AsyncAction<S>>::name(self)), skip_all)]
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
        SuccessNamed { name: &'static str },
        FailureNamed { name: &'static str },
        SuccessAfter { times: usize },
        FailureAfter { times: usize },
    }

    impl<S> Into<AsyncActionType<S>> for TestAction {
        fn into(self) -> AsyncActionType<S> {
            match self {
                TestAction::Success => {
                    let action = Box::new(GenericTestImmediateAction {
                        name: "Success",
                        status: true,
                    });
                    AsyncActionType::Immediate(action)
                }
                TestAction::Failure => {
                    let action = Box::new(GenericTestImmediateAction {
                        name: "Failure",
                        status: false,
                    });
                    AsyncActionType::Immediate(action)
                }
                TestAction::SuccessNamed { name } => {
                    let action = Box::new(GenericTestImmediateAction { name, status: true });
                    AsyncActionType::Immediate(action)
                }
                TestAction::FailureNamed { name } => {
                    let action = Box::new(GenericTestImmediateAction {
                        name,
                        status: false,
                    });
                    AsyncActionType::Immediate(action)
                }
                TestAction::SuccessAfter { times } => {
                    let action = Box::new(GenericTestAsyncAction::new(
                        format!("SuccessAfter{}", times),
                        true,
                        times + 1,
                    ));
                    AsyncActionType::Async(action)
                }
                TestAction::FailureAfter { times } => {
                    let action = Box::new(GenericTestAsyncAction::new(
                        format!("FailureAfter{}", times),
                        false,
                        times + 1,
                    ));
                    AsyncActionType::Async(action)
                }
            }
        }
    }
}
