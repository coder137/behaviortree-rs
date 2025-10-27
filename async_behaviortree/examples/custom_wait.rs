use std::{
    collections::HashMap,
    rc::Rc,
    sync::RwLock,
    time::{Duration, Instant},
};

use async_behaviortree::{AsyncActionName, AsyncActionRunner, AsyncBehaviorTree};
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

#[derive(Debug, serde::Serialize)]
enum Operation {
    Add(Input<usize>, Input<usize>, Output),
    Subtract(Input<usize>, Input<usize>, Output),
}

impl AsyncActionName for Operation {
    fn name(&self) -> &'static str {
        match self {
            Operation::Add(_, _, _) => "Add",
            Operation::Subtract(_, _, _) => "Subtract",
        }
    }
}

struct CalculatorBot {
    timer: ticked_async_executor::TickedTimerFromTimerRegistration,
    blackboard: Rc<RwLock<TypedBlackboard<usize>>>,
}

impl CalculatorBot {
    pub fn add(&mut self, a: &Input<usize>, b: &Input<usize>, c: &Output) -> bool {
        let mut blackboard = self.blackboard.write().unwrap();

        let a_data = match a {
            Input::Literal(data) => Some(data),
            Input::Blackboard(key) => blackboard.get(*key),
        };

        let b_data = match b {
            Input::Literal(data) => Some(data),
            Input::Blackboard(key) => blackboard.get(*key),
        };

        if a_data.is_none() || b_data.is_none() {
            return false;
        }

        let c_data = a_data.unwrap() + b_data.unwrap();
        match c {
            Output::Blackboard(key) => {
                blackboard.insert(key.clone(), c_data);
            }
        }
        true
    }

    pub fn sub(&mut self, a: &Input<usize>, b: &Input<usize>, c: &Output) -> bool {
        let mut blackboard = self.blackboard.write().unwrap();

        let a_data = match a {
            Input::Literal(data) => Some(data),
            Input::Blackboard(key) => blackboard.get(*key),
        };

        let b_data = match b {
            Input::Literal(data) => Some(data),
            Input::Blackboard(key) => blackboard.get(*key),
        };

        if a_data.is_none() || b_data.is_none() {
            return false;
        }

        let c_data = a_data.unwrap() - b_data.unwrap();
        match c {
            Output::Blackboard(key) => {
                blackboard.insert(key.clone(), c_data);
            }
        }
        true
    }
}

#[async_trait::async_trait(?Send)]
impl AsyncActionRunner<Operation> for CalculatorBot {
    async fn run(&mut self, _delta: tokio::sync::watch::Receiver<f64>, action: &Operation) -> bool {
        match action {
            Operation::Add(a, b, c) => self.add(a, b, c),
            Operation::Subtract(a, b, c) => self.sub(a, b, c),
        }
    }

    // NOTE: TickedAsyncExecutor specific implementation for efficient waiting
    // Users can use other executor specific wait strategies (i.e tokio/smol etc)
    async fn wait(&mut self, _delta: tokio::sync::watch::Receiver<f64>, target: f64) -> bool {
        let instant = Instant::now();
        self.timer.sleep_for(target).await;
        let elapsed = instant.elapsed();
        tracing::info!("Elapsed: {:?}", elapsed);
        true
    }

    fn reset(&mut self, _action: &Operation) {}
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
        Behavior::Wait(1000.0),
        Behavior::Action(Operation::Subtract(
            Input::Blackboard("add".into()),
            Input::Literal(20),
            Output::Blackboard("sub".into()),
        )),
    ]);
    let output = serde_json::to_string_pretty(&behavior).unwrap();
    tracing::info!("Behavior:\n{output}");

    let mut executor = TickedAsyncExecutor::default();

    let bot = CalculatorBot {
        timer: executor.create_timer_from_timer_registration(),
        blackboard: Rc::default(),
    };
    let blackboard = bot.blackboard.clone();

    let delta_rx = executor.tick_channel();

    let (future, _controller) = AsyncBehaviorTree::new(behavior, false, delta_rx, bot);

    executor
        .spawn_local("AsyncBehaviorTree::future", future)
        .detach();

    loop {
        executor.tick(16.00, None);
        if executor.num_tasks() == 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(16));
    }

    let blackboard = blackboard.read().unwrap();
    let sub = blackboard.get(&"sub".to_string()).unwrap();
    assert_eq!(*sub, 10);
    tracing::info!("Blackboard: {:?}", &(*blackboard));
    Ok(())
}
