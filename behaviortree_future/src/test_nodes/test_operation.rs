use crate::{BehaviorActionState, IntoBehaviorActionState};

#[derive(Debug, Clone, serde::Serialize)]
pub enum TestOperation {
    Add(u32, u32, bool, u32),
    Yield(bool),
}

pub enum TestOperationLocalState {
    Add(u32, u32, bool, u32, TestOperationRunner),
    Yield(bool),
}

impl TestOperationLocalState {
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

impl IntoBehaviorActionState<TestOperationRunner> for TestOperation {
    type State = TestOperationLocalState;
    fn to_state(self, runner: &mut TestOperationRunner) -> Self::State {
        match self {
            Self::Add(a, b, retval, times) => {
                TestOperationLocalState::Add(a, b, retval, times, runner.clone())
            }
            Self::Yield(retval) => TestOperationLocalState::Yield(retval),
        }
    }
}

impl BehaviorActionState for TestOperationLocalState {
    type F = reusable_box_future::ReusableLocalBoxFuture<bool>;

    fn create_future(&self) -> Self::F {
        match self {
            TestOperationLocalState::Add(a, b, retval, times, runner) => {
                Self::F::new(Self::this_add(*a, *b, *retval, *times, runner.clone()))
            }
            TestOperationLocalState::Yield(retval) => Self::F::new(Self::this_yield(*retval)),
        }
    }

    fn reset_future(&self, future: &mut Self::F) {
        match self {
            TestOperationLocalState::Add(a, b, retval, times, runner) => {
                future.set(Self::this_add(*a, *b, *retval, *times, runner.clone()))
            }
            TestOperationLocalState::Yield(retval) => future.set(Self::this_yield(*retval)),
        }
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
