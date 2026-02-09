pub trait BehaviorTreeActionToState<S> {
    fn to_state(self) -> S;
}

// TODO, Rename
pub trait BehaviorTreeFuture<R> {
    fn poll_tick(
        &mut self,
        cx: &mut std::task::Context<'_>,
        delta: f64,
        runner: &mut R,
    ) -> std::task::Poll<bool>;
    fn reset(&mut self, runner: &mut R);
}

pub trait BehaviorTreeAction<R> {
    fn tick(&mut self, delta: f64, runner: &mut R) -> std::task::Poll<bool>;
    fn reset(&mut self, runner: &mut R);
}

pub type UnsafeDeltaType = std::rc::Rc<std::cell::Cell<f64>>;
pub type UnsafeRunnerType<R> = std::rc::Rc<std::cell::Cell<*mut R>>;

pub struct SafeDeltaType(UnsafeDeltaType);

impl From<UnsafeDeltaType> for SafeDeltaType {
    fn from(value: UnsafeDeltaType) -> Self {
        Self(value)
    }
}

impl SafeDeltaType {
    pub fn get(&self) -> f64 {
        self.0.get()
    }
}

pub struct SafeRunnerType<R>(UnsafeRunnerType<R>);

impl<R> From<UnsafeRunnerType<R>> for SafeRunnerType<R> {
    fn from(value: UnsafeRunnerType<R>) -> Self {
        Self(value)
    }
}

impl<R> SafeRunnerType<R> {
    #[inline(always)]
    pub fn run<F, Ret>(&mut self, f: F) -> Ret
    where
        F: FnOnce(&mut R) -> Ret,
    {
        let ptr = self.0.get();
        if ptr.is_null() {
            panic!("FATAL: Runner accessed outside of tick context!");
        }
        let r = unsafe { &mut *ptr };
        f(r)
    }
}

pub trait BehaviorTreeAsyncAction<R> {
    fn create_future(
        self,
        delta: SafeDeltaType,
        runner: SafeRunnerType<R>,
    ) -> impl std::future::Future<Output = bool>;
}
