use std::{rc::Rc, sync::Arc};

use crate::{
    ActionName, BehaviorObserver, BehaviorObserverBuilder, BehaviorObserverRunner,
    BehaviorObserverTree, LocalBehaviorObserverTree, Status,
};

impl<O> BehaviorObserver for std::rc::Rc<O>
where
    O: BehaviorObserver,
{
    #[inline(always)]
    fn init(&self, capacity: usize) {
        self.as_ref().init(capacity);
    }

    #[inline(always)]
    fn update(&self, id: usize, status: Option<Status>) {
        self.as_ref().update(id, status);
    }
}

impl<O> BehaviorObserver for std::sync::Arc<O>
where
    O: BehaviorObserver,
{
    #[inline(always)]
    fn init(&self, capacity: usize) {
        self.as_ref().init(capacity);
    }

    #[inline(always)]
    fn update(&self, id: usize, status: Option<Status>) {
        self.as_ref().update(id, status);
    }
}

impl BehaviorObserverRunner for () {
    #[inline(always)]
    fn start(&mut self) {}

    #[inline(always)]
    fn end(&mut self, _status: bool) {}

    #[inline(always)]
    fn reset(&mut self) {}
}

//
pub struct NoObserverBuilder;

impl<A> BehaviorObserverBuilder<A> for NoObserverBuilder {
    type Observer = ();
    type Tree = ();

    #[inline(always)]
    fn new_id(&mut self) -> usize {
        0
    }

    #[inline(always)]
    fn new_observer(&self, _id: usize) -> Self::Observer {}

    #[inline(always)]
    fn action(_action: &A, _id: usize) -> Self::Tree {}

    #[inline(always)]
    fn invert(_id: usize, _child: Self::Tree) -> Self::Tree {}

    #[inline(always)]
    fn sequence(_id: usize, _children: Vec<Self::Tree>) -> Self::Tree {}

    #[inline(always)]
    fn select(_id: usize, _children: Vec<Self::Tree>) -> Self::Tree {}

    #[inline(always)]
    fn subtree(_name: String, _id: usize, _child: Self::Tree) -> Self::Tree {}
}

//
pub struct AsyncBehaviorObserver<O> {
    observer: O,
    id: usize,
    started: bool,
}

impl<O: BehaviorObserver> BehaviorObserverRunner for AsyncBehaviorObserver<O> {
    fn start(&mut self) {
        if !self.started {
            self.started = true;
            self.observer.update(self.id, Some(Status::Running));
        }
    }

    fn end(&mut self, status: bool) {
        let status = Status::from(status);
        self.observer.update(self.id, Some(status));
    }

    fn reset(&mut self) {
        self.started = false;
        self.observer.update(self.id, None);
    }
}

pub(crate) struct ObserverBuilder<O> {
    id: usize,
    observer: O,
}

impl<O> ObserverBuilder<O> {
    pub fn new(observer: O) -> Self {
        Self { id: 0, observer }
    }

    pub fn transmit_init(&self)
    where
        O: BehaviorObserver,
    {
        self.observer.init(self.id);
    }
}

impl<A: ActionName, O: BehaviorObserver> BehaviorObserverBuilder<A> for ObserverBuilder<Rc<O>> {
    type Observer = AsyncBehaviorObserver<Rc<O>>;
    type Tree = LocalBehaviorObserverTree;

    fn new_id(&mut self) -> usize {
        let this_id = self.id;
        self.id += 1;
        this_id
    }

    fn new_observer(&self, id: usize) -> Self::Observer {
        AsyncBehaviorObserver {
            observer: self.observer.clone(),
            id,
            started: false,
        }
    }

    #[inline(always)]
    fn action(action: &A, id: usize) -> Self::Tree {
        Self::Tree::Action(action.action_name(), id)
    }

    #[inline(always)]
    fn invert(id: usize, child: Self::Tree) -> Self::Tree {
        Self::Tree::Invert(id, child.into())
    }

    #[inline(always)]
    fn sequence(id: usize, children: Vec<Self::Tree>) -> Self::Tree {
        Self::Tree::Sequence(id, children.into())
    }

    #[inline(always)]
    fn select(id: usize, children: Vec<Self::Tree>) -> Self::Tree {
        Self::Tree::Select(id, children.into())
    }

    #[inline(always)]
    fn subtree(name: String, id: usize, child: Self::Tree) -> Self::Tree {
        Self::Tree::Subtree(name.into(), id, child.into())
    }
}

impl<A: ActionName, O: BehaviorObserver> BehaviorObserverBuilder<A> for ObserverBuilder<Arc<O>> {
    type Observer = AsyncBehaviorObserver<Arc<O>>;
    type Tree = BehaviorObserverTree;

    fn new_id(&mut self) -> usize {
        let this_id = self.id;
        self.id += 1;
        this_id
    }

    fn new_observer(&self, id: usize) -> Self::Observer {
        AsyncBehaviorObserver {
            observer: self.observer.clone(),
            id,
            started: false,
        }
    }

    #[inline(always)]
    fn action(action: &A, id: usize) -> Self::Tree {
        Self::Tree::Action(action.action_name(), id)
    }

    #[inline(always)]
    fn invert(id: usize, child: Self::Tree) -> Self::Tree {
        Self::Tree::Invert(id, child.into())
    }

    #[inline(always)]
    fn sequence(id: usize, children: Vec<Self::Tree>) -> Self::Tree {
        Self::Tree::Sequence(id, children.into())
    }

    #[inline(always)]
    fn select(id: usize, children: Vec<Self::Tree>) -> Self::Tree {
        Self::Tree::Select(id, children.into())
    }

    #[inline(always)]
    fn subtree(name: String, id: usize, child: Self::Tree) -> Self::Tree {
        Self::Tree::Subtree(name.into(), id, child.into())
    }
}
