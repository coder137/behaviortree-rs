use crate::{
    BehaviorTreeAction, BehaviorTreeAsyncAction, BehaviorTreeFuture, UnsafeDeltaType,
    UnsafeRunnerType,
};

pub struct Action<A> {
    action: A,
    // state
    result: Option<bool>,
}

impl<A> Action<A> {
    pub fn new(action: A) -> Self {
        Self {
            action,
            // state
            result: None,
        }
    }
}

impl<A, R> BehaviorTreeFuture<R> for Action<A>
where
    A: BehaviorTreeAction<R>,
{
    fn poll_tick(
        &mut self,
        cx: &mut std::task::Context<'_>,
        delta: f64,
        runner: &mut R,
    ) -> std::task::Poll<bool> {
        if let Some(result) = self.result {
            return std::task::Poll::Ready(result);
        }

        let status = self.action.tick(delta, runner);
        match status {
            std::task::Poll::Ready(result) => {
                self.result = Some(result);
            }
            std::task::Poll::Pending => {
                cx.waker().wake_by_ref();
            }
        }
        status
    }

    fn reset(&mut self, runner: &mut R) {
        self.action.reset(runner);
    }
}

pub struct AsyncAction<A, R> {
    action: A,
    delta: UnsafeDeltaType,
    runner: UnsafeRunnerType<R>,
    // state
    result: Option<bool>,
    future: reusable_box_future::ReusableLocalBoxFuture<bool>,
}

impl<A, R> AsyncAction<A, R>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    pub fn new(action: A) -> Self {
        let delta = std::rc::Rc::new(std::cell::Cell::default());
        let runner = std::rc::Rc::new(std::cell::Cell::default());
        let future = Self::create_future(action.clone(), delta.clone(), runner.clone());
        let future = reusable_box_future::ReusableLocalBoxFuture::new(future);
        Self {
            action,
            delta,
            runner,
            // state
            result: None,
            future,
        }
    }

    pub fn create_future(
        action: A,
        delta: UnsafeDeltaType,
        runner: UnsafeRunnerType<R>,
    ) -> impl std::future::Future<Output = bool> {
        action.create_future(delta.into(), runner.into())
    }
}

impl<A, R> BehaviorTreeFuture<R> for AsyncAction<A, R>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    fn poll_tick(
        &mut self,
        cx: &mut std::task::Context<'_>,
        delta: f64,
        runner: &mut R,
    ) -> std::task::Poll<bool> {
        if let Some(result) = self.result {
            return std::task::Poll::Ready(result);
        }
        self.delta.replace(delta);

        let runner_ptr = runner as *mut R;
        self.runner.replace(runner_ptr);
        let status = self.future.poll(cx);
        self.runner.replace(std::ptr::null_mut());
        match status {
            std::task::Poll::Ready(result) => {
                self.result = Some(result);
            }
            std::task::Poll::Pending => {}
        }
        status
    }

    fn reset(&mut self, _runner: &mut R) {
        self.result = None;
        let future =
            Self::create_future(self.action.clone(), self.delta.clone(), self.runner.clone());
        self.future.set(future);
    }
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    use crate::{
        BehaviorTreeActionToState,
        test_impl::{TestAction, TestRunner},
        test_runner_impl::{TestOperation, TestOperationRunner},
    };

    use super::*;

    #[test]
    fn test_action() {
        let _ignore = tracing_subscriber::Registry::default()
            .with(tracing_forest::ForestLayer::default())
            .try_init();

        let action = TestAction::SuccessAfter(1);
        let mut action = Action::new(action.to_state());

        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);

        const DELTA: f64 = 1000.0 / 60.0;

        let mut runner = TestRunner;
        let status = action.poll_tick(&mut cx, DELTA, &mut runner);
        assert_eq!(status, std::task::Poll::Pending);

        let status = action.poll_tick(&mut cx, DELTA, &mut runner);
        assert_eq!(status, std::task::Poll::Ready(true));
    }

    #[test]
    fn test_action_operation() {
        let _ignore = tracing_subscriber::Registry::default()
            .with(tracing_forest::ForestLayer::default())
            .try_init();

        let action = TestOperation::Add(1, 2, true, 1);
        let mut action = Action::new(action.to_state());

        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);

        const DELTA: f64 = 1000.0 / 60.0;

        let mut runner = TestOperationRunner { num: 0 };
        let status = action.poll_tick(&mut cx, DELTA, &mut runner);
        assert_eq!(status, std::task::Poll::Pending);

        let status = action.poll_tick(&mut cx, DELTA, &mut runner);
        assert_eq!(status, std::task::Poll::Ready(true));
        assert_eq!(runner.num, 3);
    }

    // TODO, Shift this global allocator elsewhere
    #[global_allocator]
    static ALLOC: dhat::Alloc = dhat::Alloc;

    #[test]
    fn test_action_operation_async() {
        let _ignore = tracing_subscriber::Registry::default()
            .with(tracing_forest::ForestLayer::default())
            .try_init();

        let mut action = {
            let _profiler = dhat::Profiler::builder()
                .file_name(format!("heap-pre-action-operation-async.json"))
                .build();
            let action = TestOperation::Add(1, 2, true, 1);
            let action = AsyncAction::<TestOperation, TestOperationRunner>::new(action);
            let stats = dhat::HeapStats::get();
            println!("Stats: {stats:?}");
            action
        };

        let _profiler = dhat::Profiler::builder()
            .file_name(format!("heap-post-action-operation-async.json"))
            .build();
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);

        let mut runner = TestOperationRunner { num: 0 };

        let status = action.poll_tick(&mut cx, 10.23, &mut runner);
        assert_eq!(status, std::task::Poll::Pending);

        let status = action.poll_tick(&mut cx, 12.34, &mut runner);
        assert_eq!(status, std::task::Poll::Ready(true));
        assert_eq!(runner.num, 3);

        action.reset(&mut runner);

        let status = action.poll_tick(&mut cx, 10.23, &mut runner);
        assert_eq!(status, std::task::Poll::Pending);

        let status = action.poll_tick(&mut cx, 12.34, &mut runner);
        assert_eq!(status, std::task::Poll::Ready(true));
        assert_eq!(runner.num, 3);

        let stats = dhat::HeapStats::get();
        println!("Stats: {stats:?}");
    }
}
