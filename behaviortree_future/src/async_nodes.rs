use behaviortree_common::Behavior;

use crate::{
    BehaviorTreeAsyncRunner, SafeDeltaType,
    behavior_nodes::{AsyncAction, AsyncInvert},
};

pub enum AsyncActionType<A> {
    Action(AsyncAction<A>),
    Invert(Box<AsyncInvert<A>>),
}

impl<A> AsyncActionType<A> {
    pub fn from_behavior<R>(behavior: Behavior<A>, runner: R, delta: SafeDeltaType) -> Self
    where
        R: BehaviorTreeAsyncRunner<A> + Clone + 'static,
        A: Clone + 'static,
    {
        match behavior {
            Behavior::Action(action) => {
                //
                Self::Action(AsyncAction::new(runner.clone(), action, delta.clone()))
            }
            Behavior::Wait(_) => todo!(),
            Behavior::Invert(behavior) => {
                let child = Self::from_behavior(*behavior, runner.clone(), delta.clone());
                Self::Invert(AsyncInvert::new(child).into())
            }
            Behavior::Sequence(behaviors) => todo!(),
            Behavior::Select(behaviors) => todo!(),
            Behavior::WhileAll(behaviors, behavior) => todo!(),
        }
    }

    pub fn reset<R>(&mut self, runner: R, delta: SafeDeltaType)
    where
        R: BehaviorTreeAsyncRunner<A> + Clone + 'static,
        A: Clone + 'static,
    {
        match self {
            AsyncActionType::Action(async_action) => async_action.reset(runner, delta),
            AsyncActionType::Invert(async_invert) => async_invert.reset(runner, delta),
        }
    }
}

impl<A> std::future::Future for AsyncActionType<A>
where
    A: Unpin,
{
    type Output = bool;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.as_mut().get_mut() {
            AsyncActionType::Action(async_action) => {
                //
                std::pin::pin!(async_action).poll(cx)
            }
            AsyncActionType::Invert(async_invert) => {
                //
                std::pin::pin!(async_invert).poll(cx)
            }
        }
    }
}
