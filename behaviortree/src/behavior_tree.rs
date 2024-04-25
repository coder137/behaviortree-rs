use crate::{Action, Behavior, State, Status, ToAction};

pub enum BehaviorTreePolicy {
    /// Resets/Reloads the behavior tree once it is completed
    ReloadOnCompletion,
    /// On completion, needs manual reset
    RetainOnCompletion,
}

pub struct BehaviorTree<A, S> {
    behavior: Behavior<A>,
    behavior_policy: BehaviorTreePolicy,

    // State
    status: Option<Status>,
    action: Box<dyn Action<S>>,
}

impl<A, S> Action<S> for BehaviorTree<A, S> {
    fn tick(&mut self, dt: f64, shared: &mut S) -> Status {
        if let Some(status) = self.status {
            if status == Status::Success || status == Status::Failure {
                match self.behavior_policy {
                    BehaviorTreePolicy::ReloadOnCompletion => {
                        self.reset();
                        // Ticks the action below
                    }
                    BehaviorTreePolicy::RetainOnCompletion => {
                        // Do nothing!
                        // `status` returns the already completed value
                        return status;
                    }
                }
            }
        }

        let status = self.action.tick(dt, shared);
        self.status = Some(status);
        status
    }

    fn reset(&mut self) {
        self.action.reset();
    }

    fn state(&self) -> State {
        self.action.state()
    }
}

impl<A, S> BehaviorTree<A, S> {
    pub fn new(behavior: Behavior<A>, behavior_policy: BehaviorTreePolicy) -> Self
    where
        A: ToAction<S> + Clone + 'static,
        S: 'static,
    {
        let action: Box<dyn Action<S>> = Box::from(behavior.clone());
        Self {
            behavior,
            behavior_policy,
            status: None,
            action,
        }
    }

    pub fn tick_with_observer<O>(&mut self, dt: f64, shared: &mut S, observer: &mut O) -> Status
    where
        O: FnMut(State, Status),
    {
        let status = self.tick(dt, shared);
        let state = self.state();
        observer(state, status);
        status
    }

    pub fn behavior(&self) -> &Behavior<A> {
        &self.behavior
    }

    pub fn status(&self) -> Option<Status> {
        self.status
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_behavior_interface::{TestActions, TestShared};

    #[test]
    fn behavior_tree() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
            Behavior::Action(TestActions::SuccessTimes { ticks: 1 }),
        ]);
        let mut tree = BehaviorTree::new(behavior, BehaviorTreePolicy::RetainOnCompletion);

        let mut shared = TestShared::default();
        let mut observer = |state: State, status: Status| {
            println!("Status: {:?}, State: {:#?}", status, state);
        };

        tree.tick_with_observer(0.1, &mut shared, &mut observer);

        tree.tick_with_observer(0.1, &mut shared, &mut observer);

        tree.tick_with_observer(0.1, &mut shared, &mut observer);

        tree.tick_with_observer(0.1, &mut shared, &mut observer);
    }
}
