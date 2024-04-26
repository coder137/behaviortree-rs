use crate::Status;

pub type ChildState = (State, Option<Status>);

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum State {
    // Leaf nodes
    NoChild,

    // Decorator nodes
    SingleChild(Box<ChildState>),

    // Control nodes
    MultipleChildren(Vec<ChildState>),
}
