use behaviortree::{Action, Behavior, BehaviorTree, BehaviorTreePolicy, Status};

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Action for Operation {
    fn tick(&mut self, _dt: f64) -> Status {
        println!("Action: {:?}", self);
        match self {
            Operation::Add => {}
            Operation::Subtract => {}
            Operation::Multiply => {}
            Operation::Divide => {}
        }
        Status::Success
    }

    fn halt(&mut self) {
        match self {
            Operation::Add => {}
            Operation::Subtract => {}
            Operation::Multiply => {}
            Operation::Divide => {}
        }
        println!("{:?} has been reset", self);
    }

    fn status(&self) -> Option<Status> {
        match self {
            Operation::Add => todo!(),
            Operation::Subtract => todo!(),
            Operation::Multiply => todo!(),
            Operation::Divide => todo!(),
        }
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
    let behavior_data = serde_json::to_string_pretty(&behavior).unwrap();
    println!("Data: {}", behavior_data);

    let mut bt = BehaviorTree::new(behavior, BehaviorTreePolicy::RetainOnCompletion);

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

    // Manually reset
    bt.reset();

    bt.tick(0.1);
    assert!(bt.status().is_some());
    assert_eq!(bt.status().unwrap(), Status::Running);

    // Test
    bt.reset();

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

#[test]
fn test_simple_sequence_reload_policy() {
    let behavior = Behavior::Sequence(vec![
        Behavior::Action(Operation::Add),
        Behavior::Action(Operation::Subtract),
        Behavior::Action(Operation::Multiply),
        Behavior::Action(Operation::Divide),
    ]);
    let mut bt = BehaviorTree::new(behavior, BehaviorTreePolicy::ReloadOnCompletion);

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
