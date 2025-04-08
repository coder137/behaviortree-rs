use crate::{behavior_nodes::*, Behavior, State, Status};

/// Modelled after the `std::future::Future` trait
pub trait Action<S> {
    /// Ticks the action once.
    ///
    /// User implementation must ensure that calls to `tick` are non-blocking.
    /// Should return `Status::Running` if action has not completed.
    ///
    /// Can be called multiple times.
    /// Once `tick` has completed i.e returns `Status::Success`/`Status::Failure`,
    /// clients should `reset` before `tick`ing.
    fn tick(&mut self, delta: f64, shared: &mut S) -> Status;

    /// Resets the current action to its initial/newly created state
    fn reset(&mut self);

    /// Identify your action
    fn name(&self) -> &'static str;
}

pub trait ToAction<S> {
    fn to_action(self) -> Box<dyn Action<S>>;
}

pub struct Child<S> {
    action: Box<dyn Action<S>>,
    status: tokio::sync::watch::Sender<Option<Status>>,
    state: State,
}

impl<S> Child<S> {
    pub fn new(
        action: Box<dyn Action<S>>,
        status: tokio::sync::watch::Sender<Option<Status>>,
        state: State,
    ) -> Self {
        Self {
            action,
            status,
            state,
        }
    }

    pub fn from_behavior<A>(behavior: Behavior<A>) -> Self
    where
        A: ToAction<S>,
        S: 'static,
    {
        match behavior {
            Behavior::Action(action) => {
                let action = action.to_action();
                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::NoChild(action.name(), rx);

                Self::new(action, tx, state)
            }
            Behavior::Wait(target) => {
                let action: Box<dyn Action<S>> = Box::new(WaitState::new(target));
                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::NoChild(action.name(), rx);

                Self::new(action, tx, state)
            }
            Behavior::Invert(child) => {
                let child = Child::from_behavior(*child);
                let child_state = child.state();

                let action = InvertState::new(child);
                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::SingleChild(action.name(), rx, child_state.into());

                Self::new(Box::new(action), tx, state)
            }
            Behavior::Sequence(children) => {
                let children = children
                    .into_iter()
                    .map(|child| Child::from_behavior(child))
                    .collect::<Vec<_>>();
                let children_states = children.iter().map(|child| child.state());
                let children_states = std::rc::Rc::from_iter(children_states);

                let action = SequenceState::new(children);
                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::MultipleChildren(action.name(), rx, children_states);

                Self::new(Box::new(action), tx, state)
            }
            Behavior::Select(children) => {
                let children = children
                    .into_iter()
                    .map(|child| Child::from_behavior(child))
                    .collect::<Vec<_>>();
                let children_states = children.iter().map(|child| child.state());
                let children_states = std::rc::Rc::from_iter(children_states);

                let action = SelectState::new(children);
                let (tx, rx) = tokio::sync::watch::channel(None);
                let state = State::MultipleChildren(action.name(), rx, children_states);

                Self::new(Box::new(action), tx, state)
            }
        }
    }

    pub fn tick(&mut self, delta: f64, shared: &mut S) -> Status {
        let status = self.action.tick(delta, shared);
        let _ignore = self.status.send(Some(status));
        status
    }

    pub fn reset(&mut self) {
        self.action.reset();
        let _ignore = self.status.send(None);
    }

    pub fn status(&self) -> Option<Status> {
        *self.status.borrow()
    }

    pub fn state(&self) -> State {
        self.state.clone()
    }
}

#[cfg(test)]
pub mod test_behavior_interface {
    use super::*;
    use tracing::info;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    #[derive(Default)]
    pub struct TestShared;

    struct GenericTestAction {
        name: &'static str,
        status: bool,
        times: usize,
        elapsed: usize,
    }

    impl GenericTestAction {
        fn new(name: String, status: bool, times: usize) -> Self {
            Self {
                name: Box::new(name).leak(),
                status,
                times,
                elapsed: 0,
            }
        }
    }

    impl<S> Action<S> for GenericTestAction {
        #[tracing::instrument(level = "trace", name = "GenericTestAction", skip_all, ret)]
        fn tick(&mut self, _dt: f64, _shared: &mut S) -> Status {
            let mut status = if self.status {
                Status::Success
            } else {
                Status::Failure
            };
            self.elapsed += 1;
            if self.elapsed < self.times {
                status = Status::Running;
            }
            status
        }

        fn reset(&mut self) {
            self.elapsed = 0;
        }

        fn name(&self) -> &'static str {
            self.name
        }
    }

    #[derive(Clone, Copy)]
    pub enum TestAction {
        Success,
        Failure,
        SuccessAfter { times: usize },
        FailureAfter { times: usize },
    }

    impl ToAction<TestShared> for TestAction {
        fn to_action(self) -> Box<dyn Action<TestShared>> {
            match self {
                TestAction::Success => Box::new(GenericTestAction::new("Success".into(), true, 1)),
                TestAction::Failure => Box::new(GenericTestAction::new("Failure".into(), false, 1)),
                TestAction::SuccessAfter { times } => {
                    assert!(times >= 1);
                    Box::new(GenericTestAction::new(
                        format!("SuccessAfter{}", times),
                        true,
                        times + 1,
                    ))
                }
                TestAction::FailureAfter { times } => {
                    assert!(times >= 1);
                    Box::new(GenericTestAction::new(
                        format!("FailureAfter{}", times),
                        false,
                        times + 1,
                    ))
                }
            }
        }
    }

    #[test]
    fn test_basic_behavior() {
        let _ignore = tracing_subscriber::Registry::default()
            .with(tracing_forest::ForestLayer::default())
            .try_init();

        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestAction::Success),
            Behavior::Wait(10.0),
            Behavior::Action(TestAction::Success),
            Behavior::Invert(Behavior::Action(TestAction::Failure).into()),
            Behavior::Action(TestAction::Success),
        ]);

        let mut child = Child::from_behavior(behavior);
        let state = child.state();

        let mut shared = TestShared;

        loop {
            let status = child.tick(1.0, &mut shared);
            info!("State:\n{:#?}", state);
            if status != Status::Running {
                break;
            }
        }
    }
}
