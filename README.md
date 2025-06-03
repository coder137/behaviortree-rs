# behaviortree-rs

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

class SyncAction~S~ {
    <<trait>>
    fn tick(&mut self, delta: f64, shared: &mut S) Status
    fn reset(&mut self, shared: &mut S)
    fn name(&self) &'static str
}

class AsyncAction~S~ {
    <<trait>>
    async fn run(&mut self, delta: &mut watch::Receiver~f64~, shared: &mut S) bool
    fn reset(&mut self, shared: &mut S)
    fn name(&self) &'static str
}

class ActionType~S~ {
    <<enum>>
    Immediate
    Sync

    fn tick(&mut self, delta: f64, shared: &mut S) Status
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

class Child~S~ {
    <<struct>>
    ActionType~S~ action_type

    fn from_behavior~A~(Behavior~A~ behavior) Self where A: Into~ActionType~S~~
}

class AsyncChild~S~ {
    <<struct>>
    AsyncActionType~S~ action_type

    fn from_behavior~A~(Behavior~A~ behavior) Self where A: Into~AsyncActionType~S~~
}

class BehaviorTree {
    <<struct>>
    Child~S~ child

    new(Behavior~A~ behavior) BehaviorTree
    tick(&mut self) Status
    reset(&mut self)
}

class AsyncBehaviorTree {
    <<struct>>
    AsyncChild~S~ child
}

Behavior --> ImmediateAction
Behavior --> SyncAction
Behavior --> AsyncAction

SyncAction --> ActionType
ImmediateAction --> ActionType
ImmediateAction --> AsyncActionType
AsyncAction --> AsyncActionType

ActionType --> Child
Child --> BehaviorTree

AsyncActionType --> AsyncChild
AsyncChild --> AsyncBehaviorTree
```

# Roadmap

- [x] ImmediateAction trait
- [x] SyncAction trait
- [x] AsyncAction trait
- [ ] Behavior Nodes
  - [ ] Action
    - [x] Wait
  - [ ] Decorator
    - [x] Invert
    - [ ] ForceSuccess
    - [ ] ForceFailure
    - [ ] Repeat
    - [ ] RunTillSuccess
    - [ ] RunTillFailure
  - [ ] Control
    - [x] Sequence
    - [x] Select
    - [x] Loop
    - [x] WhileAll
    - [ ] WhileAny
    - [ ] Parallel
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
