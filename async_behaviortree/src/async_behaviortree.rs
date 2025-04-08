use std::future::Future;

use behaviortree_common::Behavior;
use behaviortree_common::State;

use crate::AsyncChild;

use crate::ToAsyncAction;

pub enum AsyncBehaviorTreePolicy {
    /// Resets/Reloads the behavior tree once it is completed
    ReloadOnCompletion,
    /// On completion, needs manual reset
    RetainOnCompletion,
}

pub struct AsyncBehaviorController {
    observer: State,
    reset_tx: tokio::sync::watch::Sender<()>,
    shutdown_tx: tokio::sync::watch::Sender<()>,
}

impl AsyncBehaviorController {
    pub fn observer(&self) -> State {
        self.observer.clone()
    }

    pub fn reset(&self) {
        let _ignore = self.reset_tx.send(());
    }

    pub fn shutdown(&self) {
        let _ignore = self.shutdown_tx.send(());
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
        let observer = child.state();

        let (reset_tx, mut reset_rx) = tokio::sync::watch::channel(());
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(());
        let behavior_future = async move {
            enum State {
                ChildCompleted,
                ResetNotification,
                ShutdownNotification,
            }
            loop {
                let state = tokio::select! {
                    _ = child.run(&mut delta, &mut shared) => {
                        State::ChildCompleted
                    }
                    _ = reset_rx.changed() => {
                        State::ResetNotification
                    }
                    _ = shutdown_rx.changed() => {
                        State::ShutdownNotification
                    }
                };

                match state {
                    State::ChildCompleted => match behavior_policy {
                        AsyncBehaviorTreePolicy::ReloadOnCompletion => {
                            tokio::task::yield_now().await;
                            child.reset(&mut shared);
                        }
                        AsyncBehaviorTreePolicy::RetainOnCompletion => {
                            break;
                        }
                    },
                    State::ResetNotification => {
                        reset_rx.mark_unchanged();
                        tokio::task::yield_now().await;
                        child.reset(&mut shared);
                        match behavior_policy {
                            AsyncBehaviorTreePolicy::ReloadOnCompletion => {}
                            AsyncBehaviorTreePolicy::RetainOnCompletion => {
                                break;
                            }
                        }
                    }
                    State::ShutdownNotification => {
                        shutdown_rx.mark_unchanged();
                        child.reset(&mut shared);
                        break;
                    }
                }
            }
        };

        (
            behavior_future,
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
                        State::NoChild(_name, rx) => rx,
                        State::SingleChild(_name, rx, child) => {
                            pending_queue.push_back(&*child);
                            rx
                        }
                        State::MultipleChildren(_name, rx, children) => {
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
            Behavior::Invert(Box::new(Behavior::Action(TestAction::Failure))),
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

        let observer = controller.observer();
        for _ in 0..10 {
            executor.tick(DELTA);
            println!("Observer: {observer:?}");
        }
        controller.shutdown();

        while executor.num_tasks() != 0 {
            executor.tick(DELTA);
        }
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_watch_channel() {
        let (tx, mut rx) = tokio::sync::watch::channel(());
        let changed = rx.has_changed().unwrap();
        assert!(!changed);

        let _r = tx.send(());
        let changed = rx.has_changed().unwrap();
        assert!(changed);
        rx.mark_unchanged();

        let changed = rx.has_changed().unwrap();
        assert!(!changed);
    }
}
