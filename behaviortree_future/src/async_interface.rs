use std::{cell::Cell, rc::Rc};

use crate::Status;

pub struct Delta(Cell<f64>);

impl Delta {
    pub fn get(&self) -> f64 {
        self.0.get()
    }

    pub(crate) fn update(&self, delta: f64) {
        self.0.set(delta);
    }
}

impl Default for Delta {
    fn default() -> Self {
        Self(Cell::new(0.0))
    }
}

pub(crate) trait BehaviorTreeReset {
    fn reset(&mut self);
}

pub trait BehaviorTreeAsyncHandler<'a> {
    type Output;
    fn future(self, future: impl std::future::Future<Output = bool> + 'a) -> Self::Output;
}

pub trait ActionToActionState<AS, R>
where
    AS: AsyncBehaviorActionState,
{
    fn to_state(self, delta: Rc<Delta>, runner: &mut R) -> AS;
}

pub trait AsyncBehaviorActionState {
    fn make_future<'a, H>(&self, handler: H) -> H::Output
    where
        H: BehaviorTreeAsyncHandler<'a>;

    fn reset(&self);
}

pub trait BehaviorTreeObserver<AS> {
    fn action_name(action_state: &AS) -> &'static str;

    /// Ids are assigned from 0 -> capacity
    ///
    /// When init is called we have [0..=capacity] nodes have status: `None`
    fn init(&self, capacity: usize);

    fn update(&self, id: usize, status: Option<Status>);
}
