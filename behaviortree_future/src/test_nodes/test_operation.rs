use std::{cell::RefCell, rc::Rc};

use crate::{BehaviorTreeAsyncRunner, SafeDeltaType};

#[derive(Debug, Clone)]
pub enum TestOperation {
    Add(u32, u32, bool, u32),
}

#[derive(Debug)]
pub struct TestOperationRunner {
    pub num: u32,
}

impl TestOperationRunner {
    pub fn new() -> Self {
        Self {
            num: Default::default(),
        }
    }

    pub fn set_num(&mut self, num: u32, _delta: f64) {
        self.num += num;
    }
}

impl Default for TestOperationRunner {
    fn default() -> Self {
        Self::new()
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

impl BehaviorTreeAsyncRunner<TestOperation> for Rc<RefCell<TestOperationRunner>> {
    fn create_future(
        self,
        action: TestOperation,
        delta: SafeDeltaType,
    ) -> impl std::future::Future<Output = bool> {
        let future = async move {
            match action {
                TestOperation::Add(a, b, retval, times) => {
                    for _t in 0..times {
                        yield_now().await;
                    }
                    let c = a + b;
                    let delta = delta.get();
                    self.borrow_mut().set_num(c, delta);
                    retval
                }
            }
        };
        future
    }

    fn reset(&mut self, action: &TestOperation) {
        match action {
            TestOperation::Add(_, _, _, _) => {}
        }
    }
}
