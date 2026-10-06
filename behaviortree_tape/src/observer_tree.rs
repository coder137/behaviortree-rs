use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum BehaviorObserverTree {
    Action(&'static str, usize),
    Invert(usize, Rc<BehaviorObserverTree>),
    Sequence(usize, Rc<[BehaviorObserverTree]>),
    Select(usize, Rc<[BehaviorObserverTree]>),
    Loop(usize, Rc<BehaviorObserverTree>),
    Times(usize, Rc<BehaviorObserverTree>),
    Subtree(Rc<str>, usize, Rc<BehaviorObserverTree>),
}
