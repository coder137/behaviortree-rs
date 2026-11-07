use crate::{
    Behavior, BehaviorActionState, BehaviorObserverBuilder, BehaviorObserverRunner,
    BehaviorTreeReset, IntoBehaviorActionState,
    behavior_nodes::{AsyncInvert, AsyncSelect, AsyncSequence, AsyncSubtree, AsyncTimes},
};

pub(crate) enum AsyncBehaviorState<AS, O>
where
    AS: BehaviorActionState,
{
    Action(AS, AS::F),
    Invert(AsyncInvert<AsyncBehavior<AS, O>>),
    Sequence(AsyncSequence<AsyncBehavior<AS, O>>),
    Select(AsyncSelect<AsyncBehavior<AS, O>>),
    Times(AsyncTimes<AsyncBehavior<AS, O>>),
    Subtree(AsyncSubtree<AsyncBehavior<AS, O>>),
}

impl<AS, O> std::future::Future for AsyncBehaviorState<AS, O>
where
    AS: BehaviorActionState + Unpin,
    AS::F: Unpin,
    O: BehaviorObserverRunner + Unpin,
{
    type Output = bool;

    #[inline(always)]
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.as_mut().get_mut() {
            AsyncBehaviorState::Action(_state, f) => std::pin::Pin::new(f).poll(cx),
            AsyncBehaviorState::Invert(f) => std::pin::Pin::new(f).poll(cx),
            AsyncBehaviorState::Sequence(f) => std::pin::Pin::new(f).poll(cx),
            AsyncBehaviorState::Select(f) => std::pin::Pin::new(f).poll(cx),
            AsyncBehaviorState::Times(f) => std::pin::Pin::new(f).poll(cx),
            AsyncBehaviorState::Subtree(f) => std::pin::Pin::new(f).poll(cx),
        }
    }
}

impl<AS, O> BehaviorTreeReset for AsyncBehaviorState<AS, O>
where
    AS: BehaviorActionState,
    O: BehaviorObserverRunner + Unpin,
{
    #[inline(always)]
    fn reset(&mut self) {
        match self {
            AsyncBehaviorState::Action(state, f) => {
                //
                state.reset_future(f)
            }
            AsyncBehaviorState::Invert(r) => r.reset(),
            AsyncBehaviorState::Sequence(r) => r.reset(),
            AsyncBehaviorState::Select(r) => r.reset(),
            AsyncBehaviorState::Times(r) => r.reset(),
            AsyncBehaviorState::Subtree(r) => r.reset(),
        }
    }
}

pub(crate) struct AsyncBehavior<AS, O>
where
    AS: BehaviorActionState,
{
    pub state: AsyncBehaviorState<AS, O>,
    pub observer: O,
}

/// When no observer
///
/// self = AsyncBehavior {
/// state: AsyncBehaviorState<AS, ()>
/// observer: ()
/// }
///
/// tree = ()
///
/// When observer
/// self = AsyncBehavior {
/// state: AsyncBehaviorState<AS, AsyncBehaviorObserver<O>>
/// observer: AsyncBehaviorObserver<O>
/// }
///
/// tree = BehaviorObserverTree
impl<AS, O> AsyncBehavior<AS, O>
where
    AS: BehaviorActionState,
{
    pub(crate) fn from_behavior<A, R, OB>(
        behavior: Behavior<A>,
        runner: &mut R,
        observer_builder: &mut OB,
    ) -> (Self, OB::Tree)
    where
        A: IntoBehaviorActionState<R, State = AS>,
        AS: BehaviorActionState,
        OB: BehaviorObserverBuilder<A, Observer = O>,
    {
        let this_id = observer_builder.new_id();
        let (state, tree) = match behavior {
            Behavior::Action(action) => {
                let tree = OB::action(&action, this_id);
                let action_state = action.to_state(runner);
                let future = action_state.create_future();
                let state = AsyncBehaviorState::Action(action_state, future);
                (state, tree)
            }
            Behavior::Invert(behavior) => {
                let (child, child_tree) = Self::from_behavior(*behavior, runner, observer_builder);
                let state = AsyncBehaviorState::Invert(AsyncInvert::new(child));
                let tree = OB::invert(this_id, child_tree);
                (state, tree)
            }
            Behavior::Sequence(behaviors) => {
                let (children, children_tree): (Vec<Self>, Vec<OB::Tree>) = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior(b, runner, observer_builder))
                    .unzip();
                let tree = OB::sequence(this_id, children_tree);
                let state = AsyncBehaviorState::Sequence(AsyncSequence::new(children));
                (state, tree)
            }
            Behavior::Select(behaviors) => {
                let (children, children_tree): (Vec<Self>, Vec<OB::Tree>) = behaviors
                    .into_iter()
                    .map(|b| Self::from_behavior(b, runner, observer_builder))
                    .unzip();
                let tree = OB::select(this_id, children_tree);
                let state = AsyncBehaviorState::Select(AsyncSelect::new(children));
                (state, tree)
            }
            Behavior::Subtree(name, behavior) => {
                let (child, child_tree) = Self::from_behavior(*behavior, runner, observer_builder);
                let state = AsyncBehaviorState::Subtree(AsyncSubtree::new(child));
                let tree = OB::subtree(name, this_id, child_tree);
                (state, tree)
            }
        };
        let observer = observer_builder.new_observer(this_id);
        let this = AsyncBehavior::<AS, OB::Observer> { state, observer };
        (this, tree)
    }
}

impl<AS, O> std::future::Future for AsyncBehavior<AS, O>
where
    AS: BehaviorActionState + Unpin,
    AS::F: Unpin,
    O: BehaviorObserverRunner + Unpin,
{
    type Output = bool;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.as_mut().get_mut();

        this.observer.start();
        match std::pin::Pin::new(&mut this.state).poll(cx) {
            std::task::Poll::Ready(status) => {
                this.observer.end(status);
                std::task::Poll::Ready(status)
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

impl<AS, O> BehaviorTreeReset for AsyncBehavior<AS, O>
where
    AS: BehaviorActionState,
    O: BehaviorObserverRunner + Unpin,
{
    fn reset(&mut self) {
        self.state.reset();
        self.observer.reset();
    }
}

#[cfg(test)]
mod tests {
    use crate::observer_builder::NoObserverBuilder;

    use super::*;

    impl<AS> AsyncBehavior<AS, ()>
    where
        AS: BehaviorActionState,
    {
        pub fn new<A, R>(behavior: Behavior<A>, runner: &mut R) -> Self
        where
            A: IntoBehaviorActionState<R, State = AS>,
            AS: BehaviorActionState,
        {
            let (this, _tree) = Self::from_behavior(behavior, runner, &mut NoObserverBuilder);
            this
        }
    }

    #[test]
    fn test_async_behavior_trait() {
        static_assertions::assert_impl_all!(AsyncBehavior<(), ()>: Send, Sync);
        static_assertions::assert_impl_all!(AsyncBehaviorState<(), ()>: Send, Sync);
    }
}
