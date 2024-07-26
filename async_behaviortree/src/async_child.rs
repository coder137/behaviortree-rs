use std::rc::Rc;

use behaviortree_common::{Behavior, Status};

use crate::{
    behavior_nodes::{AsyncInvertState, AsyncSelectState, AsyncSequenceState, AsyncWaitState},
    AsyncAction, AsyncControl, AsyncDecorator, ToAsyncAction,
};

enum AsyncBehaviorType<S> {
    Leaf(Box<dyn AsyncAction<S>>),
    Decorator(Box<dyn AsyncDecorator<S>>, Box<AsyncChild<S>>),
    Control(Box<dyn AsyncControl<S>>, Vec<AsyncChild<S>>),
}

pub type AsyncChildObserverChannel = tokio::sync::watch::Receiver<Option<Status>>;

#[derive(Clone)]
pub enum AsyncChildObserver {
    NoChild(AsyncChildObserverChannel),
    SingleChild(AsyncChildObserverChannel, Rc<AsyncChildObserver>),
    MultipleChildren(AsyncChildObserverChannel, Rc<[AsyncChildObserver]>),
}

pub struct AsyncChild<S> {
    action: AsyncBehaviorType<S>,
    state: tokio::sync::watch::Sender<Option<Status>>,
}

impl<S> AsyncChild<S> {
    pub fn from_behavior<A>(behavior: Behavior<A>) -> Self
    where
        A: ToAsyncAction<S>,
        S: 'static,
    {
        let action = match behavior {
            Behavior::Action(action) => AsyncBehaviorType::Leaf(action.to_async_action()),
            Behavior::Wait(target) => {
                AsyncBehaviorType::Leaf(Box::new(AsyncWaitState::new(target)))
            }
            Behavior::Invert(behavior) => {
                let child = Self::from_behavior(*behavior);
                let action = Box::new(AsyncInvertState::new());
                AsyncBehaviorType::Decorator(action, Box::new(child))
            }
            Behavior::Sequence(behaviors) => {
                let children = Self::from_behaviors(behaviors);
                let action = Box::new(AsyncSequenceState::new());
                AsyncBehaviorType::Control(action, children)
            }
            Behavior::Select(behaviors) => {
                let children = Self::from_behaviors(behaviors);
                let action = Box::new(AsyncSelectState::new());
                AsyncBehaviorType::Control(action, children)
            }
        };
        Self::from_action(action)
    }

    pub fn from_behaviors<A>(mut behaviors: Vec<Behavior<A>>) -> Vec<Self>
    where
        A: ToAsyncAction<S>,
        S: 'static,
    {
        behaviors
            .drain(..)
            .map(|behavior| Self::from_behavior(behavior))
            .collect()
    }

    pub async fn run(
        &mut self,
        delta: &mut tokio::sync::watch::Receiver<f64>,
        shared: &mut S,
    ) -> bool {
        let _r = self.state.send(Some(Status::Running));
        let success = match &mut self.action {
            AsyncBehaviorType::Leaf(action) => action.run(delta, shared).await,
            AsyncBehaviorType::Decorator(decorator, child) => {
                decorator.run(child, delta, shared).await
            }
            AsyncBehaviorType::Control(control, children) => {
                control.run(children, delta, shared).await
            }
        };
        let status = if success {
            Status::Success
        } else {
            Status::Failure
        };
        let _r = self.state.send(Some(status));
        success
    }

    pub fn reset(&mut self) {
        if self.state.borrow().is_none() {
            return;
        }
        match &mut self.action {
            AsyncBehaviorType::Leaf(action) => {
                action.reset();
            }
            AsyncBehaviorType::Decorator(decorator, child) => {
                child.reset();
                decorator.reset();
            }
            AsyncBehaviorType::Control(control, children) => {
                children.iter_mut().for_each(|child| {
                    child.reset();
                });
                control.reset();
            }
        }
        let _r = self.state.send(None);
    }

    pub fn observer(&self) -> AsyncChildObserver {
        match &self.action {
            AsyncBehaviorType::Leaf(_) => AsyncChildObserver::NoChild(self.state.subscribe()),
            AsyncBehaviorType::Decorator(_, child) => {
                AsyncChildObserver::SingleChild(self.state.subscribe(), Rc::new(child.observer()))
            }
            AsyncBehaviorType::Control(_, children) => {
                let children = children.iter().map(|child| child.observer()).collect();
                AsyncChildObserver::MultipleChildren(self.state.subscribe(), children)
            }
        }
    }

    fn from_action(action: AsyncBehaviorType<S>) -> Self {
        Self {
            action,
            state: tokio::sync::watch::channel(None).0,
        }
    }
}
