use std::{cell::RefCell, rc::Rc};

use crate::Status;

pub type StateInfo = (State, Option<Status>);
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum State {
    /// Leaf nodes
    NoChild,
    /// Decorator nodes
    SingleChild(Box<StateInfo>),
    /// Control nodes
    MultipleChildren(Vec<StateInfo>),
}

pub type ChildStateInfo = Rc<RefCell<(ChildState, Option<Status>)>>;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ChildState {
    /// Leaf nodes
    NoChild,
    /// Decorator nodes
    SingleChild(ChildStateInfo),
    /// Control nodes
    MultipleChildren(Rc<[ChildStateInfo]>),
}
