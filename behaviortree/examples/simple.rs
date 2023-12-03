use std::time::Instant;

use behaviortree::{
    Action, Behavior, BehaviorTree, Blackboard, Input, Output, Shared, Status, ToAction,
};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum Operation {
    Add(Input<usize>, Input<usize>, Output),
    Subtract(Input<usize>, Input<usize>, Output),
}

/// Shared data structure for Operations

#[derive(Default)]
struct OperationShared {
    blackboard: Blackboard,
}

impl Shared for OperationShared {
    fn get_local_blackboard(&self) -> &behaviortree::Blackboard {
        &self.blackboard
    }

    fn get_mut_local_blackboard(&mut self) -> &mut behaviortree::Blackboard {
        &mut self.blackboard
    }
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
        let a = shared.read(self.0.clone());
        let b = shared.read(self.1.clone());

        if a.is_none() || b.is_none() {
            return Status::Failure;
        }

        let c = a.unwrap() + b.unwrap();
        shared.write(self.2.clone(), c);
        Status::Success
    }
}

struct SubState(Input<usize>, Input<usize>, Output);
impl Action<OperationShared> for SubState {
    fn tick(&mut self, _dt: f64, shared: &mut OperationShared) -> Status {
        let a = shared.read(self.0.clone());
        let b = shared.read(self.1.clone());

        if a.is_none() || b.is_none() {
            return Status::Failure;
        }

        let c = a.unwrap() - b.unwrap();
        shared.write(self.2.clone(), c);
        Status::Success
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

    let mut bt = BehaviorTree::new(
        behavior,
        behaviortree::BehaviorTreePolicy::RetainOnCompletion,
        OperationShared::default(),
    );

    // Shared data can be out, we don't need to keep shared data t
    let now = Instant::now();
    bt.tick(0.1);
    assert_eq!(bt.status().unwrap(), Status::Running);
    let data: usize = bt
        .get_shared()
        .read(Input::Blackboard("add".into()))
        .unwrap();
    assert_eq!(data, 30);
    println!("Elapsed: {:?}", now.elapsed());

    bt.tick(0.1);
    assert_eq!(bt.status().unwrap(), Status::Success);
    let data: usize = bt
        .get_shared()
        .read(Input::Blackboard("sub".into()))
        .unwrap();
    assert_eq!(data, 10);
    println!("Elapsed: {:?}", now.elapsed());
}
