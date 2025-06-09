# behaviortree-rs

Contains `async_behaviortree` and `behaviortree` packages

`async_behaviortree` is actively maintained

# Class Diagram

```mermaid
classDiagram

class Behavior~A~ {
    <<enum>>
    Leaf
    Decorator
    Control
}

class ImmediateAction~S~ {
    <<trait>>
    fn run(&mut self, delta: f64, shared: &mut S) bool
    fn reset(&mut self, shared: &mut S)
    fn name(&self) &'static str
}

class AsyncAction~S~ {
    <<trait>>
    async fn run(&mut self, delta: &mut watch::Receiver~f64~, shared: &mut S) bool
    fn reset(&mut self, shared: &mut S)
    fn name(&self) &'static str
}

class AsyncActionType~S~ {
    <<enum>>
    Immediate
    Async

    async fn run(&mut self, delta: &mut watch::Receiver~f64~, shared: &mut S) bool
    fn reset(&mut self, shared: &mut S)
    fn name(&self) &'static str
}

class AsyncChild~S~ {
    <<struct>>
    AsyncActionType~S~ action_type
    watch::Sender~Option~Status~~ status
}

class AsyncBehaviorTree {
    <<struct>>
    AsyncChild~S~ child
}

Behavior --> ImmediateAction
Behavior --> AsyncAction

ImmediateAction --> AsyncActionType
AsyncAction --> AsyncActionType

AsyncActionType --> AsyncChild
AsyncChild --> AsyncBehaviorTree
```

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
    - [x] WhileAll
- [x] Tracing
  - [x] BehaviorTree
  - [x] AsyncBehaviorTree
- [ ] Examples
  - [ ] BehaviorTree
    - [x] Simple Immediate
    - [ ] Simple Sync
  - [ ] AsyncBehaviorTree
    - [x] Simple Immediate
    - [ ] Simple Async
