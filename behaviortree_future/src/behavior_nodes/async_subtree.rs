use crate::{AsyncActionContext, BehaviorTreeReset};

pub struct AsyncSubtree<C> {
    child: Box<C>,
}

impl<C> AsyncSubtree<C> {
    pub fn new(child: C) -> Self {
        Self {
            child: child.into(),
        }
    }
}

impl<C, R> BehaviorTreeReset<R> for AsyncSubtree<C>
where
    C: BehaviorTreeReset<R>,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        self.child.reset(ctx);
    }
}

impl<C> std::future::Future for AsyncSubtree<C>
where
    C: std::future::Future<Output = bool> + Unpin,
{
    type Output = C::Output;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();
        let child = std::pin::Pin::new(&mut bt.child);
        child.poll(cx)
    }
}
