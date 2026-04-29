use crate::{AsyncActionContext, BehaviorTreeAsyncAction};

#[derive(Debug, Clone)]
pub enum TestOperation {
    Add(u32, u32, bool, u32),
}

impl TestOperation {}

impl BehaviorTreeAsyncAction<TestOperationRunner> for TestOperation {
    fn create_future(
        self,
        mut ctx: AsyncActionContext<TestOperationRunner>,
    ) -> impl std::future::Future<Output = bool> {
        async move {
            match self {
                TestOperation::Add(a, b, retval, times) => {
                    for _t in 0..times {
                        yield_now().await;
                    }
                    let c = a + b;
                    let delta = ctx.consume_delta();
                    ctx.runner_ref_mut(|r| {
                        r.set_num(c, delta);
                    });
                    retval
                }
            }
        }
    }

    fn reset(&self, _ctx: &mut AsyncActionContext<TestOperationRunner>) {}
}

#[derive(Debug)]
pub struct TestOperationRunner {
    pub num: std::rc::Rc<std::cell::Cell<u32>>,
}

impl TestOperationRunner {
    pub fn new(num: u32) -> Self {
        Self {
            num: std::rc::Rc::new(std::cell::Cell::new(num)),
        }
    }

    pub fn set_num(&mut self, num: u32, _delta: f64) {
        // self.num += num;
        let new_num = self.num.get() + num;
        self.num.replace(new_num);
    }
}

impl Default for TestOperationRunner {
    fn default() -> Self {
        Self::new(0)
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
