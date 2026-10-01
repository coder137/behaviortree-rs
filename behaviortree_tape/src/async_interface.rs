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

pub trait BehaviorObserver<AS> {
    fn action_name(action_state: &AS) -> &'static str;

    /// Ids are assigned from 0 -> capacity
    ///
    /// When init is called we have [0..=capacity] nodes have status: `None`
    fn init(&self, capacity: usize);

    fn update(&self, id: usize, status: Option<Status>);
}
