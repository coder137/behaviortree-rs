use std::{cell::Cell, rc::Rc};

use crate::{
    ActionName, BehaviorActionState, BehaviorObserver, IntoBehaviorActionState,
    LocalBehaviorObserverTree, RootBehavior,
    async_generic_behavior_tree::{AsyncBehaviorTreeController, AsyncGenericBehaviorTree},
    observer_builder::{AsyncBehaviorObserver, NoObserverBuilder, ObserverBuilder},
};

pub type AsyncLocalBehaviorTree<AS, O> = AsyncGenericBehaviorTree<AS, O, Rc<Cell<bool>>>;

impl<AS> AsyncLocalBehaviorTree<AS, ()>
where
    AS: BehaviorActionState,
{
    pub fn from_behavior<A, R>(
        root_behavior: RootBehavior<A>,
        runner: &mut R,
    ) -> (Self, AsyncBehaviorTreeController<Rc<Cell<bool>>>)
    where
        A: IntoBehaviorActionState<R, State = AS>,
        AS: BehaviorActionState,
    {
        let (bt, bt_controller, _bt_tree) =
            AsyncLocalBehaviorTree::from_behavior_and_observer_builder(
                root_behavior,
                runner,
                &mut NoObserverBuilder,
            );
        (bt, bt_controller)
    }
}

impl<AS, O> AsyncLocalBehaviorTree<AS, AsyncBehaviorObserver<Rc<O>>>
where
    AS: BehaviorActionState,
{
    pub fn from_behavior_with_observer<A, R>(
        root_behavior: RootBehavior<A>,
        runner: &mut R,
        observer: O,
    ) -> (
        Self,
        AsyncBehaviorTreeController<Rc<Cell<bool>>>,
        LocalBehaviorObserverTree,
    )
    where
        A: IntoBehaviorActionState<R, State = AS> + ActionName,
        AS: BehaviorActionState,
        O: BehaviorObserver,
    {
        let mut observer_builder = ObserverBuilder::new(Rc::new(observer));
        let (bt, bt_controller, bt_tree) =
            AsyncLocalBehaviorTree::from_behavior_and_observer_builder(
                root_behavior,
                runner,
                &mut observer_builder,
            );
        observer_builder.transmit_init();
        (bt, bt_controller, bt_tree)
    }
}
