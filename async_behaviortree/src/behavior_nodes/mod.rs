#[async_trait::async_trait(?Send)]
pub trait AsyncAction<R> {
    /// Asynchronously runs the action till completion
    ///
    /// User implementation must ensure that `run` is non-blocking.
    /// - Should `.await` internally if action has not completed.
    /// - Nodes with child(ren) internally must also ensure that only one child is run
    /// before yielding back to the executor.
    ///
    /// Once `run` has completed i.e returns `true`/`false`,
    /// clients should `reset` before `run`ning.
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, runner: &mut R) -> bool;

    /// Resets the current action to its initial/newly created state
    fn reset(&mut self, runner: &mut R);

    /// Identify your action
    fn name(&self) -> &'static str;
}

// Leaf
mod action_node;
pub use action_node::*;

mod wait_node;
pub use wait_node::*;

// Decorator
mod invert_node;
pub use invert_node::*;

// Control
mod sequence_node;
pub use sequence_node::*;

mod select_node;
pub use select_node::*;

mod while_all_node;
pub use while_all_node::*;
