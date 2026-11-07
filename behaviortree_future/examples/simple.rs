use std::{cell::RefCell, collections::HashMap};

use behaviortree_future::{
    ActionName, AsyncLocalBehaviorTree, Behavior, BehaviorActionState, BehaviorObserver,
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

impl ActionName for Action {
    fn action_name(&self) -> &'static str {
        match self {
            Action::Add { .. } => "Add",
            Action::Sub => "Sub",
            Action::Mul => "Mul",
            Action::Div => "Div",
        }
    }
}

impl IntoBehaviorActionState<ActionRunner> for Action {
    type State = ActionLocalState;
    fn to_state(self, runner: &mut ActionRunner) -> Self::State {
        match self {
            Self::Add { i1, i2, o } => ActionLocalState::Add {
                i1,
                i2,
                o,
                runner: runner.clone(),
            },
            Self::Sub => ActionLocalState::Sub,
            Self::Mul => ActionLocalState::Mul,
            Self::Div => ActionLocalState::Div,
        }
    }
}

pub enum ActionLocalState {
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

impl ActionLocalState {
    pub async fn add(i1: usize, i2: usize, o: usize, runner: ActionRunner) -> bool {
        let sum = runner.memory.run(|s| {
            let a = s[i1];
            let b = s[i2];
            a + b
        });

        yield_now().await;
        yield_now().await;

        runner.memory.run(|mut s| {
            println!("SUM: {sum}");
            s[o] = sum;
        });
        true
    }
}

impl BehaviorActionState for ActionLocalState {
    type F = reusable_box_future::ReusableLocalBoxFuture<bool>;

    fn create_future(&self) -> Self::F {
        match self {
            Self::Add { i1, i2, o, runner } => {
                Self::F::new(Self::add(*i1, *i2, *o, runner.clone()))
            }
            Self::Sub => Self::F::new(async move { true }),
            Self::Mul => Self::F::new(async move { true }),
            Self::Div => Self::F::new(async move { true }),
        }
    }

    fn reset_future(&self, future: &mut Self::F) {
        match self {
            Self::Add { i1, i2, o, runner } => future.set(Self::add(*i1, *i2, *o, runner.clone())),
            Self::Sub => future.set(async move { true }),
            Self::Mul => future.set(async move { true }),
            Self::Div => future.set(async move { true }),
        }
    }
}

#[derive(Clone)]
pub struct Blackboard<T> {
    blackboard: std::rc::Rc<std::cell::RefCell<slab::Slab<T>>>,
}

impl<T> Blackboard<T> {
    pub fn new() -> Self {
        Self {
            blackboard: std::rc::Rc::new(std::cell::RefCell::new(slab::Slab::new())),
        }
    }

    pub fn run<F, Ret>(&self, cb: F) -> Ret
    where
        F: Fn(std::cell::RefMut<'_, slab::Slab<T>>) -> Ret,
    {
        let s = self.blackboard.borrow_mut();
        cb(s)
    }

    pub fn create(&mut self, data: T) -> usize {
        self.blackboard.borrow_mut().insert(data)
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

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

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
        match b.get_mut(&id) {
            Some(stored_status) => {
                if *stored_status != current_status {
                    b.insert(id, current_status);
                    println!("update: {} {:?}", id, current_status);
                }
            }
            None => {
                panic!("ID: {} not found", id);
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
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

    let observer = MyObserver {
        map: RefCell::new(HashMap::default()),
    };
    let behavior = RootBehavior::Loop(behavior);

    let (bt, _bt_controller, _bt_state_tree) = AsyncLocalBehaviorTree::from_behavior_with_observer(
        behavior.clone(),
        &mut runner,
        observer,
    );
    println!("Observer: {:#?}", _bt_state_tree);

    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    executor
        .spawn_local("_", async move {
            {
                let _profiler = dhat::Profiler::builder()
                    .file_name(format!("simple_example.json"))
                    .build();

                cancel_clone.run_until_cancelled_owned(bt).await;
                let _stats = dhat::HeapStats::get();
                println!("Stats: {_stats:?}");
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
