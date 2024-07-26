use std::future::Future;

use crate::AsyncChild;
use crate::AsyncChildObserver;
use crate::Behavior;

use crate::ToAsyncAction;

pub enum AsyncBehaviorTreePolicy {
    /// Resets/Reloads the behavior tree once it is completed
    ReloadOnCompletion,
    /// On completion, needs manual reset
    RetainOnCompletion,
}

pub struct AsyncBehaviorController {
    observer: AsyncChildObserver,
    reset_tx: tokio::sync::mpsc::Sender<()>,
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
}

pub struct AsyncBehaviorTree;

impl AsyncBehaviorTree {
    pub fn new<A, S>(
        behavior: Behavior<A>,
        behavior_policy: AsyncBehaviorTreePolicy,
        mut delta: tokio::sync::watch::Receiver<f64>,
        mut shared: S,
    ) -> (impl Future<Output = ()>, AsyncBehaviorController)
    where
        A: ToAsyncAction<S>,
        S: 'static,
    {
        let mut child = AsyncChild::from_behavior(behavior);
        let observer = child.observer();
        let (reset_tx, mut reset_rx) = tokio::sync::mpsc::channel::<()>(1);
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

        let behavior_fut = async move {
            loop {
                tokio::select! {
                    _ = child.run(&mut delta, &mut shared) => {
                        match behavior_policy {
                            AsyncBehaviorTreePolicy::ReloadOnCompletion => {child.reset();},
                            AsyncBehaviorTreePolicy::RetainOnCompletion => {break;},
                        }
                    }
                    _ = reset_rx.recv() => {
                        child.reset();
                        match behavior_policy {
                            AsyncBehaviorTreePolicy::ReloadOnCompletion => {},
                            AsyncBehaviorTreePolicy::RetainOnCompletion => {break;},
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        child.reset();
                        break;
                    }
                }
            }
        };

        (
            behavior_fut,
            AsyncBehaviorController {
                observer,
                reset_tx,
                shutdown_tx,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use behaviortree_common::Behavior;
    use ticked_async_executor::TickedAsyncExecutor;
    use tokio::select;
    use tokio_stream::StreamExt;

    use crate::test_async_behavior_interface::{TestAction, TestShared, DELTA};

    #[test]
    fn test_async_behaviortree() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);

        let executor = TickedAsyncExecutor::default();
        let shared = TestShared;

        let (behaviortree_future, _controller) = AsyncBehaviorTree::new(
            behavior,
            AsyncBehaviorTreePolicy::RetainOnCompletion,
            executor.tick_channel(),
            shared,
        );

        let observer = _controller.observer.clone();
        let (shut_tx, mut shut_rx) = tokio::sync::mpsc::channel::<()>(1);
        executor
            .spawn_local("AsyncObserver", async move {
                let mut streams = tokio_stream::StreamMap::new();
                let mut counter = 0;

                let mut pending_queue = VecDeque::from_iter([&observer]);
                loop {
                    let tobs = pending_queue.pop_front();
                    let tobs = match tobs {
                        Some(tobs) => tobs,
                        None => {
                            break;
                        }
                    };
                    let rx = match tobs {
                        AsyncChildObserver::NoChild(rx) => rx,
                        AsyncChildObserver::SingleChild(rx, child) => {
                            pending_queue.push_back(&*child);
                            rx
                        }
                        AsyncChildObserver::MultipleChildren(rx, children) => {
                            for child in children.iter() {
                                pending_queue.push_back(child);
                            }
                            rx
                        }
                    };
                    // let data = (counter, *rx.borrow());
                    // println!("Data: {:?}", data);
                    streams.insert(
                        counter,
                        tokio_stream::wrappers::WatchStream::new(rx.clone()),
                    );
                    counter += 1;
                }

                let fut = async move {
                    loop {
                        let data = streams.next().await;
                        let data = match data {
                            Some(data) => data,
                            None => {
                                break;
                            }
                        };
                        println!("Data: {:?}", data);
                    }
                };

                select! {
                    _ = fut => {}
                    _ = shut_rx.recv() => {}
                }
            })
            .detach();
        executor.tick(DELTA);

        executor
            .spawn_local("AsyncBehaviorTreeFuture", behaviortree_future)
            .detach();

        for i in 0..100 {
            println!("C: {i}");
            executor.tick(DELTA);
        }
        let _r = shut_tx.try_send(());
        for _ in 0..100 {
            executor.tick(DELTA);
        }
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_async_behaviortree_early_reset() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);

        let executor = TickedAsyncExecutor::default();
        let shared = TestShared;

        let (behaviortree_future, controller) = AsyncBehaviorTree::new(
            behavior,
            AsyncBehaviorTreePolicy::RetainOnCompletion,
            executor.tick_channel(),
            shared,
        );

        executor
            .spawn_local("AsyncBehaviorTreeFuture", behaviortree_future)
            .detach();

        executor.tick(DELTA);
        controller.reset_tx.try_send(()).unwrap();

        executor.tick(DELTA);
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_async_behaviortree_shutdown() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);

        let executor = TickedAsyncExecutor::default();
        let shared = TestShared;

        let (behaviortree_future, controller) = AsyncBehaviorTree::new(
            behavior,
            AsyncBehaviorTreePolicy::ReloadOnCompletion,
            executor.tick_channel(),
            shared,
        );

        executor
            .spawn_local("AsyncBehaviorTreeFuture", behaviortree_future)
            .detach();

        for _ in 0..10 {
            executor.tick(DELTA);
        }

        controller.shutdown_tx.try_send(()).unwrap();
        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }
}
