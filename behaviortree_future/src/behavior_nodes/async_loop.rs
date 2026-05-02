use crate::{
    AsyncActionContext, BehaviorTreeAsyncAction, BehaviorTreeReset,
    async_behavior_state::AsyncBehaviorState,
};

pub struct AsyncLoop<A, R> {
    child: Box<AsyncBehaviorState<A, R>>,
    completed: bool,
    ctx: AsyncActionContext<R>,
}

impl<A, R> AsyncLoop<A, R> {
    pub fn new(child: AsyncBehaviorState<A, R>, ctx: AsyncActionContext<R>) -> Self {
        Self {
            child: Box::new(child),
            completed: false,
            ctx,
        }
    }
}

impl<A, R> BehaviorTreeReset<R> for AsyncLoop<A, R>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.completed = false;
        self.child.reset(ctx);
    }
}

impl<A, R> std::future::Future for AsyncLoop<A, R>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    type Output = bool;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();
        if bt.completed {
            bt.completed = false;
            bt.child.reset(bt.ctx);
        }

        let child = std::pin::Pin::new(&mut bt.child);
        match child.poll(cx) {
            std::task::Poll::Ready(_s) => {
                bt.completed = true;
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
