use std::{collections::HashMap, rc::Rc, sync::RwLock};

use behaviortree::{ActionType, Behavior, BehaviorTree, ImmediateAction, Status};

#[derive(Debug, serde::Serialize)]
enum Input<T> {
    Literal(T),
    Blackboard(&'static str),
}

#[derive(Debug, serde::Serialize)]
enum Output {
    Blackboard(String),
}

pub type TypedBlackboard<T> = HashMap<String, T>;

/// Shared data structure for Operations
#[derive(Default)]
struct OperationShared {
    blackboard: Rc<RwLock<TypedBlackboard<usize>>>,
}

#[derive(Debug, serde::Serialize)]
enum Operation {
    Add(Input<usize>, Input<usize>, Output),
    Subtract(Input<usize>, Input<usize>, Output),
}

impl Into<ActionType<OperationShared>> for Operation {
    fn into(self) -> ActionType<OperationShared> {
        match self {
            Operation::Add(a, b, c) => {
                let action = Box::new(AddState(a, b, c));
                ActionType::Immediate(action)
            }
            Operation::Subtract(a, b, c) => {
                let action = Box::new(SubState(a, b, c));
                ActionType::Immediate(action)
            }
        }
    }
}

struct AddState(Input<usize>, Input<usize>, Output);
impl ImmediateAction<OperationShared> for AddState {
    fn run(&mut self, _dt: f64, shared: &mut OperationShared) -> bool {
        let mut blackboard = shared.blackboard.write().unwrap();

        let a = match &self.0 {
            Input::Literal(data) => Some(data),
            Input::Blackboard(key) => blackboard.get(*key),
        };

        let b = match &self.1 {
            Input::Literal(data) => Some(data),
            Input::Blackboard(key) => blackboard.get(*key),
        };

        if a.is_none() || b.is_none() {
            return false;
        }

        let c = a.unwrap() + b.unwrap();
        match &self.2 {
            Output::Blackboard(key) => {
                blackboard.insert(key.clone(), c);
            }
        }
        true
    }

    fn reset(&mut self, _shared: &mut OperationShared) {}

    fn name(&self) -> &'static str {
        "Add"
    }
}

struct SubState(Input<usize>, Input<usize>, Output);
impl ImmediateAction<OperationShared> for SubState {
    fn run(&mut self, _dt: f64, shared: &mut OperationShared) -> bool {
        let mut blackboard = shared.blackboard.write().unwrap();

        let a = match &self.0 {
            Input::Literal(data) => Some(data),
            Input::Blackboard(key) => blackboard.get(*key),
        };

        let b = match &self.1 {
            Input::Literal(data) => Some(data),
            Input::Blackboard(key) => blackboard.get(*key),
        };

        if a.is_none() || b.is_none() {
            return false;
        }

        let c = a.unwrap() - b.unwrap();
        match &self.2 {
            Output::Blackboard(key) => {
                blackboard.insert(key.clone(), c);
            }
        }
        true
    }

    fn reset(&mut self, _shared: &mut OperationShared) {}

    fn name(&self) -> &'static str {
        "Sub"
    }
}

fn main() -> Result<(), String> {
    let behavior = Behavior::Sequence(vec![
        Behavior::Action(Operation::Add(
            Input::Literal(10),
            Input::Literal(20),
            Output::Blackboard("add".into()),
        )),
        Behavior::Action(Operation::Subtract(
            Input::Blackboard("add".into()),
            Input::Literal(20),
            Output::Blackboard("sub".into()),
        )),
    ]);

    let operation_shared = OperationShared::default();
    let blackboard = operation_shared.blackboard.clone();
    let mut bt = BehaviorTree::new(behavior, false, operation_shared);

    bt.tick(0.1);
    assert_eq!(bt.status().unwrap(), Status::Running);

    bt.tick(0.1);
    assert_eq!(bt.status().unwrap(), Status::Success);

    let blackboard = blackboard.read().unwrap();
    let sub = blackboard.get(&"sub".to_string()).unwrap();
    assert_eq!(*sub, 10);
    Ok(())
}
