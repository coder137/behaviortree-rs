use behaviortree_common::Behavior;

use crate::{
    BehaviorTreeAsyncRunner, SafeDeltaType,
    behavior_nodes::{AsyncAction, AsyncInvert, AsyncSequence},
};

pub enum AsyncBehaviorState<A> {
    Action(AsyncAction<A>),
    Invert(AsyncInvert<A>),
    Sequence(AsyncSequence<A>),
}

impl<A> AsyncBehaviorState<A> {
    pub fn from_behavior<R>(behavior: Behavior<A>, runner: R, delta: SafeDeltaType) -> Self
    where
        R: BehaviorTreeAsyncRunner<A> + 'static,
        A: Clone + 'static,
    {
        match behavior {
            Behavior::Action(action) => {
                Self::Action(AsyncAction::new(runner.clone(), action, delta.clone()))
            }
            Behavior::Wait(_) => todo!(),
            Behavior::Invert(behavior) => {
                let child = Self::from_behavior(*behavior, runner.clone(), delta.clone());
                Self::Invert(AsyncInvert::new(child))
            }
            Behavior::Sequence(behaviors) => {
                let children = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior(b, runner.clone(), delta.clone()))
                    .collect::<Vec<_>>();
                Self::Sequence(AsyncSequence::new(children))
            }
            Behavior::Select(behaviors) => todo!(),
            Behavior::WhileAll(behaviors, behavior) => todo!(),
        }
    }

    pub fn reset<R>(&mut self, runner: R, delta: SafeDeltaType)
    where
        R: BehaviorTreeAsyncRunner<A> + 'static,
        A: Clone + 'static,
    {
        match self {
            AsyncBehaviorState::Action(async_action) => async_action.reset(runner, delta),
            AsyncBehaviorState::Invert(async_invert) => async_invert.reset(runner, delta),
            AsyncBehaviorState::Sequence(async_sequence) => async_sequence.reset(runner, delta),
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
            AsyncBehaviorState::Action(async_action) => std::pin::pin!(async_action).poll(cx),
            AsyncBehaviorState::Invert(async_invert) => std::pin::pin!(async_invert).poll(cx),
            AsyncBehaviorState::Sequence(async_sequence) => std::pin::pin!(async_sequence).poll(cx),
        }
    }
}
