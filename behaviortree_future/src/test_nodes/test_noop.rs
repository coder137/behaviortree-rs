use crate::{BehaviorActionState, IntoBehaviorActionState};

impl IntoBehaviorActionState<()> for () {
    type State = ();

    fn to_state(self, _runner: &mut ()) -> Self::State {
        ()
    }
}

impl BehaviorActionState for () {
    type F = reusable_box_future::ReusableBoxFuture<bool>;

    fn create_future(&self) -> Self::F {
        Self::F::new(async move { true })
    }

    fn reset_future(&self, future: &mut Self::F) {
        future.set(async move { true });
    }
}
