use std::rc::Rc;

use crate::{
    AsyncActionContext, Behavior, BehaviorTreeAsyncAction, BehaviorTreeObserver, BehaviorTreeReset,
    Status,
    behavior_nodes::{AsyncAction, AsyncInvert, AsyncLoop, AsyncSelect, AsyncSequence, AsyncTimes},
};

#[derive(Debug, Clone)]
pub enum AsyncBehaviorStateTree {
    Action(&'static str, usize),
    Invert(usize, Rc<AsyncBehaviorStateTree>),
    Sequence(usize, Rc<[AsyncBehaviorStateTree]>),
    Select(usize, Rc<[AsyncBehaviorStateTree]>),
    Loop(usize, Rc<AsyncBehaviorStateTree>),
    Times(usize, Rc<AsyncBehaviorStateTree>),
}

#[pin_project::pin_project(project = AsyncBehaviorStateProj)]
pub enum AsyncBehaviorState<A, R, O> {
    Action(#[pin] AsyncAction<A>, Option<(Rc<O>, usize)>),
    Invert(
        #[pin] AsyncInvert<AsyncBehaviorState<A, R, O>>,
        Option<(Rc<O>, usize)>,
    ),
    Sequence(
        #[pin] AsyncSequence<AsyncBehaviorState<A, R, O>, R>,
        Option<(Rc<O>, usize)>,
    ),
    Select(
        #[pin] AsyncSelect<AsyncBehaviorState<A, R, O>, R>,
        Option<(Rc<O>, usize)>,
    ),
    Loop(
        #[pin] AsyncLoop<AsyncBehaviorState<A, R, O>, R>,
        Option<(Rc<O>, usize)>,
    ),
    Times(
        #[pin] AsyncTimes<AsyncBehaviorState<A, R, O>, R>,
        Option<(Rc<O>, usize)>,
    ),
}

impl<A, R, O> AsyncBehaviorState<A, R, O> {
    pub fn from_behavior_with_observer(
        behavior: Behavior<A>,
        ctx: AsyncActionContext<R>,
        observer: Rc<O>,
        id: &mut usize,
    ) -> (Self, AsyncBehaviorStateTree)
    where
        A: BehaviorTreeAsyncAction<R>,
        O: BehaviorTreeObserver<A>,
    {
        let parent_id = *id;
        *id += 1;
        let parent_o = (observer.clone(), parent_id);
        match behavior {
            Behavior::Action(action) => {
                let action_name = observer.action_name(&action);
                let state_tree = AsyncBehaviorStateTree::Action(action_name, parent_o.1);
                let state = Self::Action(AsyncAction::new(action, ctx), Some(parent_o));
                (state, state_tree)
            }
            Behavior::Invert(behavior) => {
                let (child_state, child_state_tree) =
                    Self::from_behavior_with_observer(*behavior, ctx, observer, id);
                let state_tree =
                    AsyncBehaviorStateTree::Invert(parent_o.1, child_state_tree.into());
                let state = Self::Invert(AsyncInvert::new(child_state), Some(parent_o));
                (state, state_tree)
            }
            Behavior::Sequence(behaviors) => {
                let (children_state, children_state_tree): (
                    Vec<AsyncBehaviorState<A, R, O>>,
                    Vec<AsyncBehaviorStateTree>,
                ) = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior_with_observer(b, ctx, observer.clone(), id))
                    .unzip();
                let children_state_tree = std::rc::Rc::from(children_state_tree);
                let state_tree = AsyncBehaviorStateTree::Sequence(parent_o.1, children_state_tree);
                let state = Self::Sequence(AsyncSequence::new(children_state, ctx), Some(parent_o));
                (state, state_tree)
            }
            Behavior::Select(behaviors) => {
                let (children_state, children_state_tree): (
                    Vec<AsyncBehaviorState<A, R, O>>,
                    Vec<AsyncBehaviorStateTree>,
                ) = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior_with_observer(b, ctx, observer.clone(), id))
                    .unzip();
                let children_state_tree = std::rc::Rc::from(children_state_tree);
                let state_tree = AsyncBehaviorStateTree::Select(parent_o.1, children_state_tree);
                let state = Self::Select(AsyncSelect::new(children_state, ctx), Some(parent_o));
                (state, state_tree)
            }
            Behavior::Loop(behavior) => {
                let (child_state, child_state_tree) =
                    Self::from_behavior_with_observer(*behavior, ctx, observer, id);
                let state_tree = AsyncBehaviorStateTree::Loop(parent_o.1, child_state_tree.into());
                let state = Self::Loop(AsyncLoop::new(child_state, ctx), Some(parent_o));
                (state, state_tree)
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
                Self::Invert(AsyncInvert::new(child), None)
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
                Self::Select(AsyncSelect::new(children, ctx), None)
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
            AsyncBehaviorState::Invert(a, o) => (a, o),
            AsyncBehaviorState::Sequence(a, o) => (a, o),
            AsyncBehaviorState::Select(a, o) => (a, o),
            AsyncBehaviorState::Loop(a, o) => (a, o),
            AsyncBehaviorState::Times(a, o) => (a, o),
        };
        r.reset(ctx);
        if let Some((o, id)) = o.as_ref() {
            o.update(*id, None);
        }
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
            AsyncBehaviorStateProj::Invert(f, o) => (f, o),
            AsyncBehaviorStateProj::Sequence(f, o) => (f, o),
            AsyncBehaviorStateProj::Select(f, o) => (f, o),
            AsyncBehaviorStateProj::Loop(f, o) => (f, o),
            AsyncBehaviorStateProj::Times(f, o) => (f, o),
        };
        let poll_status = future.poll(cx);
        let status = Status::from(poll_status);
        if let Some((o, id)) = observer.as_ref() {
            o.update(*id, Some(status));
        }
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
    fn test_trait_assumptions() {
        // Trait: Unpin
        static_assertions::assert_impl_all!(AsyncBehaviorState<TestOperation, TestOperationRunner, ()>: Unpin);

        // Trait: !Send
        static_assertions::assert_not_impl_all!(AsyncBehaviorState<TestOperation, TestOperationRunner, ()>: Send);
    }
}
