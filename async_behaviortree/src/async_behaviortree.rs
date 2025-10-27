use behaviortree_common::Behavior;
use behaviortree_common::State;
use tokio_util::sync::CancellationToken;

use crate::AsyncActionName;
use crate::AsyncActionRunner;
use crate::async_child::AsyncChild;
use crate::util::yield_now;

pub struct AsyncBehaviorController {
    state: State,
    cancellation: CancellationToken,
}

impl AsyncBehaviorController {
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn state(&self) -> State {
        self.state.clone()
    }
}

impl Drop for AsyncBehaviorController {
    fn drop(&mut self) {
        self.cancellation.cancel();
    }
}

pub struct AsyncBehaviorTree;

impl AsyncBehaviorTree {
    pub fn new<A, R>(
        behavior: Behavior<A>,
        should_loop: bool,
        delta: tokio::sync::watch::Receiver<f64>,
        mut runner: R,
    ) -> (
        impl std::future::Future<Output = ()>,
        AsyncBehaviorController,
    )
    where
        A: AsyncActionName + 'static,
        R: AsyncActionRunner<A> + 'static,
    {
        let cancellation = tokio_util::sync::CancellationToken::new();
        let cancellation_clone = cancellation.clone();

        let (mut child, state) = AsyncChild::from_behavior_with_state(behavior);
        let future = async move {
            if should_loop {
                cancellation_clone
                    .run_until_cancelled_owned(async {
                        loop {
                            let _status = child.run(delta.clone(), &mut runner).await;
                            yield_now().await;
                            child.reset(&mut runner);
                        }
                    })
                    .await;
            } else {
                cancellation_clone
                    .run_until_cancelled_owned(async {
                        let _status = child.run(delta, &mut runner).await;
                        yield_now().await;
                    })
                    .await;
            }
            child.reset(&mut runner);
        };
        (
            future,
            AsyncBehaviorController {
                state,
                cancellation,
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
    use tokio_stream::StreamExt;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    use crate::test_async_behavior_interface::{DELTA, TestAction, TestRunner};

    #[test]
    fn test_async_behaviortree() {
        let _ignore = tracing_subscriber::Registry::default()
            .with(tracing_forest::ForestLayer::default())
            .try_init();

        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);

        let mut executor = TickedAsyncExecutor::default();
        let runner = TestRunner;

        let (behaviortree_future, controller) =
            AsyncBehaviorTree::new(behavior, false, executor.tick_channel(), runner);

        let state = controller.state();
        let cancel = controller.cancel_token();
        executor
            .spawn_local("AsyncObserver", async move {
                let mut streams = tokio_stream::StreamMap::new();

                let mut pending_queue = VecDeque::from_iter([&state]);
                loop {
                    let tobs = pending_queue.pop_front();
                    let tobs = match tobs {
                        Some(tobs) => tobs,
                        None => {
                            break;
                        }
                    };
                    let (name, rx) = match tobs {
                        State::NoChild(name, rx) => (name, rx),
                        State::SingleChild(name, rx, child) => {
                            pending_queue.push_back(&*child);
                            (name, rx)
                        }
                        State::MultipleChildren(name, rx, children) => {
                            for child in children.iter() {
                                pending_queue.push_back(child);
                            }
                            (name, rx)
                        }
                    };

                    streams.insert(name, tokio_stream::wrappers::WatchStream::new(rx.clone()));
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
                        tracing::info!("State: {:?}", data);
                    }
                };
                let _r = cancel.run_until_cancelled(fut).await;
            })
            .detach();
        executor.tick(DELTA, None);

        executor
            .spawn_local("AsyncBehaviorTreeFuture", behaviortree_future)
            .detach();

        tracing::info!("Start 1");
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 2);

        tracing::info!("2");
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 2);

        tracing::info!("3");
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 1);

        tracing::info!("4");
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);

        tracing::info!("End 5");
    }

    #[test]
    fn test_async_behaviortree_shutdown() {
        let _ignore = tracing_subscriber::Registry::default()
            .with(tracing_forest::ForestLayer::default())
            .try_init();

        let behavior = Behavior::Sequence(vec![
            Behavior::Invert(Box::new(Behavior::Action(TestAction::Failure))),
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);
        // let behavior = Behavior::Loop(Box::new(behavior));

        let mut executor = TickedAsyncExecutor::default();
        let runner = TestRunner;

        let (behaviortree_future, controller) =
            AsyncBehaviorTree::new(behavior, true, executor.tick_channel(), runner);

        executor
            .spawn_local("AsyncBehaviorTreeFuture", behaviortree_future)
            .detach();

        let state = controller.state();
        for _ in 0..10 {
            executor.tick(DELTA, None);
            tracing::info!("Observer: {state:?}");
        }
        drop(controller);

        while executor.num_tasks() != 0 {
            executor.tick(DELTA, None);
        }
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_watch_channel() {
        let (tx, mut rx) = tokio::sync::watch::channel(());
        let changed = rx.has_changed().unwrap();
        assert!(!changed);

        let _r = tx.send_replace(());
        let changed = rx.has_changed().unwrap();
        assert!(changed);
        rx.mark_unchanged();

        let changed = rx.has_changed().unwrap();
        assert!(!changed);
    }
}
