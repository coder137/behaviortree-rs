pub trait AsyncActionName {
    fn name(&self) -> &'static str;
}

#[async_trait::async_trait(?Send)]
pub trait AsyncBehaviorRunner<A> {
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, action: &A) -> bool;

    fn reset(&mut self, action: &A);
}

// TODO, Shift this also
#[cfg(test)]
pub mod test_async_behavior_interface {
    use super::*;

    pub const DELTA: f64 = 1000.0 / 60.0;

    #[derive(Debug, Clone, Copy)]
    pub enum TestAction {
        Success,
        Failure,
        SuccessNamed { name: &'static str },
        FailureNamed { name: &'static str },
        SuccessAfter { times: usize },
        FailureAfter { times: usize },
    }

    impl AsyncActionName for TestAction {
        fn name(&self) -> &'static str {
            match self {
                TestAction::Success => "Success",
                TestAction::Failure => "Failure",
                TestAction::SuccessNamed { name } => name,
                TestAction::FailureNamed { name } => name,
                TestAction::SuccessAfter { .. } => "SuccessAfter",
                TestAction::FailureAfter { .. } => "FailureAfter",
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct TestShared;

    #[async_trait::async_trait(?Send)]
    impl AsyncBehaviorRunner<TestAction> for TestShared {
        async fn run(
            &mut self,
            mut delta: tokio::sync::watch::Receiver<f64>,
            action: &TestAction,
        ) -> bool {
            match action {
                TestAction::Success => true,
                TestAction::Failure => false,
                TestAction::SuccessNamed { .. } => true,
                TestAction::FailureNamed { .. } => false,
                TestAction::SuccessAfter { times } => {
                    let mut current_times = *times;
                    loop {
                        let _ignore = delta.changed().await;
                        let _ignore = delta.borrow_and_update();
                        current_times -= 1;
                        if current_times == 0 {
                            break;
                        }
                    }
                    true
                }
                TestAction::FailureAfter { times } => {
                    let mut current_times = *times;
                    loop {
                        let _ignore = delta.changed().await;
                        let _ignore = delta.borrow_and_update();
                        current_times -= 1;
                        if current_times == 0 {
                            break;
                        }
                    }
                    false
                }
            }
        }

        fn reset(&mut self, _action: &TestAction) {}
    }
}
