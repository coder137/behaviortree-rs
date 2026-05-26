use crate::{
    AsyncActionContext, Behavior, BehaviorTreeAsyncAction, BehaviorTreeReset,
    behavior_nodes::{AsyncAction, AsyncInvert, AsyncLoop, AsyncSelect, AsyncSequence, AsyncTimes},
};

#[pin_project::pin_project(project = AsyncBehaviorStateProj)]
pub enum AsyncBehaviorState<A, R> {
    Action(#[pin] AsyncAction<A>),
    Invert(#[pin] AsyncInvert<AsyncBehaviorState<A, R>>),
    Sequence(#[pin] AsyncSequence<AsyncBehaviorState<A, R>, R>),
    Select(#[pin] AsyncSelect<AsyncBehaviorState<A, R>, R>),
    Loop(#[pin] AsyncLoop<AsyncBehaviorState<A, R>, R>),
    Times(#[pin] AsyncTimes<AsyncBehaviorState<A, R>, R>),
}

impl<A, R> AsyncBehaviorState<A, R> {
    pub fn from_behavior(behavior: Behavior<A>, ctx: AsyncActionContext<R>) -> Self
    where
        A: BehaviorTreeAsyncAction<R>,
    {
        match behavior {
            Behavior::Action(action) => {
                //
                Self::Action(AsyncAction::new(action, ctx))
            }
            Behavior::Invert(behavior) => {
                let child = Self::from_behavior(*behavior, ctx);
                Self::Invert(AsyncInvert::new(child))
            }
            Behavior::Sequence(behaviors) => {
                let children = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior(b, ctx))
                    .collect::<Vec<_>>();
                Self::Sequence(AsyncSequence::new(children, ctx))
            }
            Behavior::Select(behaviors) => {
                let children = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior(b, ctx))
                    .collect::<Vec<_>>();
                Self::Select(AsyncSelect::new(children, ctx))
            }
            Behavior::Loop(behavior) => {
                let child = Self::from_behavior(*behavior, ctx);
                Self::Loop(AsyncLoop::new(child, ctx))
            }
        }
    }
}

impl<A, R> BehaviorTreeReset<R> for AsyncBehaviorState<A, R>
where
    A: BehaviorTreeAsyncAction<R>,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        let r: &mut dyn BehaviorTreeReset<R> = match self {
            AsyncBehaviorState::Action(a) => a,
            AsyncBehaviorState::Invert(a) => a,
            AsyncBehaviorState::Sequence(a) => a,
            AsyncBehaviorState::Select(a) => a,
            AsyncBehaviorState::Loop(a) => a,
            AsyncBehaviorState::Times(a) => a,
        };
        r.reset(ctx);
    }
}

impl<A, R> std::future::Future for AsyncBehaviorState<A, R>
where
    A: BehaviorTreeAsyncAction<R>,
{
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
            AsyncBehaviorStateProj::Loop(f) => f,
            AsyncBehaviorStateProj::Times(f) => f,
        };
        future.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_nodes::{TestOperation, TestOperationRunner};

    use super::*;

    #[test]
    fn test_unpin_or_not_unpin() {
        // Trait: Unpin
        static_assertions::assert_impl_all!(AsyncBehaviorState<TestOperation, TestOperationRunner>: Unpin);

        // Trait: !Send
        static_assertions::assert_not_impl_all!(AsyncBehaviorState<TestOperation, TestOperationRunner>: Send);
    }
}
