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
    async fn run(&mut self, delta: &mut tokio::sync::watch::Receiver<f64>, shared: &mut S) -> bool;

    /// Resets the current action to its initial/newly created state
    fn reset(&mut self, shared: &mut S);

    /// Identify your action
    fn name(&self) -> &'static str;
}

pub trait ToAsyncAction<S> {
    fn to_async_action(self) -> Box<dyn AsyncAction<S>>;
}

#[cfg(test)]
pub mod test_async_behavior_interface {
    use super::*;

    pub const DELTA: f64 = 1000.0 / 60.0;

    #[derive(Debug, Default)]
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

    #[async_trait::async_trait(?Send)]
    impl<S> AsyncAction<S> for GenericTestAction {
        async fn run(
            &mut self,
            delta: &mut tokio::sync::watch::Receiver<f64>,
            _shared: &mut S,
        ) -> bool {
            loop {
                let _r = delta.changed().await.unwrap();
                let _dt = *delta.borrow_and_update();
                self.elapsed += 1;
                if self.elapsed < self.times {
                    tokio::task::yield_now().await;
                } else {
                    break;
                }
            }
            self.status
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

    impl<S> ToAsyncAction<S> for TestAction {
        fn to_async_action(self) -> Box<dyn AsyncAction<S>> {
            match self {
                TestAction::Success => Box::new(GenericTestAction::new("Success".into(), true, 1)),
                TestAction::Failure => Box::new(GenericTestAction::new("Failure".into(), false, 1)),
                TestAction::SuccessAfter { times } => Box::new(GenericTestAction::new(
                    format!("SuccessAfter{}", times),
                    true,
                    times + 1,
                )),
                TestAction::FailureAfter { times } => Box::new(GenericTestAction::new(
                    format!("FailureAfter{}", times),
                    false,
                    times + 1,
                )),
            }
        }
    }
}
