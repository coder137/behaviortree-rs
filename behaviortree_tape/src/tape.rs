use crate::{
    ActionName, AsyncAction, BehaviorActionState, BehaviorObserver, BehaviorObserverTree,
    BehaviorTreeReset, Block, BlockType, DefaultObserver, IntoBehaviorActionState, NoObserver,
    RootBehavior, Status, TapeBuilder,
};

pub struct Tape<AS, O> {
    entry: usize,
    exit: usize,
    block_sections: Vec<Block>,
    action_sections: Vec<AsyncAction<AS>>,

    observer: O,

    // state
    pc: usize,
    action_started: bool,
    current_status: bool,
    loop_end_yield: bool,
}

impl<AS> Tape<AS, ()> {
    pub fn new<A, R>(root_behavior: RootBehavior<A>, runner: &mut R) -> Self
    where
        A: IntoBehaviorActionState<AS, R>,
        AS: BehaviorActionState,
    {
        let mut tape_builder = TapeBuilder::new();
        let (start, end, _observertree) =
            tape_builder.root_compile::<A, R, NoObserver>(root_behavior, runner);
        Self {
            entry: start,
            exit: end,
            block_sections: tape_builder.block_sections,
            action_sections: tape_builder.action_sections,
            observer: (),
            pc: start,
            action_started: false,
            current_status: true,
            loop_end_yield: true,
        }
    }
}

impl<AS, O> Tape<AS, O>
where
    AS: BehaviorActionState,
{
    pub fn new_with_observer<A, R>(
        root_behavior: RootBehavior<A>,
        runner: &mut R,
        observer: O,
    ) -> (Self, BehaviorObserverTree)
    where
        A: IntoBehaviorActionState<AS, R> + ActionName,
        AS: BehaviorActionState,
    {
        let mut tape_builder = TapeBuilder::new();
        let (start, end, observer_tree) =
            tape_builder.root_compile::<A, R, DefaultObserver>(root_behavior, runner);
        let this = Self {
            entry: start,
            exit: end,
            block_sections: tape_builder.block_sections,
            action_sections: tape_builder.action_sections,
            observer,
            pc: start,
            action_started: false,
            current_status: true,
            loop_end_yield: true,
        };
        (this, observer_tree)
    }

    pub fn reset(&mut self) {
        self.reset_blocks_from(self.entry, self.exit);
        // Reset state
        self.pc = self.entry;
        self.action_started = false;
        self.current_status = true;
        self.loop_end_yield = true;
    }

    fn reset_blocks_from(&mut self, start: usize, end: usize) {
        let mut pc = start;
        loop {
            let block = &self.block_sections[pc];
            // println!("PC: {} RESET: {:?}", pc, block);
            match block.block_type {
                BlockType::Action {
                    action_idx,
                    next: _,
                } => {
                    self.action_sections[action_idx].reset();
                }
                _ => {}
            }
            if pc == end {
                break;
            }
            pc = pc + 1;
        }
    }
}

impl<AS, O> Tape<AS, O> {
    pub fn to_flat_graph(&self) -> petgraph::graph::DiGraph<String, &str>
    where
        AS: std::fmt::Debug,
    {
        let mut nodes = std::collections::HashMap::new();
        let mut graph = petgraph::graph::DiGraph::<_, _>::new();
        for (index, block) in self.block_sections.iter().enumerate() {
            match block.block_type {
                BlockType::Action { action_idx, next } => {
                    let action = &self.action_sections[action_idx];
                    let node_index = graph.add_node(action.debug());
                    nodes.insert(index, node_index);
                }
                BlockType::InvertStart { next } => {
                    let node_index = graph.add_node("InvertStart".to_string());
                    nodes.insert(index, node_index);
                }
                BlockType::InvertEnd { next } => {
                    let node_index = graph.add_node("InvertEnd".to_string());
                    nodes.insert(index, node_index);
                }
                BlockType::SequenceStart { next } => {
                    let node_index = graph.add_node("SequenceStart".to_string());
                    nodes.insert(index, node_index);
                }
                BlockType::SequenceEnd { next } => {
                    let node_index = graph.add_node("SequenceEnd".to_string());
                    nodes.insert(index, node_index);
                }
                BlockType::SelectStart { next } => {
                    let node_index = graph.add_node("SelectStart".to_string());
                    nodes.insert(index, node_index);
                }
                BlockType::SelectEnd { next } => {
                    let node_index = graph.add_node("SelectEnd".to_string());
                    nodes.insert(index, node_index);
                }
                BlockType::LoopStart { next } => {
                    let node_index = graph.add_node("LoopStart".to_string());
                    nodes.insert(index, node_index);
                }
                BlockType::LoopEnd { next } => {
                    let node_index = graph.add_node("LoopEnd".to_string());
                    nodes.insert(index, node_index);
                }
            }
        }

        for (index, block) in self.block_sections.iter().enumerate() {
            match block.block_type {
                BlockType::Action { action_idx, next } => {
                    if let Some(next) = next {
                        let (on_success_index, on_failure_index) = next;
                        let this_node_index = nodes[&index];
                        let on_success_node_index = nodes[&on_success_index];
                        let on_failure_node_index = nodes[&on_failure_index];
                        if on_success_index == on_failure_index {
                            graph.add_edge(
                                this_node_index,
                                on_success_node_index,
                                "success/failure",
                            );
                        } else {
                            graph.add_edge(this_node_index, on_success_node_index, "success");
                            graph.add_edge(this_node_index, on_failure_node_index, "failure");
                        }
                    }
                }
                BlockType::InvertStart { next } => {
                    let this_node_index = nodes[&index];
                    let next_node_index = nodes[&next];
                    graph.add_edge(this_node_index, next_node_index, "");
                }
                BlockType::InvertEnd { next } => {
                    //
                    if let Some(next) = next {
                        let (on_success_index, on_failure_index) = next;
                        let this_node_index = nodes[&index];
                        let on_success_node_index = nodes[&on_success_index];
                        let on_failure_node_index = nodes[&on_failure_index];
                        if on_success_index == on_failure_index {
                            graph.add_edge(
                                this_node_index,
                                on_success_node_index,
                                "success/failure",
                            );
                        } else {
                            graph.add_edge(this_node_index, on_success_node_index, "success");
                            graph.add_edge(this_node_index, on_failure_node_index, "failure");
                        }
                    }
                }
                BlockType::SequenceStart { next } => {
                    let this_node_index = nodes[&index];
                    let next_node_index = nodes[&next];
                    graph.add_edge(this_node_index, next_node_index, "");
                }
                BlockType::SequenceEnd { next } => {
                    //
                    if let Some(next) = next {
                        let (on_success_index, on_failure_index) = next;
                        let this_node_index = nodes[&index];
                        let on_success_node_index = nodes[&on_success_index];
                        let on_failure_node_index = nodes[&on_failure_index];
                        if on_success_index == on_failure_index {
                            graph.add_edge(
                                this_node_index,
                                on_success_node_index,
                                "success/failure",
                            );
                        } else {
                            graph.add_edge(this_node_index, on_success_node_index, "success");
                            graph.add_edge(this_node_index, on_failure_node_index, "failure");
                        }
                    }
                }
                BlockType::SelectStart { next } => {
                    let this_node_index = nodes[&index];
                    let next_node_index = nodes[&next];
                    graph.add_edge(this_node_index, next_node_index, "");
                }
                BlockType::SelectEnd { next } => {
                    //
                    if let Some(next) = next {
                        let (on_success_index, on_failure_index) = next;
                        let this_node_index = nodes[&index];
                        let on_success_node_index = nodes[&on_success_index];
                        let on_failure_node_index = nodes[&on_failure_index];
                        if on_success_index == on_failure_index {
                            graph.add_edge(
                                this_node_index,
                                on_success_node_index,
                                "success/failure",
                            );
                        } else {
                            graph.add_edge(this_node_index, on_success_node_index, "success");
                            graph.add_edge(this_node_index, on_failure_node_index, "failure");
                        }
                    }
                }
                BlockType::LoopStart { next } => {
                    let this_node_index = nodes[&index];
                    let next_node_index = nodes[&next];
                    graph.add_edge(this_node_index, next_node_index, "");
                }
                BlockType::LoopEnd { next } => {
                    let this_node_index = nodes[&index];
                    let next_node_index = nodes[&next];
                    graph.add_edge(this_node_index, next_node_index, "");
                }
            }
        }

        graph
    }
}

impl<AS, O> std::future::Future for Tape<AS, O>
where
    AS: BehaviorActionState,
    O: BehaviorObserver + Unpin,
{
    type Output = bool;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        loop {
            let block = self.block_sections[self.pc];
            // println!("PC: {} RUN: {:?}", self.pc, block);
            match block.block_type {
                BlockType::Action { action_idx, next } => {
                    if !self.action_started {
                        self.action_started = true;
                        self.observer.update(block.block_id, Some(Status::Running));
                    }
                    match std::pin::Pin::new(&mut self.action_sections[action_idx]).poll(cx) {
                        std::task::Poll::Ready(status) => {
                            self.current_status = status;
                            //
                            self.action_started = false;
                            self.observer
                                .update(block.block_id, Some(Status::from(self.current_status)));
                            //
                            match next {
                                Some((on_success, on_failure)) => {
                                    if self.current_status {
                                        self.pc = on_success;
                                    } else {
                                        self.pc = on_failure;
                                    }
                                }
                                None => {
                                    break std::task::Poll::Ready(self.current_status);
                                }
                            }
                        }
                        std::task::Poll::Pending => {
                            break std::task::Poll::Pending;
                        }
                    }
                }
                BlockType::InvertStart { next } => {
                    self.observer.update(block.block_id, Some(Status::Running));
                    self.pc = next;
                }
                BlockType::InvertEnd { next } => {
                    //
                    self.current_status = !self.current_status;
                    //
                    self.observer
                        .update(block.block_id, Some(Status::from(self.current_status)));
                    //
                    match next {
                        Some((on_success, on_failure)) => {
                            if self.current_status {
                                self.pc = on_success;
                            } else {
                                self.pc = on_failure;
                            }
                        }
                        None => {
                            break std::task::Poll::Ready(self.current_status);
                        }
                    }
                }
                BlockType::SequenceStart { next } => {
                    self.observer.update(block.block_id, Some(Status::Running));
                    self.pc = next;
                }
                BlockType::SequenceEnd { next } => {
                    //
                    self.observer
                        .update(block.block_id, Some(Status::from(self.current_status)));
                    //
                    match next {
                        Some((on_success, on_failure)) => {
                            if self.current_status {
                                self.pc = on_success;
                            } else {
                                self.pc = on_failure;
                            }
                        }
                        None => {
                            break std::task::Poll::Ready(self.current_status);
                        }
                    }
                }
                BlockType::SelectStart { next } => {
                    self.observer.update(block.block_id, Some(Status::Running));
                    self.pc = next;
                }
                BlockType::SelectEnd { next } => {
                    //
                    self.observer
                        .update(block.block_id, Some(Status::from(self.current_status)));
                    //
                    match next {
                        Some((on_success, on_failure)) => {
                            if self.current_status {
                                self.pc = on_success;
                            } else {
                                self.pc = on_failure;
                            }
                        }
                        None => {
                            break std::task::Poll::Ready(self.current_status);
                        }
                    }
                }
                BlockType::LoopStart { next } => {
                    self.observer.update(block.block_id, Some(Status::Running));
                    self.pc = next;
                }
                BlockType::LoopEnd { next } => {
                    if self.loop_end_yield {
                        self.loop_end_yield = false;
                        cx.waker().wake_by_ref();
                        break std::task::Poll::Pending;
                    } else {
                        self.loop_end_yield = true;
                        self.observer
                            .update(block.block_id, Some(Status::from(self.current_status)));
                        let current = self.pc;
                        self.reset_blocks_from(next, current);
                        self.pc = next;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Behavior,
        test_nodes::{
            TestOperation, TestOperationObserver, TestOperationRunner, TestOperationState,
        },
    };
    use ticked_async_executor::TickedAsyncExecutor;

    use super::*;

    #[tokio::test]
    async fn test_action() {
        let behavior = Behavior::Action(TestOperation::Add(1, 2, true, 0));

        let mut runner = TestOperationRunner::new(0);
        let tape = Tape::<TestOperationState, ()>::new(RootBehavior::Once(behavior), &mut runner);

        let graph = tape.to_flat_graph();
        let dot = petgraph::dot::Dot::with_config(&graph, &[]);
        println!("{}", dot);

        let status = tape.await;
        println!("Status: {status}");
    }

    #[tokio::test]
    async fn test_invert() {
        let behavior = Behavior::Action(TestOperation::Add(1, 2, true, 0));
        let behavior = Behavior::Invert(behavior.into());

        let mut runner = TestOperationRunner::new(0);
        let tape = Tape::<TestOperationState, ()>::new(RootBehavior::Once(behavior), &mut runner);

        let graph = tape.to_flat_graph();
        let dot = petgraph::dot::Dot::with_config(&graph, &[]);
        println!("{}", dot);

        let status = tape.await;
        println!("Status: {status}");
    }

    #[tokio::test]
    async fn test_invert_invert() {
        let behavior = Behavior::Action(TestOperation::Add(1, 2, true, 0));
        let behavior = Behavior::Invert(behavior.into());
        let behavior = Behavior::Invert(behavior.into());
        // let behavior = Behavior::Invert(behavior.into());

        let mut runner = TestOperationRunner::new(0);
        let tape = Tape::<TestOperationState, ()>::new(RootBehavior::Once(behavior), &mut runner);

        let graph = tape.to_flat_graph();
        let dot = petgraph::dot::Dot::with_config(&graph, &[]);
        println!("{}", dot);

        let status = tape.await;
        println!("Status: {status}");
    }

    #[test]
    fn test_invert_sequence() {
        let behavior = Behavior::Select(vec![
            Behavior::Action(TestOperation::Add(1, 2, false, 1)),
            Behavior::Action(TestOperation::Add(3, 4, false, 1)),
            Behavior::Action(TestOperation::Add(5, 6, false, 1)),
        ]);
        let behavior = Behavior::Invert(behavior.into());

        let mut runner = TestOperationRunner::new(0);

        let observer = TestOperationObserver {};
        let (tape, observer_tree) =
            Tape::<TestOperationState, TestOperationObserver>::new_with_observer(
                RootBehavior::Once(behavior),
                &mut runner,
                observer,
            );
        println!("Observer Tree: {:?}", observer_tree);

        //
        let graph = tape.to_flat_graph();
        let dot = petgraph::dot::Dot::with_config(&graph, &[]);
        println!("{}", dot);

        let mut executor = TickedAsyncExecutor::default();
        executor
            .spawn_local((), async move {
                let status = tape.await;
                println!("Status: {status}");
            })
            .detach();

        for i in 0..10 {
            // println!("BEFORE: {}", i);
            executor.tick(1.0, None);
            // println!("AFTER: {}", i);
        }
    }
}
