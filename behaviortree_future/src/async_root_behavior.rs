use crate::{
    BehaviorActionState, BehaviorObserverRunner, BehaviorTreeReset, async_behavior::AsyncBehavior,
    behavior_nodes::AsyncLoop,
};

pub(crate) enum AsyncRootBehavior<AS, O>
where
    AS: BehaviorActionState,
{
    Once(AsyncBehavior<AS, O>),
    Loop(AsyncLoop<AsyncBehavior<AS, O>>),
}

impl<AS, O> std::future::Future for AsyncRootBehavior<AS, O>
where
    AS: BehaviorActionState + Unpin,
    AS::F: Unpin,
    O: BehaviorObserverRunner + Unpin,
{
    type Output = bool;

    #[inline(always)]
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.as_mut().get_mut() {
            AsyncRootBehavior::Once(f) => std::pin::Pin::new(f).poll(cx),
            AsyncRootBehavior::Loop(f) => std::pin::Pin::new(f).poll(cx),
        }
    }
}

impl<AS, O> BehaviorTreeReset for AsyncRootBehavior<AS, O>
where
    O: BehaviorObserverRunner + Unpin,
    AS: BehaviorActionState,
{
    #[inline(always)]
    fn reset(&mut self) {
        match self {
            AsyncRootBehavior::Once(r) => r.reset(),
            AsyncRootBehavior::Loop(r) => r.reset(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_behavior_tree_trait() {
        static_assertions::assert_impl_all!(AsyncRootBehavior<(), ()>: Send, Sync);
    }
}
