use async_trait::async_trait;

use crate::{AsyncAction, async_child::AsyncChild, util::yield_now};

pub struct AsyncSelectState<S> {
    children: Vec<AsyncChild<S>>,
    completed: bool,
}

impl<S> AsyncSelectState<S> {
    pub fn new(children: Vec<AsyncChild<S>>) -> Self {
        Self {
            children,
            completed: false,
        }
    }
}

#[async_trait(?Send)]
impl<S> AsyncAction<S> for AsyncSelectState<S> {
    #[tracing::instrument(level = "trace", name = "Select::run", skip_all, ret)]
    async fn run(&mut self, delta: tokio::sync::watch::Receiver<f64>, shared: &S) -> bool {
        match self.completed {
            true => unreachable!(),
            false => {}
        }
        let mut status = false;
        let last = self.children.len() - 1;
        for (index, child) in self.children.iter_mut().enumerate() {
            let child_status = child.run(delta.clone(), shared).await;
            if child_status {
                status = true;
                break;
            }
            // Only one child should be run per tick
            // This means that if they are more children after the current child,
            // we must yield back to the executor
            if index != last {
                yield_now().await;
            }
        }
        self.completed = true;
        status
    }

    #[tracing::instrument(level = "trace", name = "Select::reset", skip_all, ret)]
    fn reset(&mut self, shared: &mut S) {
        self.children.iter_mut().for_each(|child| {
            child.reset(shared);
        });
        self.completed = false;
    }

    fn name(&self) -> &'static str {
        "Select"
    }
}

#[cfg(test)]
mod tests {
    use behaviortree_common::Behavior;
    use ticked_async_executor::TickedAsyncExecutor;

    use crate::test_async_behavior_interface::{DELTA, TestAction, TestShared};

    use super::*;

    #[test]
    fn test_select_success() {
        let behavior = Behavior::Select(vec![Behavior::Action(TestAction::Success)]);
        let mut select = AsyncChild::from_behavior(behavior);

        let executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let shared = TestShared;

        executor
            .spawn_local("SelectFuture", async move {
                let status = select.run(delta, &shared).await;
                assert!(status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_select_failure() {
        let behavior = Behavior::Select(vec![Behavior::Action(TestAction::Failure)]);
        let mut select = AsyncChild::from_behavior(behavior);

        let executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let shared = TestShared;

        executor
            .spawn_local("SelectFuture", async move {
                let status = select.run(delta, &shared).await;
                assert!(!status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_select_running() {
        let behavior = Behavior::Select(vec![Behavior::Action(TestAction::SuccessAfter {
            times: 1,
        })]);
        let mut select = AsyncChild::from_behavior(behavior);

        let executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let shared = TestShared;

        executor
            .spawn_local("SelectFuture", async move {
                let status = select.run(delta, &shared).await;
                assert!(status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_select_multiple_children() {
        let behavior = Behavior::Select(vec![
            Behavior::Action(TestAction::Failure),
            Behavior::Action(TestAction::Failure),
        ]);
        let mut select = AsyncChild::from_behavior(behavior);

        let executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let shared = TestShared;

        executor
            .spawn_local("SelectFuture", async move {
                let status = select.run(delta, &shared).await;
                assert!(!status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }

    #[test]
    fn test_select_multiple_children_early_failure() {
        let behavior = Behavior::Select(vec![
            Behavior::Action(TestAction::Failure),
            Behavior::Action(TestAction::Success),
            Behavior::Action(TestAction::Success),
        ]);
        let mut select = AsyncChild::from_behavior(behavior);

        let executor = TickedAsyncExecutor::default();

        let delta = executor.tick_channel();
        let mut shared = TestShared;

        executor
            .spawn_local("SelectFuture", async move {
                let status = select.run(delta.clone(), &shared).await;
                assert!(status);
                select.reset(&mut shared);
                let status = select.run(delta, &shared).await;
                assert!(status);
            })
            .detach();

        assert_eq!(executor.num_tasks(), 1);
        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        // reset

        executor.tick(DELTA, None);
        executor.tick(DELTA, None);
        assert_eq!(executor.num_tasks(), 0);
    }
}
