use std::{cell::Cell, rc::Rc};

pub type UnsafeDeltaType = Rc<Cell<f64>>;

#[derive(Clone)]
pub struct SafeDeltaType(UnsafeDeltaType);

impl SafeDeltaType {
    pub fn get(&self) -> f64 {
        self.0.get()
    }
}

impl From<UnsafeDeltaType> for SafeDeltaType {
    fn from(value: Rc<Cell<f64>>) -> Self {
        Self(value)
    }
}

/// Trait that must be implemented on runner for user action
///
/// Runner must either
/// - Point to the same memory location
/// - Contain fields that point to the same memory location
///
/// For example:
///
/// ```
/// #[derive(Clone)]
/// pub enum Action {}
///
///
/// pub struct Data {}
/// type RcData = std::rc::Rc<Data>;
/// impl behaviortree_future::BehaviorTreeAsyncRunner<Action> for RcData {
///
/// fn create_future(
///        self,
///        action: Action, delta: behaviortree_future::SafeDeltaType) -> impl std::future::Future<Output = bool> {
///     async move { true }
/// }
///
/// fn reset(&mut self, action: &Action) {}
///
/// }
/// ```
///
/// ```
/// pub struct Data {}
/// type R = std::rc::Rc<std::cell::RefCell<Data>>;
/// ```
///
/// ```rust
/// pub struct Data {}
/// #[derive(Clone)]
/// struct RefData {
///     inner: std::rc::Rc<Data>
/// }
/// type R = RefData;
/// ```
pub trait BehaviorTreeAsyncRunner<A>
where
    Self: Clone,
    A: Clone,
{
    fn create_future(
        self,
        action: A,
        delta: SafeDeltaType,
    ) -> impl std::future::Future<Output = bool>;

    fn reset(&mut self, action: &A);
}
