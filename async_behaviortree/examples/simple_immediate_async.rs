use std::{collections::HashMap, rc::Rc, sync::RwLock};

use async_behaviortree::{AsyncActionType, AsyncBehaviorTree, ImmediateAction};
use behaviortree_common::Behavior;
use ticked_async_executor::TickedAsyncExecutor;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone, Copy, serde::Serialize)]
enum Input<T> {
    Literal(T),
    Blackboard(&'static str),
}

#[derive(Debug, Clone, serde::Serialize)]
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

impl Into<AsyncActionType<OperationShared>> for Operation {
    fn into(self) -> AsyncActionType<OperationShared> {
        match self {
            Operation::Add(a, b, c) => {
                let action = Box::new(AddState(a, b, c));
                AsyncActionType::Immediate(action)
            }
            Operation::Subtract(a, b, c) => {
                let action = Box::new(SubState(a, b, c));
                AsyncActionType::Immediate(action)
            }
        }
    }
}

#[derive(Debug)]
struct AddState(Input<usize>, Input<usize>, Output);
impl ImmediateAction<OperationShared> for AddState {
    #[tracing::instrument(level = "trace", name = "Add::run", skip(shared), ret)]
    fn run(&mut self, _dt: f64, shared: &OperationShared) -> bool {
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

    #[tracing::instrument(level = "trace", name = "Add::reset", skip_all)]
    fn reset(&mut self, _shared: &OperationShared) {}

    fn name(&self) -> &'static str {
        "Add"
    }
}

#[derive(Debug)]
struct SubState(Input<usize>, Input<usize>, Output);
impl ImmediateAction<OperationShared> for SubState {
    #[tracing::instrument(level = "trace", name = "Sub::run", skip(shared), ret)]
    fn run(&mut self, _dt: f64, shared: &OperationShared) -> bool {
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

    #[tracing::instrument(level = "trace", name = "Sub::reset", skip_all)]
    fn reset(&mut self, _shared: &OperationShared) {}

    fn name(&self) -> &'static str {
        "Sub"
    }
}

fn main() -> Result<(), String> {
    tracing_subscriber::Registry::default()
        .with(tracing_forest::ForestLayer::default())
        .try_init()
        .map_err(|e| e.to_string())?;

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
    tracing::info!("Behavior:\n{output}");

    let operation_shared = OperationShared::default();
    let blackboard = operation_shared.blackboard.clone();

    let executor = TickedAsyncExecutor::default();
    let delta_rx = executor.tick_channel();

    let (future, controller) = AsyncBehaviorTree::new(behavior, delta_rx, operation_shared);

    executor
        .spawn_local("AsyncBehaviorTree::future", future)
        .detach();

    let state = controller.state();

    executor.tick(0.1, None);
    assert_eq!(executor.num_tasks(), 1);
    tracing::info!("State: {:?}", state);

    executor.tick(0.1, None);
    assert_eq!(executor.num_tasks(), 1);
    tracing::info!("State: {:?}", state);

    executor.tick(0.1, None);
    assert_eq!(executor.num_tasks(), 0);
    tracing::info!("State: {:?}", state);

    let blackboard = blackboard.read().unwrap();
    let sub = blackboard.get(&"sub".to_string()).unwrap();
    assert_eq!(*sub, 10);
    tracing::info!("Blackboard: {:?}", &(*blackboard));
    Ok(())
}
