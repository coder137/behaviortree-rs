use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::{
    BehaviorActionState, BehaviorObserverBuilder, BehaviorObserverRunner, BehaviorTreeReset,
    IntoBehaviorActionState, RootBehavior, async_behavior::AsyncBehavior,
    async_root_behavior::AsyncRootBehavior, behavior_nodes::AsyncLoop,
};

pub struct AsyncBehaviorTreeController<CR>(CR);

impl AsyncBehaviorTreeController<Rc<Cell<bool>>> {
    pub fn reset(&self) {
        self.0.set(true);
    }
}

impl AsyncBehaviorTreeController<Arc<AtomicBool>> {
    pub fn reset(&self) {
        self.0.store(true, Ordering::Release);
    }
}

trait BehaviorTreeControllerReader {
    fn should_reset(&self) -> bool;
}

impl BehaviorTreeControllerReader for Rc<Cell<bool>> {
    fn should_reset(&self) -> bool {
        self.replace(false)
    }
}

impl BehaviorTreeControllerReader for Arc<AtomicBool> {
    fn should_reset(&self) -> bool {
        self.swap(false, Ordering::Acquire)
    }
}

pub struct AsyncGenericBehaviorTree<AS, O, CR>
where
    AS: BehaviorActionState,
{
    state: AsyncRootBehavior<AS, O>,
    controller_reader: CR,
}

impl<AS, O, CR> AsyncGenericBehaviorTree<AS, O, CR>
where
    AS: BehaviorActionState,
{
    pub(crate) fn from_behavior_and_observer_builder<A, R, OB>(
        root_behavior: RootBehavior<A>,
        runner: &mut R,
        observer_builder: &mut OB,
    ) -> (Self, AsyncBehaviorTreeController<CR>, OB::Tree)
    where
        A: IntoBehaviorActionState<R, State = AS>,
        AS: BehaviorActionState,
        OB: BehaviorObserverBuilder<A, Observer = O>,
        CR: Default + Clone,
    {
        let (state, bt_observer_tree) = match root_behavior {
            RootBehavior::Once(behavior) => {
                let (bt, bt_observer_tree) =
                    AsyncBehavior::from_behavior(behavior, runner, observer_builder);
                (AsyncRootBehavior::Once(bt), bt_observer_tree)
            }
            RootBehavior::Loop(behavior) => {
                let (bt, bt_observer_tree) =
                    AsyncBehavior::from_behavior(behavior, runner, observer_builder);
                (
                    AsyncRootBehavior::Loop(AsyncLoop::new(bt)),
                    bt_observer_tree,
                )
            }
        };

        let controller_reader = CR::default();
        let controller = AsyncBehaviorTreeController(controller_reader.clone());
        let bt = Self {
            state,
            controller_reader,
        };
        (bt, controller, bt_observer_tree)
    }
}

impl<AS, O, CR> std::future::Future for AsyncGenericBehaviorTree<AS, O, CR>
where
    AS: BehaviorActionState + Unpin,
    AS::F: Unpin,
    O: BehaviorObserverRunner + Unpin,
    CR: BehaviorTreeControllerReader + Unpin,
{
    type Output = bool;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();
        if bt.controller_reader.should_reset() {
            bt.state.reset();
        }
        let state = std::pin::Pin::new(&mut bt.state);
        state.poll(cx)
    }
}
