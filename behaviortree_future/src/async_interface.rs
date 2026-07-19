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

    pub fn runner_ref<Ret>(&self, cb: impl FnOnce(&R) -> Ret) -> Ret {
        let r = &self.safe_ctx_ref().runner;
        cb(r)
    }

    pub fn runner_ref_mut<Ret>(&mut self, cb: impl FnOnce(&mut R) -> Ret) -> Ret {
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

pub trait BehaviorTreeAsyncHandler<'a> {
    type Output;
    fn future(self, future: impl std::future::Future<Output = bool> + 'a) -> Self::Output;
}

pub trait ActionToActionState<AS, R>
where
    AS: AsyncBehaviorActionState<R>,
{
    fn to_state(self, runner: &mut R) -> AS;
}

pub trait AsyncBehaviorActionState<R> {
    fn make_future<'a, H>(&self, ctx: AsyncActionContext<R>, handler: H) -> H::Output
    where
        H: BehaviorTreeAsyncHandler<'a>;

    fn reset(&self, ctx: AsyncActionContext<R>);
}

pub trait BehaviorTreeObserver<AS> {
    fn action_name(action_state: &AS) -> &'static str;

    /// Ids are assigned from 0 -> capacity
    ///
    /// When init is called we have [0..=capacity] nodes have status: `None`
    fn init(&self, capacity: usize);

    fn update(&self, id: usize, status: Option<Status>);
}
