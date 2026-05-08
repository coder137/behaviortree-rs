# behaviortree-rs

Experimental codebase that contains the following packages

Different strategies for running behavior trees

- `behaviortree`
  - Flaws: 
    - Ticks actions even when they return Status::Pending
- `async_behaviortree`
  - Flaws: 
    - Uses dynamic memory allocation during runtime (`Box::pin`)
    - Frequent dynamic memory allocations causing fragmentation
    - Memory allocation is not contiguous (needs arena allocation)
- `behaviortree_future`
  - Improvements:
    - Improvement over `behaviortree`: Wakes the parent only after action future wakes up
    - Improvement over `async_behaviortree`: Dynamic memory allocation during creation / reset via `reusable-box-future`, but not during runtime.
  - Flaws:
    - Memory allocation is not contiguous (needs arena allocation)
    - Needs Runner and Action (ex: `Behavior<Action>`) to be `Clone`

# Roadmap

- [ ] Behavior Nodes
  - [ ] Action
  - [ ] Decorator
    - [x] Invert
    - [ ] ForceSuccess
    - [ ] ForceFailure
  - [ ] Control
    - [x] Sequence
    - [x] Select
    - [ ] ReactiveSequence
    - [ ] ReactiveSelect
