use std::rc::Rc;

use crate::{ActionToActionState, AsyncBehaviorActionState, BehaviorTreeAsyncHandler, Delta};

#[derive(Debug, Clone)]
pub enum TestOperation {
    Add(u32, u32, bool, u32),
    Yield(bool),
    ConsumeDelta(bool),
}

pub enum TestOperationState {
    Add(u32, u32, bool, u32, Rc<Delta>, TestOperationRunner),
    Yield(bool),
    ConsumeDelta(bool, Rc<Delta>, TestOperationRunner),
}

impl TestOperationState {
    pub async fn this_add(
        a: u32,
        b: u32,
        retval: bool,
        times: u32,
        delta: Rc<Delta>,
        runner: TestOperationRunner,
    ) -> bool {
        for _t in 0..times {
            let d = delta.get();
            runner.set_delta(d);
            yield_now().await;
        }

        let c = a + b;
        let delta = delta.get();
        runner.set_num(c);
        runner.set_delta(delta);
        retval
    }

    pub async fn this_yield(retval: bool) -> bool {
        yield_now().await;
        retval
    }

    pub async fn this_consume_delta(
        retval: bool,
        delta: Rc<Delta>,
        runner: TestOperationRunner,
    ) -> bool {
        let d = delta.get();
        runner.set_delta(d);
        retval
    }
}

impl ActionToActionState<TestOperationState, TestOperationRunner> for TestOperation {
    fn to_state(self, delta: Rc<Delta>, runner: &mut TestOperationRunner) -> TestOperationState {
        match self {
            Self::Add(a, b, retval, times) => {
                TestOperationState::Add(a, b, retval, times, delta, runner.clone())
            }
            Self::Yield(retval) => TestOperationState::Yield(retval),
            Self::ConsumeDelta(retval) => {
                TestOperationState::ConsumeDelta(retval, delta, runner.clone())
            }
        }
    }
}

impl AsyncBehaviorActionState for TestOperationState {
    fn make_future<'a, H>(&self, handler: H) -> H::Output
    where
        H: BehaviorTreeAsyncHandler<'a>,
    {
        match self {
            Self::Add(a, b, retval, times, delta, runner) => handler.future(Self::this_add(
                *a,
                *b,
                *retval,
                *times,
                delta.clone(),
                runner.clone(),
            )),
            Self::Yield(retval) => handler.future(Self::this_yield(*retval)),
            Self::ConsumeDelta(retval, delta, runner) => handler.future(Self::this_consume_delta(
                *retval,
                delta.clone(),
                runner.clone(),
            )),
        }
    }

    fn reset(&self) {}
}

#[derive(Debug, Clone)]
pub struct TestOperationRunner {
    pub num: std::rc::Rc<std::cell::Cell<u32>>,
    pub delta: std::rc::Rc<std::cell::Cell<f64>>,
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
            delta: std::rc::Rc::new(std::cell::Cell::new(0.0)),
        }
    }

    pub fn set_delta(&self, delta: f64) {
        self.delta.replace(delta);
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
