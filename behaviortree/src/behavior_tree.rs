use crate::{Behavior, Status};

pub trait ActionCallback<A> {
    /// mutable self -> shared data
    fn tick(&mut self, dt: f64, action: &mut A) -> Status;
}

#[derive(Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum State<A> {
    // Leaves
    Wait(f64, f64),
    Action(A),
    // Composites
    Sequence(SequenceState<A>),
    Select(Vec<Behavior<A>>, usize, Box<State<A>>),

    // Decorators
    Invert(Box<Behavior<A>>, Box<State<A>>),
    /// Ignores failures and returns `Success`.
    AlwaysSucceed(Box<Behavior<A>>),
    /// `If(condition, success, failure)`
    If(Box<Behavior<A>>, Box<Behavior<A>>, Box<Behavior<A>>),
}

impl<A> From<Behavior<A>> for State<A>
where
    A: Clone,
{
    fn from(behavior: Behavior<A>) -> Self {
        match behavior {
            Behavior::Wait(target_time) => Self::Wait(target_time, 0.0),
            Behavior::Action(action) => Self::Action(action),
            Behavior::Sequence(behaviors) => {
                let current_state = Box::new(State::from(behaviors[0].clone()));
                Self::Sequence(SequenceState::new(behaviors))
            }
            Behavior::Select(behaviors) => {
                let current_state = Box::new(State::from(behaviors[0].clone()));
                Self::Select(behaviors, 0, current_state)
            }
            Behavior::Invert(behavior) => {
                Self::Invert(behavior.clone(), Box::new(State::from(*behavior)))
            }
            Behavior::AlwaysSucceed(_) => todo!(),
            Behavior::If(_, _, _) => todo!(),
        }
    }
}

pub enum BehaviorTreePolicy {
    ReloadOnCompletion, // Resets/Reloads the behavior tree once it is completed
    RetainOnCompletion, // On completion, needs manual reset
}

pub struct BehaviorTree<A, C>
where
    C: ActionCallback<A>,
{
    // Original
    behavior: Behavior<A>,
    behavior_policy: BehaviorTreePolicy,
    callback: C,

    // State
    state: State<A>,
    status: Option<Status>,
}

impl<A, C> BehaviorTree<A, C>
where
    A: Clone,
    C: ActionCallback<A>,
{
    pub fn new(behavior: Behavior<A>, behavior_policy: BehaviorTreePolicy, callback: C) -> Self {
        let state = State::from(behavior.clone());
        Self {
            behavior,
            behavior_policy,
            callback,
            state,
            status: None,
        }
    }

    pub fn run_once(state: &mut State<A>, dt: f64, callback: &mut C) -> Status {
        match state {
            State::Action(action) => callback.tick(dt, action),
            State::Sequence(sequence_state) => sequence_state.tick(dt, callback),
            _ => todo!(),
        }
    }

    pub fn tick(&mut self, dt: f64) {
        let new_status = match self.status {
            Some(Status::Success) | Some(Status::Failure) => {
                match self.behavior_policy {
                    BehaviorTreePolicy::ReloadOnCompletion => {
                        self.reset();
                        let status = Self::run_once(&mut self.state, dt, &mut self.callback);
                        self.status = Some(status);
                    }
                    BehaviorTreePolicy::RetainOnCompletion => {
                        // Do nothing!
                        // `status` returns the already completed value
                    }
                }
            }
            None | Some(Status::Running) => {
                let status = Self::run_once(&mut self.state, dt, &mut self.callback);
                self.status = Some(status);
            }
        };
    }

    pub fn reset(&mut self) {
        self.status = None;
        self.state = State::from(self.behavior.clone());
    }

    pub fn status(&self) -> Option<Status> {
        self.status
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct SequenceState<A> {
    // originial
    behaviors: Vec<Behavior<A>>,
    // state
    index: usize,
    current_state: Box<State<A>>,
}

impl<A> SequenceState<A>
where
    A: Clone,
{
    fn new(behaviors: Vec<Behavior<A>>) -> Self {
        assert!(behaviors.len() != 0);
        let current_state = Box::new(State::from(behaviors[0].clone()));
        Self {
            behaviors,
            index: 0,
            current_state,
        }
    }

    fn tick<C>(&mut self, dt: f64, callback: &mut C) -> Status
    where
        C: ActionCallback<A>,
    {
        let status = self.run_once(dt, callback);
        // match status {
        //     Status::Success | Status::Failure => {
        //         self.reset();
        //     }
        //     Status::Running => {}
        // }
        status
    }

    fn run_once<C>(&mut self, dt: f64, callback: &mut C) -> Status
    where
        C: ActionCallback<A>,
    {
        let status = BehaviorTree::run_once(&mut self.current_state, dt, callback);
        match status {
            Status::Success => {
                let next_index = self.index + 1;
                match self.behaviors.get(next_index) {
                    Some(b) => {
                        self.index = next_index;
                        self.current_state = Box::new(State::from((*b).clone()));
                        Status::Running
                    }
                    None => Status::Success,
                }
            }
            Status::Failure => Status::Failure,
            Status::Running => Status::Running,
        }
    }

    // fn reset(&mut self) {
    //     self.index = 0;
    //     self.current_state = Box::new(State::from(self.behaviors[0].clone()));
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    enum Operation {
        Add,
        Subtract,
        Multiply,
        Divide,
    }

    struct OperationAction {}

    impl ActionCallback<Operation> for OperationAction {
        fn tick(&mut self, _dt: f64, action: &mut Operation) -> Status {
            println!("Action: {:?}", action);
            Status::Success
        }
    }

    #[test]
    fn test_simple_sequence_retain_policy() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(Operation::Add),
            Behavior::Action(Operation::Subtract),
            Behavior::Action(Operation::Multiply),
            Behavior::Action(Operation::Divide),
        ]);
        let mut bt = BehaviorTree::new(
            behavior,
            BehaviorTreePolicy::RetainOnCompletion,
            OperationAction {},
        );

        assert!(bt.status().is_none());

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert!(bt.status().unwrap() == Status::Running);

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert!(bt.status().unwrap() == Status::Running);

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert!(bt.status().unwrap() == Status::Running);

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert_eq!(bt.status().unwrap(), Status::Success);

        // Retains after completion
        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert_eq!(bt.status().unwrap(), Status::Success);

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert_eq!(bt.status().unwrap(), Status::Success);

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert_eq!(bt.status().unwrap(), Status::Success);

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert_eq!(bt.status().unwrap(), Status::Success);
    }

    #[test]
    fn test_simple_sequence_reload_policy() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(Operation::Add),
            Behavior::Action(Operation::Subtract),
            Behavior::Action(Operation::Multiply),
            Behavior::Action(Operation::Divide),
        ]);
        let mut bt = BehaviorTree::new(
            behavior,
            BehaviorTreePolicy::ReloadOnCompletion,
            OperationAction {},
        );

        assert!(bt.status().is_none());

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert!(bt.status().unwrap() == Status::Running);

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert!(bt.status().unwrap() == Status::Running);

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert!(bt.status().unwrap() == Status::Running);

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert_eq!(bt.status().unwrap(), Status::Success);

        // Reload after completion
        // Starts from Add again
        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert_eq!(bt.status().unwrap(), Status::Running);

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert_eq!(bt.status().unwrap(), Status::Running);

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert_eq!(bt.status().unwrap(), Status::Running);

        bt.tick(0.1);
        assert!(bt.status().is_some());
        assert_eq!(bt.status().unwrap(), Status::Success);
    }
}
