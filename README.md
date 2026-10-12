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
    - Memory allocation is not contiguous
    - Recursive implementation (can cause stack overflow for very deep trees)
- `behaviortree_tape`
  - Improvements:
    - Improvement over `behaviortree_future`: Memory allocation is contiguous and traversal is linear (vector)
  - Flaws:
    - Redundant blocks for observer profiling causes performance impacts
    - Proof of concept (use `behaviortree_future` for now)

# Roadmap

- Behavior Nodes
  - [x] Action
  - Decorator
    - [x] Invert
    - [ ] ForceSuccess
    - [ ] ForceFailure
  - Control
    - [x] Sequence
    - [x] Select
    - [ ] Times
    - [ ] ReactiveSequence
    - [ ] ReactiveSelect
