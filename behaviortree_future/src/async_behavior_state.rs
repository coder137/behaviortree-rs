use std::rc::Rc;

use crate::{
    AsyncActionContext, Behavior, BehaviorTreeAsyncAction, BehaviorTreeObserver, BehaviorTreeReset,
    Status,
    behavior_nodes::{AsyncAction, AsyncInvert, AsyncLoop, AsyncSelect, AsyncSequence, AsyncTimes},
};

#[derive(Debug)]
pub enum AsyncBehaviorStateObserver<A> {
    Action(A, usize),
    Invert(usize, Rc<AsyncBehaviorStateObserver<A>>),
    Sequence(usize, Rc<[AsyncBehaviorStateObserver<A>]>),
    Select(usize, Rc<[AsyncBehaviorStateObserver<A>]>),
    Loop(usize, Rc<AsyncBehaviorStateObserver<A>>),
    Times(usize, Rc<AsyncBehaviorStateObserver<A>>),
}

#[pin_project::pin_project(project = AsyncBehaviorStateProj)]
pub enum AsyncBehaviorState<A, R, O> {
    Action(#[pin] AsyncAction<A>, Option<(Rc<O>, usize)>),
    Invert(#[pin] AsyncInvert<AsyncBehaviorState<A, R, O>>),
    Sequence(
        #[pin] AsyncSequence<AsyncBehaviorState<A, R, O>, R>,
        Option<(Rc<O>, usize)>,
    ),
    Select(#[pin] AsyncSelect<AsyncBehaviorState<A, R, O>, R>),
    Loop(
        #[pin] AsyncLoop<AsyncBehaviorState<A, R, O>, R>,
        Option<(Rc<O>, usize)>,
    ),
    Times(#[pin] AsyncTimes<AsyncBehaviorState<A, R, O>, R>),
}

impl<A, R, O> AsyncBehaviorState<A, R, O> {
    pub fn from_behavior_with_observer(
        behavior: Behavior<A>,
        ctx: AsyncActionContext<R>,
        observer: Rc<O>,
        id: &mut usize,
    ) -> (Self, AsyncBehaviorStateObserver<A>)
    where
        A: Clone + BehaviorTreeAsyncAction<R>,
        O: BehaviorTreeObserver<A>,
    {
        let parent_id = *id;
        *id += 1;
        let parent_o = (observer.clone(), parent_id);
        match behavior {
            Behavior::Action(action) => {
                let state_observer = AsyncBehaviorStateObserver::Action(action.clone(), parent_o.1);
                let state = Self::Action(AsyncAction::new(action, ctx), Some(parent_o));
                (state, state_observer)
            }
            Behavior::Invert(behavior) => {
                let (child_state, child_state_observer) =
                    Self::from_behavior_with_observer(*behavior, ctx, observer, id);
                let state = Self::Invert(AsyncInvert::new(child_state));
                let state_observer =
                    AsyncBehaviorStateObserver::Invert(parent_o.1, child_state_observer.into());
                (state, state_observer)
            }
            Behavior::Sequence(behaviors) => {
                let (children_state, children_state_observer): (
                    Vec<AsyncBehaviorState<A, R, O>>,
                    Vec<AsyncBehaviorStateObserver<A>>,
                ) = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior_with_observer(b, ctx, observer.clone(), id))
                    .unzip();
                let children_state_observer = std::rc::Rc::from(children_state_observer);
                let state_observer =
                    AsyncBehaviorStateObserver::Sequence(parent_o.1, children_state_observer);
                let state = Self::Sequence(AsyncSequence::new(children_state, ctx), Some(parent_o));
                (state, state_observer)
            }
            Behavior::Select(behaviors) => {
                let (children_state, children_state_observer): (
                    Vec<AsyncBehaviorState<A, R, O>>,
                    Vec<AsyncBehaviorStateObserver<A>>,
                ) = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior_with_observer(b, ctx, observer.clone(), id))
                    .unzip();
                let children_state_observer = std::rc::Rc::from(children_state_observer);
                let state_observer =
                    AsyncBehaviorStateObserver::Select(parent_o.1, children_state_observer);
                let state = Self::Select(AsyncSelect::new(children_state, ctx));
                (state, state_observer)
            }
            Behavior::Loop(behavior) => {
                let (child_state, child_state_observer) =
                    Self::from_behavior_with_observer(*behavior, ctx, observer, id);
                let state_observer =
                    AsyncBehaviorStateObserver::Loop(parent_o.1, child_state_observer.into());
                let state = Self::Loop(AsyncLoop::new(child_state, ctx), Some(parent_o));
                (state, state_observer)
            }
        }
    }
}

impl<A, R> AsyncBehaviorState<A, R, ()> {
    pub fn from_behavior(behavior: Behavior<A>, ctx: AsyncActionContext<R>) -> Self
    where
        A: BehaviorTreeAsyncAction<R>,
    {
        match behavior {
            Behavior::Action(action) => Self::Action(AsyncAction::new(action, ctx), None),
            Behavior::Invert(behavior) => {
                let child = Self::from_behavior(*behavior, ctx);
                Self::Invert(AsyncInvert::new(child))
            }
            Behavior::Sequence(behaviors) => {
                let children = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior(b, ctx))
                    .collect::<Vec<_>>();
                Self::Sequence(AsyncSequence::new(children, ctx), None)
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
                Self::Loop(AsyncLoop::new(child, ctx), None)
            }
        }
    }
}

impl<A, R, O> BehaviorTreeReset<R> for AsyncBehaviorState<A, R, O>
where
    A: BehaviorTreeAsyncAction<R>,
    O: BehaviorTreeObserver<A>,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        let (r, o): (&mut dyn BehaviorTreeReset<R>, &mut Option<(Rc<O>, usize)>) = match self {
            AsyncBehaviorState::Action(a, o) => (a, o),
            AsyncBehaviorState::Invert(a) => (a, &mut None),
            AsyncBehaviorState::Sequence(a, o) => (a, o),
            AsyncBehaviorState::Select(a) => (a, &mut None),
            AsyncBehaviorState::Loop(a, o) => (a, o),
            AsyncBehaviorState::Times(a) => (a, &mut None),
        };
        r.reset(ctx);
        o.as_ref().map(|(o, id)| {
            o.update(*id, None);
        });
    }
}

impl<A, R, O> std::future::Future for AsyncBehaviorState<A, R, O>
where
    A: BehaviorTreeAsyncAction<R>,
    O: BehaviorTreeObserver<A>,
{
    type Output = bool;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let (future, observer): (
            std::pin::Pin<&mut dyn std::future::Future<Output = bool>>,
            &mut Option<(Rc<O>, usize)>,
        ) = match this {
            AsyncBehaviorStateProj::Action(f, o) => (f, o),
            AsyncBehaviorStateProj::Invert(f) => (f, &mut None),
            AsyncBehaviorStateProj::Sequence(f, o) => (f, o),
            AsyncBehaviorStateProj::Select(f) => (f, &mut None),
            AsyncBehaviorStateProj::Loop(f, o) => (f, o),
            AsyncBehaviorStateProj::Times(f) => (f, &mut None),
        };
        let poll_status = future.poll(cx);
        let status = Status::from(poll_status);
        observer.as_ref().map(|(o, id)| {
            o.update(*id, Some(status));
        });
        poll_status
    }
}

impl From<std::task::Poll<bool>> for Status {
    fn from(value: std::task::Poll<bool>) -> Self {
        match value {
            std::task::Poll::Ready(status) => match status {
                true => Status::Success,
                false => Status::Failure,
            },
            std::task::Poll::Pending => Status::Running,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_nodes::{TestOperation, TestOperationRunner};

    use super::*;

    #[test]
    fn test_unpin_or_not_unpin() {
        // Trait: Unpin
        static_assertions::assert_impl_all!(AsyncBehaviorState<TestOperation, TestOperationRunner, ()>: Unpin);

        // Trait: !Send
        static_assertions::assert_not_impl_all!(AsyncBehaviorState<TestOperation, TestOperationRunner, ()>: Send);
    }
}
