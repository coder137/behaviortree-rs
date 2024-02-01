use crate::Status;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct ChildState {
    pub child_state: State,
    pub child_status: Option<Status>,
}
impl ChildState {
    pub fn new(child_state: State, child_status: Option<Status>) -> Self {
        Self {
            child_state,
            child_status,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum State {
    // Leaf nodes
    NoChild,

    // Decorator nodes
    SingleChild(Box<ChildState>),

    // Control nodes
    MultipleChildren(Vec<ChildState>),
}
