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
    async fn run(&mut self, delta: &mut watch::Receiver<f64>, shared: &mut S) bool
    fn reset(&mut self, shared: &mut S)
    fn name(&self) &'static str
}

class ActionType~S~ {
    <<enum>>
    Immediate
    Sync
    Async
}

class Child~S~ {
    <<struct>>
    ActionType~S~ action_type
    tokio::sync::watch::Sender~Option~Status~~ status

    fn from_behavior~A~(Behavior~A~ behavior) Self where A: Into~ActionType~S~~
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
ImmediateAction --> ActionType
SyncAction --> ActionType
AsyncAction --> ActionType
ActionType --> Child
Child --> BehaviorTree
```

# Roadmap

- [x] Action trait
  - [ ] Rename from `Action` to `SyncAction`
- [x] AsyncAction trait
- [x] ImmediateAction trait
- [ ] Unify `ImmediateAction`, `SyncAction` and `AsyncAction`
- [ ] Behavior Nodes
  - [x] Wait
  - [x] Invert
  - [x] Sequence
  - [x] Select
