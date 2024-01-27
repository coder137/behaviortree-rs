use crate::Status;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct ChildState {
    child_state: State,
    child_status: Option<Status>,
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
    // Control nodes
    Sequence(Vec<ChildState>),
    Select(Vec<ChildState>),
    // Decorator nodes
    Invert(Box<ChildState>),
}
