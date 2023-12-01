use behaviortree::{ActionCallback, Behavior, BehaviorTree, BehaviorTreePolicy, Status};

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
