use crate::Status;

pub(crate) trait BehaviorTreeReset {
    fn reset(&mut self);
}

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

pub trait IntoBehaviorActionState<AS, R>
where
    AS: BehaviorActionState,
{
    fn to_state(self, runner: &mut R) -> AS;
}

pub trait BehaviorObserver {
    /// Ids are assigned from 0 -> capacity
    ///
    /// When init is called we have [0..=capacity] nodes have status: `None`
    fn init(&self, capacity: usize);

    fn update(&self, id: usize, status: Option<Status>);
}

pub(crate) trait BehaviorObserverBuilder<A> {
    type Node;

    fn action(action: &A, id: usize) -> Self::Node;
    fn invert(id: usize, child: Self::Node) -> Self::Node;
    fn sequence(id: usize, children: Vec<Self::Node>) -> Self::Node;
    fn select(id: usize, children: Vec<Self::Node>) -> Self::Node;
    fn subtree(name: std::rc::Rc<str>, id: usize, child: Self::Node) -> Self::Node;
}

pub trait ActionName {
    fn action_name(&self) -> &'static str;
}

impl BehaviorObserver for () {
    #[inline(always)]
    fn init(&self, _capacity: usize) {}

    #[inline(always)]
    fn update(&self, _id: usize, _status: Option<Status>) {}
}
