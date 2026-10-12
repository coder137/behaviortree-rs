use crate::Status;

pub trait BehaviorFutureHandler<'a> {
    type Output;
    fn future(self, future: impl std::future::Future<Output = bool> + 'a) -> Self::Output;
}

pub trait BehaviorActionState {
    fn make_future<'a, H>(&self, handler: H) -> H::Output
    where
        H: BehaviorFutureHandler<'a>;

    fn reset(&self);
}

pub trait IntoBehaviorActionState<R> {
    type State: BehaviorActionState;
    fn to_state(self, runner: &mut R) -> Self::State;
}

pub trait ActionName {
    fn action_name(&self) -> &'static str;
}

pub trait BehaviorObserver {
    /// Ids are assigned from 0 -> capacity
    ///
    /// When init is called we have [0..=capacity] nodes have status: `None`
    fn init(&self, capacity: usize);

    fn update(&self, id: usize, status: Option<Status>);
}

pub(crate) trait BehaviorTreeReset {
    fn reset(&mut self);
}

pub(crate) trait BehaviorObserverRunner {
    fn start(&mut self);
    fn end(&mut self, status: bool);
    fn reset(&mut self);
}

pub(crate) trait BehaviorObserverBuilder<A> {
    type Tree;
    type Observer: BehaviorObserverRunner;

    fn new_id(&mut self) -> usize;
    fn new_observer(&self, id: usize) -> Self::Observer;

    fn action(action: &A, id: usize) -> Self::Tree;
    fn invert(id: usize, child: Self::Tree) -> Self::Tree;
    fn sequence(id: usize, children: Vec<Self::Tree>) -> Self::Tree;
    fn select(id: usize, children: Vec<Self::Tree>) -> Self::Tree;
    fn subtree(name: std::rc::Rc<str>, id: usize, child: Self::Tree) -> Self::Tree;
}
