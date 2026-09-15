use std::rc::Rc;

use crate::{
    ActionToActionState, AsyncBehaviorActionState, Behavior, BehaviorTreeReset, Delta,
    behavior_nodes::{
        AsyncAction, AsyncInvert, AsyncLoop, AsyncSelect, AsyncSequence, AsyncSubtree, AsyncTimes,
    },
};

#[pin_project::pin_project(project = AsyncBehaviorStateProj)]
pub enum AsyncBehaviorState<AS> {
    Action(#[pin] AsyncAction<AS>),
    Invert(#[pin] AsyncInvert<AsyncBehaviorState<AS>>),
    Sequence(#[pin] AsyncSequence<AsyncBehaviorState<AS>>),
    Select(#[pin] AsyncSelect<AsyncBehaviorState<AS>>),
    Loop(#[pin] AsyncLoop<AsyncBehaviorState<AS>>),
    Times(#[pin] AsyncTimes<AsyncBehaviorState<AS>>),
    Subtree(#[pin] AsyncSubtree<AsyncBehaviorState<AS>>),
}

impl<AS> AsyncBehaviorState<AS> {
    pub fn from_behavior<A, R>(behavior: Behavior<A>, delta: Rc<Delta>, runner: &mut R) -> Self
    where
        A: ActionToActionState<AS, R>,
        AS: AsyncBehaviorActionState,
    {
        match behavior {
            Behavior::Action(action) => {
                let action_state = action.to_state(delta, runner);
                Self::Action(AsyncAction::new(action_state))
            }
            Behavior::Invert(behavior) => {
                let child = Self::from_behavior(*behavior, delta, runner);
                Self::Invert(AsyncInvert::new(child))
            }
            Behavior::Sequence(behaviors) => {
                let children = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior(b, delta.clone(), runner))
                    .collect::<Vec<_>>();
                Self::Sequence(AsyncSequence::new(children))
            }
            Behavior::Select(behaviors) => {
                let children = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior(b, delta.clone(), runner))
                    .collect::<Vec<_>>();
                Self::Select(AsyncSelect::new(children))
            }
            Behavior::Loop(behavior) => {
                let child = Self::from_behavior(*behavior, delta, runner);
                Self::Loop(AsyncLoop::new(child))
            }
            Behavior::Subtree(_name, behavior) => {
                let child = Self::from_behavior(*behavior, delta, runner);
                Self::Subtree(AsyncSubtree::new(child))
            }
        }
    }
}

impl<AS> BehaviorTreeReset for AsyncBehaviorState<AS>
where
    AS: AsyncBehaviorActionState,
{
    fn reset(&mut self) {
        let r: &mut dyn BehaviorTreeReset = match self {
            AsyncBehaviorState::Action(a) => a,
            AsyncBehaviorState::Invert(a) => a,
            AsyncBehaviorState::Sequence(a) => a,
            AsyncBehaviorState::Select(a) => a,
            AsyncBehaviorState::Loop(a) => a,
            AsyncBehaviorState::Times(a) => a,
            AsyncBehaviorState::Subtree(a) => a,
        };
        r.reset();
    }
}

impl<AS> std::future::Future for AsyncBehaviorState<AS>
where
    AS: AsyncBehaviorActionState,
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
    use crate::test_nodes::TestOperationState;

    use super::*;

    #[test]
    fn test_trait_assumptions() {
        // Trait: Unpin
        static_assertions::assert_impl_all!(AsyncBehaviorState<TestOperationState>: Unpin);

        // Trait: !Send
        static_assertions::assert_not_impl_all!(AsyncBehaviorState<TestOperationState>: Send);
    }
}
