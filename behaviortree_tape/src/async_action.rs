use reusable_box_future::ReusableLocalBoxFuture;

use crate::{BehaviorActionState, BehaviorFutureHandler, BehaviorTreeReset};

struct CreateReusableLocalBoxFutureHandler;
impl BehaviorFutureHandler<'static> for CreateReusableLocalBoxFutureHandler {
    type Output = ReusableLocalBoxFuture<bool>;
    fn future(self, future: impl std::future::Future<Output = bool> + 'static) -> Self::Output {
        reusable_box_future::ReusableLocalBoxFuture::new(future)
    }
}

struct UpdateReusableLocalBoxFutureHandler<'a>(&'a mut ReusableLocalBoxFuture<bool>);
impl<'a> BehaviorFutureHandler<'static> for UpdateReusableLocalBoxFutureHandler<'a> {
    type Output = ();
    fn future(self, future: impl std::future::Future<Output = bool> + 'static) -> Self::Output {
        self.0.set(future);
    }
}

#[pin_project::pin_project]
pub struct AsyncAction<AS> {
    action_state: AS,
    // state
    #[pin]
    future: reusable_box_future::ReusableLocalBoxFuture<bool>,
}

impl<AS> AsyncAction<AS> {
    pub fn new(action_state: AS) -> Self
    where
        AS: BehaviorActionState,
    {
        let future = action_state.make_future(CreateReusableLocalBoxFutureHandler);
        Self {
            action_state,
            future,
        }
    }
}

impl<AS> BehaviorTreeReset for AsyncAction<AS>
where
    AS: BehaviorActionState,
{
    fn reset(&mut self) {
        self.action_state.reset();
        self.action_state
            .make_future(UpdateReusableLocalBoxFutureHandler(&mut self.future));
    }
}

impl<AS> AsyncAction<AS> {
    pub fn debug(&self) -> String
    where
        AS: std::fmt::Debug,
    {
        format!("{:?}", self.action_state)
    }
}

impl<AS> std::future::Future for AsyncAction<AS> {
    type Output = bool;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        this.future.poll(cx)
    }
}

// #[cfg(test)]
// mod tests {
//     use std::rc::Rc;

//     use crate::{
//         Behavior,
//         async_behavior_state::AsyncBehaviorState,
//         behavior_nodes::AsyncTimes,
//         test_nodes::{DhatTester, TestOperation, TestOperationRunner},
//     };

//     #[test]
//     fn test_action_with_dhat() {
//         let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

//         let mut runner = TestOperationRunner::default();
//         let delta = Rc::new(Delta::default());

//         let action = {
//             let _profiler = DhatTester::new("test_action_with_dhat_pre");
//             let behavior = Behavior::Action(TestOperation::Yield(true));
//             let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
//             action
//         };

//         executor
//             .spawn_local("_", async move {
//                 let _profiler = DhatTester::new("test_action_with_dhat_post");
//                 let status = action.await;
//                 assert!(status);
//                 DhatTester::stats(|stats| {
//                     assert_eq!(stats.total_bytes, 0);
//                 });
//             })
//             .detach();

//         executor.tick(16.67, None);
//         executor.tick(16.67, None);
//         assert_eq!(executor.num_tasks(), 0);
//     }

//     #[test]
//     fn test_action_reset_with_dhat() {
//         let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

//         let mut runner = TestOperationRunner::default();
//         let delta = Rc::new(Delta::default());

//         let action = {
//             let _profiler = DhatTester::new("test_action_reset_with_dhat_pre");
//             let behavior = Behavior::Action(TestOperation::Yield(true));
//             let action = AsyncBehaviorState::from_behavior(behavior, delta, &mut runner);
//             let action = AsyncBehaviorState::Times(AsyncTimes::new(action, 2));
//             action
//         };

//         executor
//             .spawn_local("_", async move {
//                 let _profiler = DhatTester::new("test_action_reset_with_dhat_post");
//                 let status = action.await;
//                 assert!(status);
//                 DhatTester::stats(|stats| {
//                     assert_eq!(stats.total_bytes, 0);
//                 });
//             })
//             .detach();

//         executor.tick(16.67, None);
//         executor.tick(16.67, None);

//         executor.tick(16.67, None);
//         executor.tick(16.67, None);

//         assert_eq!(executor.num_tasks(), 0);
//     }
// }
