use crate::{Action, Behavior, Shared, Status, ToAction};

impl<A, S> From<Behavior<A>> for Box<dyn Action<S>>
where
    A: ToAction<S> + Clone + 'static,
    S: Shared + 'static,
{
    fn from(behavior: Behavior<A>) -> Self {
        match behavior {
            Behavior::Action(action) => action.to_action(),
            Behavior::Sequence(behaviors) => Box::new(SequenceState::new(behaviors)),
            _ => {
                todo!()
            }
        }
    }
}

pub enum BehaviorTreePolicy {
    ReloadOnCompletion, // Resets/Reloads the behavior tree once it is completed
    RetainOnCompletion, // On completion, needs manual reset
}

pub struct BehaviorTree<A, S> {
    behavior: Behavior<A>,
    behavior_policy: BehaviorTreePolicy,

    // State
    shared: S,
    status: Option<Status>,
    action: Box<dyn Action<S>>,
}

impl<A, S> BehaviorTree<A, S>
where
    A: ToAction<S> + Clone + 'static,
    S: Shared + 'static,
{
    pub fn new(behavior: Behavior<A>, behavior_policy: BehaviorTreePolicy, shared: S) -> Self {
        let action = Box::from(behavior.clone());
        Self {
            behavior,
            behavior_policy,
            shared,
            status: None,
            action,
        }
    }

    pub fn tick(&mut self, dt: f64) {
        match self.status {
            None | Some(Status::Running) => {
                let status = self.action.tick(dt, &mut self.shared);
                self.status = Some(status);
            }
            Some(Status::Success) | Some(Status::Failure) => {
                match self.behavior_policy {
                    BehaviorTreePolicy::ReloadOnCompletion => {
                        self.reset();
                        let status = self.action.tick(dt, &mut self.shared);
                        self.status = Some(status);
                    }
                    BehaviorTreePolicy::RetainOnCompletion => {
                        // Do nothing!
                        // `status` returns the already completed value
                    }
                }
            }
        }
    }

    pub fn reset(&mut self) {
        if let Some(status) = self.status {
            if status == Status::Running {
                self.action.halt();
            }
        }
        self.status = None;
        // *, It would be cleaner for the Behaviors to just reset themselves rather than reloading the entire behavior all over again?
        // Pros (more efficient maybe? Reset is propagated down the tree resetting all the children)
        // Cons (requires another trait fn -> Action::reset())
        // For now, the action is re-constructed,
        // TODO optimize by reusing the Box
        self.action = Box::from(self.behavior.clone());
    }

    pub fn get_shared(&self) -> &S {
        &self.shared
    }

    pub fn status(&self) -> Option<Status> {
        self.status
    }
}

pub struct SequenceState<A, S> {
    // originial
    behaviors: Vec<Behavior<A>>,
    // state
    status: Option<Status>,

    // state for child actions
    index: usize,
    current_action: Box<dyn Action<S>>,
    current_action_status: Option<Status>,
}

impl<A, S> Action<S> for SequenceState<A, S>
where
    A: ToAction<S> + Clone + 'static,
    S: Shared + 'static,
{
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        // Once sequence is complete return the completed status
        if let Some(status) = self.status {
            if status == Status::Success || status == Status::Failure {
                return status;
            }
        }

        let next_status = self.current_action.tick(dt, shared);
        let new_status = match next_status {
            Status::Success => {
                let next_index = self.index + 1;
                match self.behaviors.get(next_index) {
                    Some(b) => {
                        self.index = next_index;
                        self.current_action = Box::from(b.clone());
                        self.current_action_status = None;
                        Status::Running
                    }
                    None => {
                        // current_action `cannot run`
                        // No actions left to tick, success since sequence is completed
                        self.current_action_status = None;
                        Status::Success
                    }
                }
            }
            _ => {
                // Failure | Running
                self.current_action_status = Some(next_status);
                next_status
            }
        };
        self.status = Some(new_status);
        new_status
    }

    fn halt(&mut self) {
        if let Some(status) = self.current_action_status {
            if status == Status::Running {
                self.current_action.halt();
            }
        }
        self.status = None;
    }
}

impl<A, S> SequenceState<A, S>
where
    A: ToAction<S> + Clone + 'static,
    S: Shared + 'static,
{
    pub fn new(behaviors: Vec<Behavior<A>>) -> Self {
        assert!(!behaviors.is_empty());
        let current_action = Box::from(behaviors[0].clone());
        Self {
            behaviors,
            status: None,
            index: 0,
            current_action,
            current_action_status: None,
        }
    }
}
