/// Describes a behavior.
///
/// This is used for more complex event logic.
/// Can also be used for game AI.
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Behavior<A> {
    /// A high level description of an action.
    Action(A),
    /// Waits an amount of time before continuing.
    ///
    /// f64: Time in milliseconds
    Wait(f64),

    /// Converts `Success` into `Failure` and vice versa.
    Invert(Box<Behavior<A>>),

    /// Runs behaviors one by one until all succeeded.
    ///
    /// The sequence fails if a behavior fails.
    /// The sequence succeeds if all the behavior succeeds.
    /// Can be thought of as a short-circuited logical AND gate.
    Sequence(Vec<Behavior<A>>),
    /// Runs behaviors one by one until a behavior succeeds.
    ///
    /// If a behavior fails it will try the next one.
    /// Fails if the last behavior fails.
    /// Can be thought of as a short-circuited logical OR gate.
    Select(Vec<Behavior<A>>),
    /// Runs behavior in a loop
    ///
    /// If behavior fails / succeeds, reset and restart the behavior
    #[deprecated]
    Loop(Box<Behavior<A>>),
    /// Run this behavior while all conditional actions are running / success
    /// Fails if any conditional action fails
    ///
    /// If the child behavior fails / succeeds, reset and restart the behavior
    /// If conditional action succeeds, reset and restart the behavior
    WhileAll(Vec<Behavior<A>>, Box<Behavior<A>>),
}
