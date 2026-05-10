use behaviortree_future::{
    AsyncActionContext, AsyncBehaviorTree, Behavior, BehaviorTreeAsyncAction,
};
use ticked_async_executor::TickedAsyncExecutor;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub enum Data<T> {
    Literal(std::rc::Rc<std::cell::RefCell<T>>),
    Blackboard(std::rc::Rc<String>),
}

#[derive(Clone)]
pub enum Action {
    Add { i1: usize, i2: usize, o: usize },
    Sub,
    Mul,
    Div,
}

impl Action {
    pub async fn add(
        i1: usize,
        i2: usize,
        o: usize,
        ctx: AsyncActionContext<ActionRunner>,
    ) -> bool {
        let memory = ctx.runner_ref(|r| r.memory.clone());
        let sum = memory.run(|s| {
            let a = s[i1];
            let b = s[i2];
            a + b
        });

        yield_now().await;

        memory.run(|mut s| {
            println!("SUM: {sum}");
            s[o] = sum;
        });
        true
    }
}

impl BehaviorTreeAsyncAction<ActionRunner> for Action {
    fn create_future(
        &self,
        ctx: AsyncActionContext<ActionRunner>,
    ) -> reusable_box_future::ReusableLocalBoxFuture<bool> {
        match *self {
            Action::Add { i1, i2, o } => {
                reusable_box_future::ReusableLocalBoxFuture::new(Self::add(i1, i2, o, ctx))
            }
            Action::Sub => reusable_box_future::ReusableLocalBoxFuture::new(async move { true }),
            Action::Mul => reusable_box_future::ReusableLocalBoxFuture::new(async move { true }),
            Action::Div => reusable_box_future::ReusableLocalBoxFuture::new(async move { true }),
        }
    }

    fn reset_future(
        &self,
        ctx: AsyncActionContext<ActionRunner>,
        future: &mut reusable_box_future::ReusableLocalBoxFuture<bool>,
    ) {
        match *self {
            Action::Add { i1, i2, o } => {
                future
                    .try_set(Self::add(i1, i2, o, ctx))
                    .map_err(|_| {})
                    .unwrap();
            }
            Action::Sub => {
                future.set(async move { true });
            }
            Action::Mul => {
                future.set(async move { true });
            }
            Action::Div => {
                future.set(async move { true });
            }
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

    let runner = ActionRunner {
        memory: memory.clone(),
    };

    let mut executor = TickedAsyncExecutor::default();
    let delta = executor.delta().inner();

    let (bt, bt_controller) =
        AsyncBehaviorTree::from_behavior(Behavior::Loop(behavior.into()), runner, delta.into());

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
