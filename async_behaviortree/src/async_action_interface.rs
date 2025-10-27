pub trait AsyncActionName {
    fn name(&self) -> &'static str;
}

#[async_trait::async_trait(?Send)]
pub trait AsyncActionRunner<A> {
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, action: &A) -> bool;

    fn reset(&mut self, action: &A);

    async fn wait(&mut self, mut delta: tokio::sync::watch::Receiver<f64>, target: f64) -> bool {
        let mut elapsed = 0.0;
        loop {
            let _r = delta.changed().await;
            if _r.is_err() {
                // This means that the executor supplying the delta channel has shutdown
                // We must stop waiting gracefully
                break;
            }
            elapsed += *(delta.borrow_and_update());
            if elapsed >= target {
                break;
            }
            crate::util::yield_now().await;
        }
        true
    }
}

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
    pub struct TestRunner;

    #[async_trait::async_trait(?Send)]
    impl AsyncActionRunner<TestAction> for TestRunner {
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
