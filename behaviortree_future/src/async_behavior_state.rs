use crate::{
    AsyncActionContext, Behavior, BehaviorTreeAsyncAction,
    behavior_nodes::{AsyncAction, AsyncInvert, AsyncSelect, AsyncSequence},
};

pub enum AsyncBehaviorState<A> {
    Action(AsyncAction<A>),
    Invert(AsyncInvert<A>),
    Sequence(AsyncSequence<A>),
    Select(AsyncSelect<A>),
}

impl<A> AsyncBehaviorState<A> {
    pub fn from_behavior<R>(behavior: Behavior<A>, ctx: AsyncActionContext<R>) -> Self
    where
        A: BehaviorTreeAsyncAction<R> + Clone + 'static,
        R: 'static,
    {
        match behavior {
            Behavior::Action(action) => Self::Action(AsyncAction::new(action, ctx)),
            Behavior::Invert(behavior) => {
                let child = Self::from_behavior(*behavior, ctx);
                Self::Invert(AsyncInvert::new(child))
            }
            Behavior::Sequence(behaviors) => {
                let children = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior(b, ctx))
                    .collect::<Vec<_>>();
                Self::Sequence(AsyncSequence::new(children))
            }
            Behavior::Select(behaviors) => {
                let children = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior(b, ctx))
                    .collect::<Vec<_>>();
                Self::Select(AsyncSelect::new(children))
            }
        }
    }

    pub fn reset<R>(&mut self, ctx: AsyncActionContext<R>)
    where
        A: BehaviorTreeAsyncAction<R> + Clone + 'static,
        R: 'static,
    {
        match self {
            AsyncBehaviorState::Action(a) => a.reset(ctx),
            AsyncBehaviorState::Invert(a) => a.reset(ctx),
            AsyncBehaviorState::Sequence(a) => a.reset(ctx),
            AsyncBehaviorState::Select(a) => a.reset(ctx),
        }
    }
}

impl<A> std::future::Future for AsyncBehaviorState<A>
where
    A: Unpin,
{
    type Output = bool;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.as_mut().get_mut() {
            AsyncBehaviorState::Action(a) => std::pin::pin!(a).poll(cx),
            AsyncBehaviorState::Invert(a) => std::pin::pin!(a).poll(cx),
            AsyncBehaviorState::Sequence(a) => std::pin::pin!(a).poll(cx),
            AsyncBehaviorState::Select(a) => std::pin::pin!(a).poll(cx),
        }
    }
}
