use crate::Status;

#[derive(Clone, Copy)]
struct AsyncActionContextInner<R> {
    runner: R,
    delta: f64,
}

impl<R> AsyncActionContextInner<R> {
    fn consume_delta(&mut self) -> f64 {
        let delta = self.delta;
        self.delta = 0.0;
        delta
    }
}

pub(crate) struct AsyncActionContextOwned<R> {
    ctx: std::rc::Rc<std::cell::UnsafeCell<AsyncActionContextInner<R>>>,
}

impl<R> Clone for AsyncActionContextOwned<R> {
    fn clone(&self) -> Self {
        Self {
            ctx: self.ctx.clone(),
        }
    }
}

impl<R> AsyncActionContextOwned<R> {
    pub fn new(runner: R, delta: f64) -> Self {
        let ctx = std::rc::Rc::new(std::cell::UnsafeCell::new(AsyncActionContextInner {
            runner,
            delta,
        }));
        Self { ctx }
    }

    pub fn create_ctx(&self) -> AsyncActionContext<R> {
        let ctx = self.ctx.get();
        AsyncActionContext { ctx }
    }

    pub fn update_delta(&self, current_delta: f64) {
        let mut ctx = self.create_ctx();
        let delta = &mut ctx.safe_ctx_ref_mut().delta;
        *delta = current_delta;
    }
}

pub struct AsyncActionContext<R> {
    ctx: *mut AsyncActionContextInner<R>,
}

impl<R> Clone for AsyncActionContext<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for AsyncActionContext<R> {}

impl<R> AsyncActionContext<R> {
    pub fn peek_delta(&self) -> f64 {
        self.safe_ctx_ref().delta
    }

    pub fn consume_delta(&mut self) -> f64 {
        self.safe_ctx_ref_mut().consume_delta()
    }

    pub fn runner_ref<Ret>(&self, mut cb: impl FnMut(&R) -> Ret) -> Ret {
        let r = &self.safe_ctx_ref().runner;
        cb(r)
    }

    pub fn runner_ref_mut<Ret>(&mut self, mut cb: impl FnMut(&mut R) -> Ret) -> Ret {
        let r = &mut self.safe_ctx_ref_mut().runner;
        cb(r)
    }

    fn safe_ctx_ref(&self) -> &AsyncActionContextInner<R> {
        unsafe { &*self.ctx }
    }

    fn safe_ctx_ref_mut(&mut self) -> &mut AsyncActionContextInner<R> {
        unsafe { &mut *self.ctx }
    }
}

pub(crate) trait BehaviorTreeReset<R> {
    fn reset(&mut self, ctx: AsyncActionContext<R>);
}

pub trait BehaviorTreeAsyncAction<R> {
    fn create_future(
        &self,
        ctx: AsyncActionContext<R>,
    ) -> reusable_box_future::ReusableLocalBoxFuture<bool>;

    fn reset_future(
        &self,
        ctx: AsyncActionContext<R>,
        future: &mut reusable_box_future::ReusableLocalBoxFuture<bool>,
    );
}

pub trait BehaviorTreeObserver<A> {
    fn update(&self, id: usize, status: Option<Status>);
}

impl<A> BehaviorTreeObserver<A> for () {
    fn update(&self, _id: usize, _status: Option<Status>) {}
}
