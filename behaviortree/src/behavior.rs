/// Describes a behavior.
///
/// This is used for more complex event logic.
/// Can also be used for game AI.
#[derive(Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub enum Behavior<A> {
    /// A high level description of an action.
    Action(A),
    /// Waits an amount of time before continuing.
    ///
    /// f64: Time in seconds
    Wait(f64),

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

    /// Converts `Success` into `Failure` and vice versa.
    Invert(Box<Behavior<A>>),
    /// Ignores failures and returns `Success`.
    AlwaysSucceed(Box<Behavior<A>>),
}
