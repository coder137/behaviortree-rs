use crate::Behavior;

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RootBehavior<A> {
    Once(Behavior<A>),
    Loop(Behavior<A>),
}
