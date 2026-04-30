use crate::{
    AsyncActionContext, Behavior, BehaviorTreeAsyncAction, BehaviorTreeReset,
    behavior_nodes::{AsyncAction, AsyncInvert, AsyncSelect, AsyncSequence},
};

#[pin_project::pin_project(project = AsyncBehaviorStateProj)]
pub enum AsyncBehaviorState<A> {
    Action(#[pin] AsyncAction<A>),
    Invert(#[pin] AsyncInvert<A>),
    Sequence(#[pin] AsyncSequence<A>),
    Select(#[pin] AsyncSelect<A>),
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
}

impl<A, R> BehaviorTreeReset<R> for AsyncBehaviorState<A>
where
    A: BehaviorTreeAsyncAction<R> + Clone + 'static,
    R: 'static,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        let r: &mut dyn BehaviorTreeReset<R> = match self {
            AsyncBehaviorState::Action(a) => a,
            AsyncBehaviorState::Invert(a) => a,
            AsyncBehaviorState::Sequence(a) => a,
            AsyncBehaviorState::Select(a) => a,
        };
        r.reset(ctx);
    }
}

impl<A> std::future::Future for AsyncBehaviorState<A> {
    type Output = bool;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let future: std::pin::Pin<&mut dyn std::future::Future<Output = bool>> = match this {
            AsyncBehaviorStateProj::Action(f) => f,
            AsyncBehaviorStateProj::Invert(f) => f,
            AsyncBehaviorStateProj::Sequence(f) => f,
            AsyncBehaviorStateProj::Select(f) => f,
        };
        future.poll(cx)
    }
}
