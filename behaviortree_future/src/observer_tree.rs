use std::{rc::Rc, sync::Arc};

#[derive(Debug, Clone)]
pub enum LocalBehaviorObserverTree {
    Action(&'static str, usize),
    Invert(usize, Rc<LocalBehaviorObserverTree>),
    Sequence(usize, Rc<[LocalBehaviorObserverTree]>),
    Select(usize, Rc<[LocalBehaviorObserverTree]>),
    Subtree(Rc<str>, usize, Rc<LocalBehaviorObserverTree>),
}

#[derive(Debug, Clone)]
pub enum BehaviorObserverTree {
    Action(&'static str, usize),
    Invert(usize, Arc<BehaviorObserverTree>),
    Sequence(usize, Arc<[BehaviorObserverTree]>),
    Select(usize, Arc<[BehaviorObserverTree]>),
    Subtree(Arc<str>, usize, Arc<BehaviorObserverTree>),
}
