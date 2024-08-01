use std::future::Future;

use behaviortree_common::Behavior;

use crate::AsyncChild;
use crate::AsyncChildObserver;

use crate::ToAsyncAction;

pub enum AsyncBehaviorTreePolicy {
    /// Resets/Reloads the behavior tree once it is completed
    ReloadOnCompletion,
    /// On completion, needs manual reset
    RetainOnCompletion,
}

enum BehaviorControllerMessage {
    Reset,
    Shutdown,
}

pub struct AsyncBehaviorController {
    observer: AsyncChildObserver,
    message_tx: tokio::sync::mpsc::Sender<BehaviorControllerMessage>,
}

impl AsyncBehaviorController {
    pub fn observer(&self) -> AsyncChildObserver {
        self.observer.clone()
    }

    pub fn reset(&self) {
        let _r = self.message_tx.try_send(BehaviorControllerMessage::Reset);
    }

    pub fn shutdown(&self) {
        let _r = self
            .message_tx
            .try_send(BehaviorControllerMessage::Shutdown);
    }
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
        let (message_tx, mut message_rx) =
            tokio::sync::mpsc::channel::<BehaviorControllerMessage>(2);

        let behavior_future = async move {
            loop {
                tokio::select! {
                    _ = child.run(&mut delta, &mut shared) => {
                        match behavior_policy {
                            AsyncBehaviorTreePolicy::ReloadOnCompletion => {child.reset();},
                            AsyncBehaviorTreePolicy::RetainOnCompletion => {break;},
                        }
                    }
                    message = message_rx.recv() => {
                        let message = match message {
                            Some(message) => message,
                            None => break,
                        };
                        match message {
                            BehaviorControllerMessage::Reset => {
                                child.reset();
                                match behavior_policy {
                                    AsyncBehaviorTreePolicy::ReloadOnCompletion => {},
                                    AsyncBehaviorTreePolicy::RetainOnCompletion => {break;},
                                }
                            },
                            BehaviorControllerMessage::Shutdown => {
                                child.reset();
                                break;
                            },
                        }
                    }
                }
            }
        };

        (
            behavior_future,
            AsyncBehaviorController {
                observer,
                message_tx,
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

        executor.tick(DELTA);
        executor.tick(DELTA);
        executor.tick(DELTA);
        let _r = shut_tx.try_send(());
        executor.tick(DELTA);
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
        controller.reset();

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
        controller.shutdown();

        executor.tick(DELTA);
        assert_eq!(executor.num_tasks(), 0);
    }
}
