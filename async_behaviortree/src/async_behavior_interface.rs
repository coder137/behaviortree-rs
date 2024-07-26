use crate::AsyncChild;

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
    ///
    /// Decorator and Control nodes need to also reset their ticked children
    fn reset(&mut self);
}

pub trait ToAsyncAction<S> {
    fn to_async_action(self) -> Box<dyn AsyncAction<S>>;
}

#[async_trait::async_trait(?Send)]
pub trait AsyncDecorator<S> {
    async fn run(
        &mut self,
        child: &mut AsyncChild<S>,
        delta: &mut tokio::sync::watch::Receiver<f64>,
        shared: &mut S,
    ) -> bool;

    fn reset(&mut self);
}

#[async_trait::async_trait(?Send)]
pub trait AsyncControl<S> {
    async fn run(
        &mut self,
        children: &mut [AsyncChild<S>],
        delta: &mut tokio::sync::watch::Receiver<f64>,
        shared: &mut S,
    ) -> bool;

    fn reset(&mut self);
}

#[cfg(test)]
pub mod test_async_behavior_interface {
    use super::*;

    pub const DELTA: f64 = 1000.0 / 60.0;

    #[derive(Debug, Default)]
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
                    async_std::task::yield_now().await;
                } else {
                    break;
                }
            }
            self.status
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

    impl<S> ToAsyncAction<S> for TestAction {
        fn to_async_action(self) -> Box<dyn AsyncAction<S>> {
            match self {
                TestAction::Success => Box::new(GenericTestAction::new(true, 1)),
                TestAction::Failure => Box::new(GenericTestAction::new(false, 1)),
                TestAction::SuccessAfter { times } => {
                    Box::new(GenericTestAction::new(true, times + 1))
                }
                TestAction::FailureAfter { times } => {
                    Box::new(GenericTestAction::new(false, times + 1))
                }
            }
        }
    }
}
