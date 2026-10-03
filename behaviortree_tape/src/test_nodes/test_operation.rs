use crate::{
    ActionName, BehaviorActionState, BehaviorFutureHandler, BehaviorObserver,
    IntoBehaviorActionState, Status,
};

#[derive(Debug, Clone, serde::Serialize)]
pub enum TestOperation {
    Add(u32, u32, bool, u32),
    Yield(bool),
}

pub enum TestOperationState {
    Add(u32, u32, bool, u32, TestOperationRunner),
    Yield(bool),
}

impl std::fmt::Debug for TestOperationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add(arg0, arg1, arg2, arg3, _arg5) => f
                .debug_tuple("Add")
                .field(arg0)
                .field(arg1)
                .field(arg2)
                .field(arg3)
                .finish(),
            Self::Yield(arg0) => f.debug_tuple("Yield").field(arg0).finish(),
        }
    }
}

impl TestOperationState {
    pub async fn this_add(
        a: u32,
        b: u32,
        retval: bool,
        times: u32,
        runner: TestOperationRunner,
    ) -> bool {
        for _t in 0..times {
            yield_now().await;
        }

        let c = a + b;
        runner.set_num(c);
        retval
    }

    pub async fn this_yield(retval: bool) -> bool {
        yield_now().await;
        retval
    }
}

impl IntoBehaviorActionState<TestOperationState, TestOperationRunner> for TestOperation {
    fn to_state(self, runner: &mut TestOperationRunner) -> TestOperationState {
        match self {
            Self::Add(a, b, retval, times) => {
                TestOperationState::Add(a, b, retval, times, runner.clone())
            }
            Self::Yield(retval) => TestOperationState::Yield(retval),
        }
    }
}

impl BehaviorActionState for TestOperationState {
    fn make_future<'a, H>(&self, handler: H) -> H::Output
    where
        H: BehaviorFutureHandler<'a>,
    {
        match self {
            Self::Add(a, b, retval, times, runner) => {
                handler.future(Self::this_add(*a, *b, *retval, *times, runner.clone()))
            }
            Self::Yield(retval) => handler.future(Self::this_yield(*retval)),
        }
    }

    fn reset(&self) {}
}

impl ActionName for TestOperation {
    fn action_name(&self) -> &'static str {
        match self {
            TestOperation::Add(_, _, _, _) => "Add",
            TestOperation::Yield(_) => "Yield",
        }
    }
}

pub struct TestOperationObserver {}

impl BehaviorObserver for TestOperationObserver {
    fn init(&self, _capacity: usize) {}

    fn update(&self, id: usize, status: Option<Status>) {
        println!("Update: {} {:?}", id, status);
    }
}

#[derive(Debug, Clone)]
pub struct TestOperationRunner {
    pub num: std::rc::Rc<std::cell::Cell<u32>>,
}

impl Default for TestOperationRunner {
    fn default() -> Self {
        Self::new(0)
    }
}

impl TestOperationRunner {
    pub fn new(num: u32) -> Self {
        Self {
            num: std::rc::Rc::new(std::cell::Cell::new(num)),
        }
    }

    pub fn set_num(&self, num: u32) {
        let new_num = self.num.get() + num;
        self.num.replace(new_num);
    }
}

//

pub async fn yield_now() {
    let mut yielded = false;
    std::future::poll_fn(|cx| {
        if yielded {
            std::task::Poll::Ready(())
        } else {
            yielded = true;
            cx.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    })
    .await;
}
