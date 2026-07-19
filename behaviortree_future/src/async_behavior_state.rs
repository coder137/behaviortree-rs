use crate::{
    ActionToActionState, AsyncActionContext, AsyncBehaviorActionState, Behavior, BehaviorTreeReset,
    behavior_nodes::{
        AsyncAction, AsyncInvert, AsyncLoop, AsyncSelect, AsyncSequence, AsyncSubtree, AsyncTimes,
    },
};

#[pin_project::pin_project(project = AsyncBehaviorStateProj)]
pub enum AsyncBehaviorState<AS, R> {
    Action(#[pin] AsyncAction<AS>),
    Invert(#[pin] AsyncInvert<AsyncBehaviorState<AS, R>>),
    Sequence(#[pin] AsyncSequence<AsyncBehaviorState<AS, R>, R>),
    Select(#[pin] AsyncSelect<AsyncBehaviorState<AS, R>, R>),
    Loop(#[pin] AsyncLoop<AsyncBehaviorState<AS, R>, R>),
    Times(#[pin] AsyncTimes<AsyncBehaviorState<AS, R>, R>),
    Subtree(#[pin] AsyncSubtree<AsyncBehaviorState<AS, R>>),
}

impl<AS, R> AsyncBehaviorState<AS, R> {
    pub fn from_behavior<A>(behavior: Behavior<A>, mut ctx: AsyncActionContext<R>) -> Self
    where
        A: ActionToActionState<AS, R>,
        AS: AsyncBehaviorActionState<R>,
    {
        match behavior {
            Behavior::Action(action) => {
                let action_state = ctx.runner_ref_mut(|r| {
                    //
                    action.to_state(r)
                });
                Self::Action(AsyncAction::new(action_state, ctx))
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
            Behavior::Subtree(_name, behavior) => {
                let child = Self::from_behavior(*behavior, ctx);
                Self::Subtree(AsyncSubtree::new(child))
            }
        }
    }
}

impl<AS, R> BehaviorTreeReset<R> for AsyncBehaviorState<AS, R>
where
    AS: AsyncBehaviorActionState<R>,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        let r: &mut dyn BehaviorTreeReset<R> = match self {
            AsyncBehaviorState::Action(a) => a,
            AsyncBehaviorState::Invert(a) => a,
            AsyncBehaviorState::Sequence(a) => a,
            AsyncBehaviorState::Select(a) => a,
            AsyncBehaviorState::Loop(a) => a,
            AsyncBehaviorState::Times(a) => a,
            AsyncBehaviorState::Subtree(a) => a,
        };
        r.reset(ctx);
    }
}

impl<AS, R> std::future::Future for AsyncBehaviorState<AS, R>
where
    AS: AsyncBehaviorActionState<R>,
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
            AsyncBehaviorStateProj::Subtree(f) => f,
        };
        future.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_nodes::{TestOperation, TestOperationRunner};

    use super::*;

    #[test]
    fn test_trait_assumptions() {
        // Trait: Unpin
        static_assertions::assert_impl_all!(AsyncBehaviorState<TestOperation, TestOperationRunner>: Unpin);

        // Trait: !Send
        static_assertions::assert_not_impl_all!(AsyncBehaviorState<TestOperation, TestOperationRunner>: Send);
    }
}
