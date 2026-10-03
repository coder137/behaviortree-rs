use std::{cell::RefCell, collections::HashMap};

use behaviortree_tape::{
    AsyncBehaviorTree, Behavior, BehaviorActionState, BehaviorFutureHandler, BehaviorObserver,
    IntoBehaviorActionState, RootBehavior, Status,
};
use ticked_async_executor::TickedAsyncExecutor;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub enum Data<T> {
    Literal(std::rc::Rc<std::cell::RefCell<T>>),
    Blackboard(std::rc::Rc<String>),
}

#[derive(Debug, Clone)]
pub enum Action {
    Add { i1: usize, i2: usize, o: usize },
    Sub,
    Mul,
    Div,
}

pub enum ActionState {
    Add {
        i1: usize,
        i2: usize,
        o: usize,
        runner: ActionRunner,
    },
    Sub,
    Mul,
    Div,
}

impl ActionState {
    pub async fn add(i1: usize, i2: usize, o: usize, runner: ActionRunner) -> bool {
        let sum = runner.memory.run(|s| {
            let a = s[&i1];
            let b = s[&i2];
            a + b
        });

        yield_now().await;
        yield_now().await;
        // yield_now().await;

        runner.memory.run(|mut s| {
            println!("SUM: {sum}");
            let d = s.get_mut(&o).unwrap();
            *d = sum;
        });
        true
    }
}

impl IntoBehaviorActionState<ActionState, ActionRunner> for Action {
    fn to_state(self, runner: &mut ActionRunner) -> ActionState {
        match self {
            Self::Add { i1, i2, o } => ActionState::Add {
                i1,
                i2,
                o,
                runner: runner.clone(),
            },
            Self::Sub => ActionState::Sub,
            Self::Mul => ActionState::Mul,
            Self::Div => ActionState::Div,
        }
    }
}

impl BehaviorActionState for ActionState {
    fn make_future<'a, H>(&self, handler: H) -> H::Output
    where
        H: BehaviorFutureHandler<'a>,
    {
        match self {
            Self::Add { i1, i2, o, runner } => {
                handler.future(Self::add(*i1, *i2, *o, runner.clone()))
            }
            Self::Sub => handler.future(async move { true }),
            Self::Mul => handler.future(async move { true }),
            Self::Div => handler.future(async move { true }),
        }
    }

    fn reset(&self) {}
}

#[derive(Clone)]
pub struct Blackboard<T> {
    // blackboard: std::rc::Rc<std::cell::RefCell<slab::Slab<T>>>,
    blackboard: std::rc::Rc<std::cell::RefCell<HashMap<usize, T>>>,
    counter: std::rc::Rc<std::cell::Cell<usize>>,
}

impl<T> Blackboard<T> {
    pub fn new() -> Self {
        Self {
            // blackboard: std::rc::Rc::new(std::cell::RefCell::new(slab::Slab::new())),
            blackboard: std::rc::Rc::new(std::cell::RefCell::new(HashMap::default())),
            counter: std::rc::Rc::new(std::cell::Cell::new(0)),
        }
    }

    pub fn run<F, Ret>(&self, cb: F) -> Ret
    where
        F: Fn(std::cell::RefMut<'_, HashMap<usize, T>>) -> Ret,
    {
        let s = self.blackboard.borrow_mut();
        cb(s)
    }

    pub fn create(&mut self, data: T) -> usize {
        let current = self.counter.get();
        self.counter.set(current + 1);
        self.blackboard.borrow_mut().insert(current, data);
        current
    }
}

impl<T> Blackboard<T>
where
    T: Default,
{
    pub fn create_default(&mut self) -> usize {
        self.create(T::default())
    }
}

impl<T> std::fmt::Debug for Blackboard<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let b = self.blackboard.borrow();
        let v = b.iter().collect::<Vec<_>>();
        f.debug_struct("Blackboard").field("_", &v).finish()
    }
}

async fn yield_now() {
    let mut yielded = false;
    std::future::poll_fn(|cx| {
        if !yielded {
            yielded = true;
            cx.waker().wake_by_ref();
            return std::task::Poll::Pending;
        }
        return std::task::Poll::Ready(());
    })
    .await;
}

#[derive(Clone)]
pub struct ActionRunner {
    memory: Blackboard<i64>,
}

// #[global_allocator]
// static ALLOC: dhat::Alloc = dhat::Alloc;

pub struct MyObserver {
    map: RefCell<HashMap<usize, Option<Status>>>,
}

impl BehaviorObserver for MyObserver {
    fn init(&self, capacity: usize) {
        let mut m = self.map.borrow_mut();
        m.extend((0..capacity).into_iter().map(|i| (i, None)));
        println!("init: {:?}", m);
    }

    fn update(&self, id: usize, current_status: Option<Status>) {
        let mut b = self.map.borrow_mut();
        if b[&id] != current_status {
            b.insert(id, current_status);
            println!("update: {} {:?}", id, current_status);
        }
    }
}

fn main() -> Result<(), ()> {
    println!("Hello World");

    let mut memory = Blackboard::new();
    let i1 = memory.create(1);
    let i2 = memory.create(2);
    let o = memory.create_default();
    println!("Memory: {:?}", memory);

    let behavior = Behavior::Sequence(vec![
        Behavior::Action(Action::Add {
            i1: i1,
            i2: i2,
            o: o,
        }),
        Behavior::Action(Action::Add {
            i1: o,
            i2: i1,
            o: o,
        }),
        Behavior::Action(Action::Add {
            i1: o,
            i2: i2,
            o: o,
        }),
    ]);

    let mut runner = ActionRunner {
        memory: memory.clone(),
    };

    let mut executor = TickedAsyncExecutor::default();

    let (bt, _bt_controller) =
        AsyncBehaviorTree::from_behavior(RootBehavior::Loop(behavior.clone()), &mut runner);

    let flat_graph = bt.to_flat_graph();
    println!("{}", flat_graph);

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    executor
        .spawn_local("_", async move {
            {
                // let _profiler = dhat::Profiler::builder()
                //     .file_name(format!("simple_example.json"))
                //     .build();

                cancel_clone.run_until_cancelled_owned(bt).await;
                // let _stats = dhat::HeapStats::get();
                // println!("Stats: {_stats:?}");
            }
        })
        .detach();

    for _i in 0..10 {
        executor.tick(16.67, None);
    }

    cancel.cancel();
    executor.tick(16.67, None);
    assert_eq!(executor.num_tasks(), 0);
    println!("Memory: {memory:?}");
    Ok(())
}
