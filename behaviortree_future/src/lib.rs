pub use behaviortree_common::*;

mod interface;
pub use interface::*;

// Private
mod behavior_nodes;

#[cfg(test)]
mod test_impl {
    use super::*;

    #[derive(Debug)]
    pub enum TestAction {
        Success,
        Failure,
        SuccessAfter(u32),
        FailureAfter(u32),
    }

    impl BehaviorTreeActionToState<TestActionState> for TestAction {
        fn to_state(self) -> TestActionState {
            match self {
                TestAction::Success => TestActionState::Success,
                TestAction::Failure => TestActionState::Failure,
                TestAction::SuccessAfter(times) => TestActionState::SuccessAfter(times, 0),
                TestAction::FailureAfter(times) => TestActionState::FailureAfter(times, 0),
            }
        }
    }

    #[derive(Debug)]
    pub enum TestActionState {
        Success,
        Failure,
        SuccessAfter(u32, u32),
        FailureAfter(u32, u32),
    }

    pub struct TestRunner;

    impl BehaviorTreeAction<TestRunner> for TestActionState {
        fn tick(&mut self, delta: f64, runner: &mut TestRunner) -> std::task::Poll<bool> {
            match self {
                TestActionState::Success => std::task::Poll::Ready(true),
                TestActionState::Failure => std::task::Poll::Ready(false),
                TestActionState::SuccessAfter(times, elapsed) => {
                    //
                    if *elapsed == *times {
                        std::task::Poll::Ready(true)
                    } else {
                        *elapsed += 1;
                        std::task::Poll::Pending
                    }
                }
                TestActionState::FailureAfter(times, elapsed) => {
                    //
                    if *elapsed == *times {
                        std::task::Poll::Ready(false)
                    } else {
                        *elapsed += 1;
                        std::task::Poll::Pending
                    }
                }
            }
        }

        fn reset(&mut self, runner: &mut TestRunner) {
            match self {
                TestActionState::Success => {}
                TestActionState::Failure => {}
                TestActionState::SuccessAfter(needed, elapsed) => {
                    *elapsed = *needed;
                }
                TestActionState::FailureAfter(needed, elapsed) => {
                    *elapsed = *needed;
                }
            }
        }
    }
}

#[cfg(test)]
mod test_runner_impl {
    use super::*;

    #[derive(Debug, Clone)]
    pub enum TestOperation {
        Add(u32, u32, bool, u32),
    }

    impl BehaviorTreeActionToState<TestOperationState> for TestOperation {
        fn to_state(self) -> TestOperationState {
            match self {
                TestOperation::Add(a, b, retval, needed) => {
                    TestOperationState::Add(a, b, retval, needed, 0)
                }
            }
        }
    }

    #[derive(Debug)]
    pub enum TestOperationState {
        Add(u32, u32, bool, u32, u32),
    }

    #[derive(Debug)]
    pub struct TestOperationRunner {
        pub num: u32,
    }

    impl TestOperationRunner {
        pub fn set_num(&mut self, num: u32, _delta: f64) {
            self.num = num;
        }
    }

    impl BehaviorTreeAction<TestOperationRunner> for TestOperationState {
        fn tick(&mut self, delta: f64, runner: &mut TestOperationRunner) -> std::task::Poll<bool> {
            match self {
                TestOperationState::Add(a, b, retval, needed, elapsed) => {
                    if *needed == *elapsed {
                        let c = *a + *b;
                        runner.set_num(c, delta);
                        std::task::Poll::Ready(*retval)
                    } else {
                        *elapsed += 1;
                        std::task::Poll::Pending
                    }
                }
            }
        }

        fn reset(&mut self, _runner: &mut TestOperationRunner) {
            match self {
                TestOperationState::Add(_, _, _, needed, elapsed) => {
                    *elapsed = *needed;
                }
            }
        }
    }

    impl BehaviorTreeAsyncAction<TestOperationRunner> for TestOperation {
        fn create_future(
            self,
            delta: SafeDeltaType,
            mut runner: SafeRunnerType<TestOperationRunner>,
        ) -> impl std::future::Future<Output = bool> {
            async move {
                match self {
                    TestOperation::Add(a, b, retval, times) => {
                        for _t in 0..times {
                            tokio::task::yield_now().await;
                        }
                        let c = a + b;
                        let delta = delta.get();
                        runner.run(|r| {
                            r.set_num(c, delta);
                        });
                        retval
                    }
                }
            }
        }
    }
}
