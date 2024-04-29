use std::{cell::RefCell, rc::Rc};

use crate::Status;

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
