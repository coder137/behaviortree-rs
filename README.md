# behaviortree-rs

Experimental codebase that contains the following packages

Different strategies for running behavior trees

- `behaviortree`
  - Flaws:
    - Ticks actions even when they return `Status::Pending` (need a more `Future` like API)
- `async_behaviortree`
  - Flaws:
    - Uses dynamic memory allocation during runtime (`Box::pin`)
    - Frequent dynamic memory allocations causing fragmentation
    - Memory allocation is not contiguous (needs arena allocation)

# Roadmap

- [x] ImmediateAction trait
- [x] AsyncAction trait
- [ ] Behavior Nodes
  - [ ] Action
    - [x] Wait
  - [ ] Decorator
    - [x] Invert
    - [ ] ForceSuccess
    - [ ] ForceFailure
  - [ ] Control
    - [x] Sequence
    - [x] Select
