use crate::{Behavior, Status};

pub trait Action {
    /// Function is invoked as long as `Status::Running` is returned.
    ///
    /// No longer invoked after `Status::Success` or `Status::Failure` is returned,
    /// unless reset
    ///
    /// NOTE: See `BehaviorTree` implementation. User is not expected to invoke this manually
    fn tick(&mut self, dt: f64) -> Status;

    /// Function is only invoked when a `Status::Running` action is halted.
    fn halt(&mut self);

    // TODO, Try to remove dependency on this!
    fn status(&self) -> Option<Status>;
}

pub enum BehaviorTreePolicy {
    ReloadOnCompletion, // Resets/Reloads the behavior tree once it is completed
    RetainOnCompletion, // On completion, needs manual reset
}

pub struct BehaviorTree<A> {
    behavior: Behavior<A>,
    behavior_policy: BehaviorTreePolicy,

    // State
    status: Option<Status>,
    action: Box<dyn Action>,
}

impl<A> BehaviorTree<A>
where
    A: Action + Clone + 'static,
{
    pub fn new(behavior: Behavior<A>, behavior_policy: BehaviorTreePolicy) -> Self {
        let action = get_action_impl(behavior.clone());
        Self {
            behavior,
            behavior_policy,
            status: None,
            action,
        }
    }

    pub fn tick(&mut self, dt: f64) {
        match self.status {
            None | Some(Status::Running) => {
                let status = self.action.tick(dt);
                self.status = Some(status);
            }
            Some(Status::Success) | Some(Status::Failure) => {
                match self.behavior_policy {
                    BehaviorTreePolicy::ReloadOnCompletion => {
                        self.reset();
                        let status = self.action.tick(dt);
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
        // TODO, halt
        self.status = None;
        // ! FIXME, It would be cleaner for the Behaviors to just reset themselves rather than reloading the entire behavior all over again
        self.action = get_action_impl(self.behavior.clone());
    }

    pub fn status(&self) -> Option<Status> {
        self.status
    }
}

fn get_action_impl<A>(behavior: Behavior<A>) -> Box<dyn Action>
where
    A: Action + Clone + 'static,
{
    match behavior {
        Behavior::Action(action) => Box::new(action),
        Behavior::Sequence(behaviors) => Box::new(SequenceState::new(behaviors)),
        _ => {
            todo!()
        }
    }
}

pub struct SequenceState<A> {
    // originial
    behaviors: Vec<Behavior<A>>,
    // state
    index: usize,
    current_action: Box<dyn Action>,
    status: Option<Status>,
}

impl<A> Action for SequenceState<A>
where
    A: Action + Clone + 'static,
{
    fn tick(&mut self, dt: f64) -> Status {
        // TODO, If complete return the completed status
        let status = self.current_action.tick(dt);
        let status = match status {
            Status::Success => {
                let next_index = self.index + 1;
                match self.behaviors.get(next_index) {
                    Some(b) => {
                        self.index = next_index;
                        self.current_action = get_action_impl(b.clone());
                        Status::Running
                    }
                    None => Status::Success,
                }
            }
            Status::Failure => Status::Failure,
            Status::Running => Status::Running,
        };
        self.status = Some(status);
        status
    }

    fn halt(&mut self) {
        if let Some(status) = self.current_action.status() {
            if status == Status::Running {
                self.current_action.halt();
            }
        }
        self.status = None;
    }

    // TODO, We also need a reset

    fn status(&self) -> Option<Status> {
        self.status
    }
}

impl<A> SequenceState<A>
where
    A: Action + Clone + 'static,
{
    pub fn new(behaviors: Vec<Behavior<A>>) -> Self {
        assert!(!behaviors.is_empty());
        let current_action = get_action_impl(behaviors[0].clone());
        Self {
            behaviors,
            index: 0,
            current_action,
            status: None,
        }
    }
}
