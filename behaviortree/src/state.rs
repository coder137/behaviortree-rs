use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use crate::Status;

pub type ChildStateInfoInner = Rc<RefCell<(ChildState, Option<Status>)>>;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChildStateInfo(ChildStateInfoInner);

impl From<ChildStateInfoInner> for ChildStateInfo {
    fn from(inner: ChildStateInfoInner) -> Self {
        Self(inner)
    }
}

impl From<(ChildState, Option<Status>)> for ChildStateInfo {
    fn from(inner: (ChildState, Option<Status>)) -> Self {
        Self(Rc::new(RefCell::new(inner)))
    }
}

impl ChildStateInfo {
    pub fn get(&self) -> Ref<'_, (ChildState, Option<Status>)> {
        self.0.borrow()
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ChildState {
    /// Leaf nodes
    NoChild,
    /// Decorator nodes
    SingleChild(ChildStateInfo),
    /// Control nodes
    MultipleChildren(Rc<[ChildStateInfo]>),
}
