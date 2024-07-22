use crate::{
    behavior_nodes::{AsyncInvertState, AsyncSelectState, AsyncSequenceState, AsyncWaitState},
    Behavior, Status,
};

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

pub struct AsyncChild<S> {
    action: Box<dyn AsyncAction<S>>,
    status: Option<Status>,
}

impl<S> AsyncChild<S> {
    pub fn from_behavior<A>(behavior: Behavior<A>) -> Self
    where
        A: ToAsyncAction<S>,
        S: 'static,
    {
        let action = match behavior {
            Behavior::Action(action) => action.to_async_action(),
            Behavior::Wait(target) => Box::new(AsyncWaitState::new(target)),
            Behavior::Invert(behavior) => {
                let child = Self::from_behavior(*behavior);
                Box::new(AsyncInvertState { child })
            }
            Behavior::Sequence(behaviors) => {
                let children = Self::from_behaviors(behaviors);
                Box::new(AsyncSequenceState { children })
            }
            Behavior::Select(behaviors) => {
                let children = Self::from_behaviors(behaviors);
                Box::new(AsyncSelectState { children })
            }
        };
        Self::from_action(action)
    }

    pub fn from_behaviors<A>(mut behaviors: Vec<Behavior<A>>) -> Vec<Self>
    where
        A: ToAsyncAction<S>,
        S: 'static,
    {
        behaviors
            .drain(..)
            .map(|behavior| Self::from_behavior(behavior))
            .collect()
    }

    pub async fn run(
        &mut self,
        delta: &mut tokio::sync::watch::Receiver<f64>,
        shared: &mut S,
    ) -> bool {
        self.status = Some(Status::Running);
        let success = self.action.run(delta, shared).await;
        let status = if success {
            Status::Success
        } else {
            Status::Failure
        };
        self.status = Some(status);
        success
    }

    pub fn reset(&mut self) {
        if self.status.is_none() {
            return;
        }
        self.action.reset();
        self.status = None;
    }

    fn from_action(action: Box<dyn AsyncAction<S>>) -> Self {
        Self {
            action,
            status: None,
        }
    }
}

#[cfg(test)]
pub mod test_async_behavior_interface {
    use super::*;

    pub const DELTA: f64 = 1000.0 / 60.0;

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
