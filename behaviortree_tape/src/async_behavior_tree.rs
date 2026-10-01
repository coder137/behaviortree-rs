use std::{cell::Cell, rc::Rc};

use crate::{Behavior, BehaviorActionState, IntoBehaviorActionState, Tape, TapeFuture};

#[derive(Clone, Copy)]
enum Control {
    None,
    Reset,
    Shutdown,
}

#[derive(Clone)]
pub struct AsyncBehaviorTreeController {
    control: Rc<Cell<Control>>,
}

///
/// State transitions
/// None -> Reset
/// None -> Shutdown
///
/// Reset -> None
/// Reset -> Shutdown
///
/// Shutdown -> Shutdown
impl AsyncBehaviorTreeController {
    pub fn reset(&self) {
        match self.control.get() {
            Control::None | Control::Reset => {
                self.control.replace(Control::Reset);
            }
            Control::Shutdown => {
                // Cannot move from shutdown to reset
            }
        }
    }

    pub fn shutdown(&self) {
        self.control.replace(Control::Shutdown);
    }
}

pub struct AsyncBehaviorTree<AS> {
    future: TapeFuture<AS>,
    control: Rc<Cell<Control>>,
}

impl<AS> AsyncBehaviorTree<AS> {
    pub fn from_behavior<A, R>(
        behavior: Behavior<A>,
        runner: &mut R,
    ) -> (Self, AsyncBehaviorTreeController)
    where
        A: IntoBehaviorActionState<AS, R>,
        AS: BehaviorActionState,
    {
        let control = Rc::new(Cell::new(Control::None));
        let controller = AsyncBehaviorTreeController {
            control: control.clone(),
        };

        let future = TapeFuture::new(behavior, runner);
        let bt = Self { future, control };
        (bt, controller)
    }

    pub fn to_flat_graph(&self) -> String {
        let graph = self.future.to_flat_graph();
        let dot = petgraph::dot::Dot::with_config(&graph, &[]);
        format!("{}", dot)
    }
}

impl<AS> std::future::Future for AsyncBehaviorTree<AS>
where
    AS: BehaviorActionState,
{
    type Output = Option<bool>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let bt = self.as_mut().get_mut();
        match bt.control.get() {
            Control::None => {}
            Control::Reset => {
                bt.control.replace(Control::None);
                bt.future.reset();
            }
            Control::Shutdown => {
                return std::task::Poll::Ready(None);
            }
        }
        let state = std::pin::Pin::new(&mut bt.future);
        state.poll(cx).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_nodes::{TestOperation, TestOperationRunner};

    use super::*;

    #[test]
    fn test_behaviortree_no_loop_with_dhat() {
        let mut executor = ticked_async_executor::TickedAsyncExecutor::default();

        let mut runner = TestOperationRunner::default();

        let bt = {
            // let _profiler = DhatTester::new("test_behaviortree_no_loop_with_dhat_pre");
            let behavior = TestOperation::Add(1, 2, true, 1);
            let (bt, _bt_controller) =
                AsyncBehaviorTree::from_behavior(Behavior::Action(behavior), &mut runner);
            bt
        };

        executor
            .spawn_local("_", async move {
                // let _profiler = DhatTester::new("test_behaviortree_no_loop_with_dhat_post");
                let status = bt.await;
                println!("Status: {status:?}");
                assert!(status.unwrap());
            })
            .detach();

        executor.tick(1.0, None);
        assert_eq!(executor.num_tasks(), 1);

        executor.tick(1.0, None);
        assert_eq!(executor.num_tasks(), 0);
    }
}
