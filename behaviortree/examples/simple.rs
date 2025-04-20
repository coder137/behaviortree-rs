use std::{collections::HashMap, rc::Rc, sync::RwLock};

use behaviortree::{Action, Behavior, BehaviorTree, Status, ToAction};

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

// Convert Operation data to functionality
impl ToAction<OperationShared> for Operation {
    fn to_action(self) -> Box<dyn Action<OperationShared>> {
        match self {
            Operation::Add(a, b, c) => Box::new(AddState(a, b, c)),
            Operation::Subtract(a, b, c) => Box::new(SubState(a, b, c)),
        }
    }
}

struct AddState(Input<usize>, Input<usize>, Output);
impl Action<OperationShared> for AddState {
    fn tick(&mut self, _dt: f64, shared: &mut OperationShared) -> Status {
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
            return Status::Failure;
        }

        let c = a.unwrap() + b.unwrap();
        match &self.2 {
            Output::Blackboard(key) => {
                blackboard.insert(key.clone(), c);
            }
        }
        Status::Success
    }

    fn reset(&mut self, _shared: &mut OperationShared) {}

    fn name(&self) -> &'static str {
        "Add"
    }
}

struct SubState(Input<usize>, Input<usize>, Output);
impl Action<OperationShared> for SubState {
    fn tick(&mut self, _dt: f64, shared: &mut OperationShared) -> Status {
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
            return Status::Failure;
        }

        let c = a.unwrap() - b.unwrap();
        match &self.2 {
            Output::Blackboard(key) => {
                blackboard.insert(key.clone(), c);
            }
        }
        Status::Success
    }

    fn reset(&mut self, _shared: &mut OperationShared) {}

    fn name(&self) -> &'static str {
        "Sub"
    }
}

fn main() {
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
    let output = serde_json::to_string_pretty(&behavior).unwrap();
    println!("Behavior:\n{output}");

    let operation_shared = OperationShared::default();
    let blackboard = operation_shared.blackboard.clone();
    let mut bt = BehaviorTree::new(
        behavior,
        behaviortree::BehaviorTreePolicy::RetainOnCompletion,
        operation_shared,
    );

    bt.tick(0.1);
    assert_eq!(bt.status().unwrap(), Status::Running);

    bt.tick(0.1);
    assert_eq!(bt.status().unwrap(), Status::Success);

    // NOTE, Since our policy is to retain on completion, ticking the behavior tree again does nothing!
    bt.tick(0.1);
    assert_eq!(bt.status().unwrap(), Status::Success);

    // In this case we need to manually reset
    bt.reset();
    assert_eq!(bt.status(), None);

    let blackboard = blackboard.read().unwrap();
    let sub = blackboard.get(&"sub".to_string()).unwrap();
    assert_eq!(*sub, 10);
    println!("Blackboard: {:?}", &(*blackboard));
}
