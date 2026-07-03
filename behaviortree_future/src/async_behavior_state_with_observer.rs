use std::rc::Rc;

use crate::{
    ActionToActionState, AsyncActionContext, AsyncBehaviorActionState, Behavior,
    BehaviorTreeObserver, BehaviorTreeReset, Status,
    behavior_nodes::{
        AsyncAction, AsyncInvert, AsyncLoop, AsyncSelect, AsyncSequence, AsyncSubtree, AsyncTimes,
    },
};

#[derive(Debug, Clone)]
pub enum AsyncBehaviorStateTree {
    Action(&'static str, usize),
    Invert(usize, Rc<AsyncBehaviorStateTree>),
    Sequence(usize, Rc<[AsyncBehaviorStateTree]>),
    Select(usize, Rc<[AsyncBehaviorStateTree]>),
    Loop(usize, Rc<AsyncBehaviorStateTree>),
    Times(usize, Rc<AsyncBehaviorStateTree>),
    Subtree(Rc<String>, usize, Rc<AsyncBehaviorStateTree>),
}

#[pin_project::pin_project(project = AsyncBehaviorStateWithObserverProj)]
pub enum AsyncBehaviorStateWithObserver<AS, R, O> {
    Action(#[pin] AsyncAction<AS>, (Rc<O>, usize)),
    Invert(
        #[pin] AsyncInvert<AsyncBehaviorStateWithObserver<AS, R, O>>,
        (Rc<O>, usize),
    ),
    Sequence(
        #[pin] AsyncSequence<AsyncBehaviorStateWithObserver<AS, R, O>, R>,
        (Rc<O>, usize),
    ),
    Select(
        #[pin] AsyncSelect<AsyncBehaviorStateWithObserver<AS, R, O>, R>,
        (Rc<O>, usize),
    ),
    Loop(
        #[pin] AsyncLoop<AsyncBehaviorStateWithObserver<AS, R, O>, R>,
        (Rc<O>, usize),
    ),
    Times(
        #[pin] AsyncTimes<AsyncBehaviorStateWithObserver<AS, R, O>, R>,
        (Rc<O>, usize),
    ),
    Subtree(
        #[pin] AsyncSubtree<AsyncBehaviorStateWithObserver<AS, R, O>>,
        (Rc<O>, usize),
    ),
}

impl<AS, R, O> AsyncBehaviorStateWithObserver<AS, R, O> {
    pub fn from_behavior<A>(
        behavior: Behavior<A>,
        ctx: AsyncActionContext<R>,
        observer: Rc<O>,
        id: &mut usize,
    ) -> (Self, AsyncBehaviorStateTree)
    where
        A: ActionToActionState<AS>,
        AS: AsyncBehaviorActionState<R>,
        O: BehaviorTreeObserver<AS>,
    {
        let parent_id = *id;
        *id += 1;
        let parent_o = (observer.clone(), parent_id);
        match behavior {
            Behavior::Action(action) => {
                let action_state = action.to_state();
                let action_name = O::action_name(&action_state);
                let state_tree = AsyncBehaviorStateTree::Action(action_name, parent_o.1);
                let state = Self::Action(AsyncAction::new(action_state, ctx), parent_o);
                (state, state_tree)
            }
            Behavior::Invert(behavior) => {
                let (child_state, child_state_tree) =
                    Self::from_behavior(*behavior, ctx, observer, id);
                let state_tree =
                    AsyncBehaviorStateTree::Invert(parent_o.1, child_state_tree.into());
                let state = Self::Invert(AsyncInvert::new(child_state), parent_o);
                (state, state_tree)
            }
            Behavior::Sequence(behaviors) => {
                let (children_state, children_state_tree): (
                    Vec<AsyncBehaviorStateWithObserver<AS, R, O>>,
                    Vec<AsyncBehaviorStateTree>,
                ) = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior(b, ctx, observer.clone(), id))
                    .unzip();
                let children_state_tree = std::rc::Rc::from(children_state_tree);
                let state_tree = AsyncBehaviorStateTree::Sequence(parent_o.1, children_state_tree);
                let state = Self::Sequence(AsyncSequence::new(children_state, ctx), parent_o);
                (state, state_tree)
            }
            Behavior::Select(behaviors) => {
                let (children_state, children_state_tree): (
                    Vec<AsyncBehaviorStateWithObserver<AS, R, O>>,
                    Vec<AsyncBehaviorStateTree>,
                ) = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior(b, ctx, observer.clone(), id))
                    .unzip();
                let children_state_tree = std::rc::Rc::from(children_state_tree);
                let state_tree = AsyncBehaviorStateTree::Select(parent_o.1, children_state_tree);
                let state = Self::Select(AsyncSelect::new(children_state, ctx), parent_o);
                (state, state_tree)
            }
            Behavior::Loop(behavior) => {
                let (child_state, child_state_tree) =
                    Self::from_behavior(*behavior, ctx, observer, id);
                let state_tree = AsyncBehaviorStateTree::Loop(parent_o.1, child_state_tree.into());
                let state = Self::Loop(AsyncLoop::new(child_state, ctx), parent_o);
                (state, state_tree)
            }
            Behavior::Subtree(name, behavior) => {
                let (child_state, child_state_tree) =
                    Self::from_behavior(*behavior, ctx, observer, id);
                let state_tree = AsyncBehaviorStateTree::Subtree(
                    name.into(),
                    parent_o.1,
                    child_state_tree.into(),
                );
                let state = Self::Subtree(AsyncSubtree::new(child_state), parent_o);
                (state, state_tree)
            }
        }
    }
}

impl<AS, R, O> BehaviorTreeReset<R> for AsyncBehaviorStateWithObserver<AS, R, O>
where
    AS: AsyncBehaviorActionState<R>,
    O: BehaviorTreeObserver<AS>,
{
    fn reset(&mut self, ctx: AsyncActionContext<R>) {
        let (r, observer): (&mut dyn BehaviorTreeReset<R>, &mut (Rc<O>, usize)) = match self {
            AsyncBehaviorStateWithObserver::Action(a, o) => (a, o),
            AsyncBehaviorStateWithObserver::Invert(a, o) => (a, o),
            AsyncBehaviorStateWithObserver::Sequence(a, o) => (a, o),
            AsyncBehaviorStateWithObserver::Select(a, o) => (a, o),
            AsyncBehaviorStateWithObserver::Loop(a, o) => (a, o),
            AsyncBehaviorStateWithObserver::Times(a, o) => (a, o),
            AsyncBehaviorStateWithObserver::Subtree(a, o) => (a, o),
        };
        r.reset(ctx);
        observer.0.update(observer.1, None);
    }
}

impl<AS, R, O> std::future::Future for AsyncBehaviorStateWithObserver<AS, R, O>
where
    AS: AsyncBehaviorActionState<R>,
    O: BehaviorTreeObserver<AS>,
{
    type Output = bool;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let (future, observer): (
            std::pin::Pin<&mut dyn std::future::Future<Output = bool>>,
            &mut (Rc<O>, usize),
        ) = match this {
            AsyncBehaviorStateWithObserverProj::Action(f, o) => (f, o),
            AsyncBehaviorStateWithObserverProj::Invert(f, o) => (f, o),
            AsyncBehaviorStateWithObserverProj::Sequence(f, o) => (f, o),
            AsyncBehaviorStateWithObserverProj::Select(f, o) => (f, o),
            AsyncBehaviorStateWithObserverProj::Loop(f, o) => (f, o),
            AsyncBehaviorStateWithObserverProj::Times(f, o) => (f, o),
            AsyncBehaviorStateWithObserverProj::Subtree(f, o) => (f, o),
        };
        let poll_status = future.poll(cx);
        let status = Status::from(poll_status);
        observer.0.update(observer.1, Some(status));
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
