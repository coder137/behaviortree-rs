use crate::BehaviorTreeReset;

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

impl<C> BehaviorTreeReset for AsyncSubtree<C>
where
    C: BehaviorTreeReset,
{
    fn reset(&mut self) {
        self.child.reset();
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
