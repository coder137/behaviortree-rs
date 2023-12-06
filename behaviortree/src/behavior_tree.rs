use crate::{Action, Behavior, Shared, Status, ToAction};

pub enum BehaviorTreePolicy {
    /// Resets/Reloads the behavior tree once it is completed
    ReloadOnCompletion,
    /// On completion, needs manual reset
    RetainOnCompletion,
}

pub struct BehaviorTree<A, S> {
    behavior: Behavior<A>,
    behavior_policy: BehaviorTreePolicy,

    // State
    status: Option<Status>,
    action: Box<dyn Action<S>>,
}

impl<A, S> Action<S> for BehaviorTree<A, S>
where
    A: ToAction<S> + Clone + 'static,
    S: Shared + 'static,
{
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        if let Some(status) = self.status {
            if status == Status::Success || status == Status::Failure {
                match self.behavior_policy {
                    BehaviorTreePolicy::ReloadOnCompletion => {
                        self.reset();
                        // Ticks the action below
                    }
                    BehaviorTreePolicy::RetainOnCompletion => {
                        // Do nothing!
                        // `status` returns the already completed value
                        return status;
                    }
                }
            }
        }

        let status = self.action.tick(dt, shared);
        self.status = Some(status);
        status
    }

    fn halt(&mut self) {
        if let Some(status) = self.status {
            if status == Status::Running {
                self.action.halt();
            }
        }
        self.status = None;
    }
}

impl<A, S> BehaviorTree<A, S>
where
    A: ToAction<S> + Clone + 'static,
    S: Shared + 'static,
{
    pub fn new(behavior: Behavior<A>, behavior_policy: BehaviorTreePolicy) -> Self {
        let action = Box::from(behavior.clone());
        Self {
            behavior,
            behavior_policy,
            status: None,
            action,
        }
    }

    pub fn reset(&mut self) {
        self.halt();
        // *, It would be cleaner for the Behaviors to just reset themselves rather than reloading the entire behavior all over again?
        // Pros (more efficient maybe? Reset is propagated down the tree resetting all the children)
        // Cons (requires another trait fn -> Action::reset())
        // For now, the action is re-constructed,
        // TODO optimize by reusing the Box
        self.action = Box::from(self.behavior.clone());
    }

    pub fn status(&self) -> Option<Status> {
        self.status
    }
}
