pub trait ImmediateAction<S> {
    /// Runs the action in a single tick
    ///
    /// Cannot return `Status::Running`
    /// true == `Status::Success`
    /// false == `Status::Failure`
    fn run(&mut self, delta: f64, shared: &mut S) -> bool;

    /// Resets the current action to its initial/newly created state
    fn reset(&mut self, shared: &mut S);

    /// Identify your action
    fn name(&self) -> &'static str;
}
