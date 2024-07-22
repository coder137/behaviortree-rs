use crate::{
    behavior_nodes::{AsyncInvertState, AsyncSequenceState, AsyncWaitState},
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
                let children = behaviors
                    .into_iter()
                    .map(|behavior| Self::from_behavior(behavior))
                    .collect::<Vec<_>>();
                Box::new(AsyncSequenceState { children })
            }
            Behavior::Select(_) => todo!(),
        };
        Self::from_action(action)
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
