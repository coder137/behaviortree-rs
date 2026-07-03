use crate::{
    ActionToActionState, AsyncActionContext, AsyncBehaviorActionState, BehaviorTreeAsyncHandler,
};

#[derive(Debug, Clone)]
pub enum TestOperation {
    Add(u32, u32, bool, u32),
    Yield(bool),
    ConsumeDelta(bool),
}

impl TestOperation {}

impl TestOperation {
    pub async fn this_add(
        a: u32,
        b: u32,
        retval: bool,
        times: u32,
        ctx: AsyncActionContext<TestOperationRunner>,
    ) -> bool {
        for _t in 0..times {
            let delta = ctx.peek_delta();
            ctx.runner_ref(|r| {
                r.set_delta(delta);
            });
            yield_now().await;
        }

        let c = a + b;
        let delta = ctx.peek_delta();
        ctx.runner_ref(|r| {
            r.set_num(c);
            r.set_delta(delta);
        });
        retval
    }

    pub async fn this_yield(retval: bool) -> bool {
        yield_now().await;
        retval
    }

    pub async fn this_consume_delta(
        retval: bool,
        mut ctx: AsyncActionContext<TestOperationRunner>,
    ) -> bool {
        let delta = ctx.consume_delta();
        ctx.runner_ref(|r| {
            r.set_delta(delta);
        });
        retval
    }
}

impl ActionToActionState<TestOperation, TestOperationRunner> for TestOperation {
    fn to_state(self) -> TestOperation {
        self
    }
}

impl AsyncBehaviorActionState<TestOperationRunner> for TestOperation {
    fn make_future<'a, H>(
        &self,
        ctx: AsyncActionContext<TestOperationRunner>,
        handler: H,
    ) -> H::Output
    where
        H: BehaviorTreeAsyncHandler<'a>,
    {
        match *self {
            TestOperation::Add(a, b, retval, times) => {
                handler.future(Self::this_add(a, b, retval, times, ctx))
            }
            TestOperation::Yield(retval) => handler.future(Self::this_yield(retval)),
            TestOperation::ConsumeDelta(retval) => {
                handler.future(Self::this_consume_delta(retval, ctx))
            }
        }
    }

    fn reset(&self, ctx: AsyncActionContext<TestOperationRunner>) {}
}

#[derive(Debug)]
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
