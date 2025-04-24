# behaviortree-rs

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
```

# Roadmap

- [x] Action trait
  - [ ] Rename from `Action` to `SyncAction`
- [x] AsyncAction trait
- [x] ImmediateAction trait
- [ ] Unification
  - [ ] Remove workspace and keep only 1 crate
  - [ ] Unify `SyncAction` with `ImmediateAction`
  - [ ] Unify `AsyncAction` with `ImmediateAction`
- [ ] Behavior Nodes
  - [x] Wait
  - [x] Invert
  - [x] Sequence
  - [x] Select
