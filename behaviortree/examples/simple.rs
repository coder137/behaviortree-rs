use std::time::Instant;

use behaviortree::{Action, Behavior, BehaviorTree, Blackboard, Input, Output, Status, ToAction};

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
        let a = self.0.read_ref(&shared.blackboard);
        let b = self.1.read_ref(&shared.blackboard);

        if a.is_none() || b.is_none() {
            return Status::Failure;
        }

        let c = a.unwrap() + b.unwrap();
        self.2.write(&mut shared.blackboard, c);
        Status::Success
    }

    fn reset(&mut self) {}
}

struct SubState(Input<usize>, Input<usize>, Output);
impl Action<OperationShared> for SubState {
    fn tick(&mut self, _dt: f64, shared: &mut OperationShared) -> Status {
        let a = self.0.read_ref(&shared.blackboard);
        let b = self.1.read_ref(&shared.blackboard);

        if a.is_none() || b.is_none() {
            return Status::Failure;
        }

        let c = a.unwrap() - b.unwrap();
        self.2.write(&mut shared.blackboard, c);
        Status::Success
    }

    fn reset(&mut self) {}
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
    );

    let mut shared = OperationShared::default();
    // Shared data can be out, we don't need to keep shared data t
    let now = Instant::now();
    bt.tick(0.1, &mut shared);
    assert_eq!(bt.status().unwrap(), Status::Running);
    let data: usize = shared.blackboard.read(&"add".into()).unwrap();
    assert_eq!(data, 30);
    println!("Elapsed: {:?}", now.elapsed());

    bt.tick(0.1, &mut shared);
    assert_eq!(bt.status().unwrap(), Status::Success);
    let data: usize = shared.blackboard.read(&"sub".into()).unwrap();
    assert_eq!(data, 10);
    println!("Elapsed: {:?}", now.elapsed());

    // NOTE, Since our policy is to retain on completion, ticking the behavior tree again does nothing!
    bt.tick(0.1, &mut shared);
    assert_eq!(bt.status().unwrap(), Status::Success);

    // In this case we need to manually reset
    bt.reset();
    assert_eq!(bt.status(), None);
}
