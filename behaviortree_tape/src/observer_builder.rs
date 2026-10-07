use std::rc::Rc;

use crate::{ActionName, BehaviorObserverBuilder, BehaviorObserverTree};

pub struct NoObserver;

impl<A> BehaviorObserverBuilder<A> for NoObserver {
    type Node = ();

    #[inline(always)]
    fn action(_action: &A, _id: usize) -> Self::Node {}

    #[inline(always)]
    fn invert(_id: usize, _child: Self::Node) -> Self::Node {}

    #[inline(always)]
    fn sequence(_id: usize, _children: Vec<Self::Node>) -> Self::Node {}

    #[inline(always)]
    fn select(_id: usize, _children: Vec<Self::Node>) -> Self::Node {}

    #[inline(always)]
    fn subtree(_name: std::rc::Rc<str>, _id: usize, _child: Self::Node) -> Self::Node {}
}

pub struct DefaultObserver;

impl<A: ActionName> BehaviorObserverBuilder<A> for DefaultObserver {
    type Node = BehaviorObserverTree;

    #[inline(always)]
    fn action(action: &A, id: usize) -> Self::Node {
        BehaviorObserverTree::Action(action.action_name(), id)
    }

    #[inline(always)]
    fn invert(id: usize, child: Self::Node) -> Self::Node {
        BehaviorObserverTree::Invert(id, Rc::new(child))
    }

    #[inline(always)]
    fn sequence(id: usize, children: Vec<Self::Node>) -> Self::Node {
        BehaviorObserverTree::Sequence(id, children.into())
    }

    #[inline(always)]
    fn select(id: usize, children: Vec<Self::Node>) -> Self::Node {
        BehaviorObserverTree::Select(id, children.into())
    }

    #[inline(always)]
    fn subtree(name: std::rc::Rc<str>, id: usize, child: Self::Node) -> Self::Node {
        BehaviorObserverTree::Subtree(name, id, child.into())
    }
}
