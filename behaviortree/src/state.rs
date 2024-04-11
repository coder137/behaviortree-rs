use crate::Status;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum State {
    // Leaf nodes
    NoChild,

    // Decorator nodes
    SingleChild(Box<State>, Option<Status>),

    // Control nodes
    MultipleChildren(Vec<(State, Option<Status>)>),
}
