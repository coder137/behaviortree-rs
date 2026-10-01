use crate::{
    AsyncAction, Behavior, BehaviorActionState, BehaviorTreeReset, Block, BlockType,
    IntoBehaviorActionState, TapeBuilder,
};

pub struct Tape<AS> {
    entry: usize,
    exit: usize,
    block_sections: Vec<Block>,
    action_sections: Vec<AsyncAction<AS>>,
}

// impl<AS> std::fmt::Debug for Tape<AS> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("Tape")
//             .field("entry", &self.entry)
//             .field("block_sections", &self.block_sections)
//             // .field("action_sections", &self.action_sections)
//             .finish()
//     }
// }

impl<AS> Tape<AS> {
    // pub fn new<A, R>(behavior: Behavior<A>, runner: &mut R) -> Self
    // where
    //     A: IntoBehaviorActionState<AS, R>,
    //     AS: BehaviorActionState,
    // {
    //     let mut tape_builder = TapeBuilder::new();
    //     let (start, end) = tape_builder.compile(behavior, runner);
    //     Self {
    //         entry: start,
    //         exit: end,
    //         block_sections: tape_builder.block_sections,
    //         action_sections: tape_builder.action_sections,
    //     }
    // }

    // pub fn to_future(self) -> TapeFuture<AS> {
    //     TapeFuture {
    //         entry: self.entry,
    //         exit: self.exit,
    //         block_sections: self.block_sections,
    //         action_sections: self.action_sections,
    //         pc: self.entry,
    //         current_status: false,
    //     }
    // }

    // pub async fn run(&mut self) -> bool
    // where
    //     AS: BehaviorActionState,
    // {
    //     let mut pc = self.entry;
    //     let mut current_status = false;

    //     let status = loop {
    //         let block = &self.block_sections[pc];
    //         println!("PC: {} RUN: {:?}", pc, block);
    //         match block.block_type {
    //             BlockType::Action { action_idx, next } => {
    //                 let action = &mut self.action_sections[action_idx];
    //                 current_status = action.await;
    //                 match next {
    //                     Some((on_success, on_failure)) => {
    //                         if current_status {
    //                             pc = on_success;
    //                         } else {
    //                             pc = on_failure;
    //                         }
    //                     }
    //                     None => {
    //                         break current_status;
    //                     }
    //                 }
    //             }
    //             BlockType::InvertStart { next } => {
    //                 pc = next;
    //             }
    //             BlockType::InvertEnd { next } => {
    //                 //
    //                 current_status = !current_status;
    //                 match next {
    //                     Some((on_success, on_failure)) => {
    //                         if current_status {
    //                             pc = on_success;
    //                         } else {
    //                             pc = on_failure;
    //                         }
    //                     }
    //                     None => {
    //                         break current_status;
    //                     }
    //                 }
    //             }
    //             BlockType::SequenceStart { next } => {
    //                 pc = next;
    //             }
    //             BlockType::SequenceEnd { next } => {
    //                 //
    //                 match next {
    //                     Some((on_success, on_failure)) => {
    //                         if current_status {
    //                             pc = on_success;
    //                         } else {
    //                             pc = on_failure;
    //                         }
    //                     }
    //                     None => {
    //                         break current_status;
    //                     }
    //                 }
    //             }
    //             BlockType::LoopStart { next } => {
    //                 pc = next;
    //             }
    //             BlockType::LoopEnd { next } => {
    //                 self.reset_from(next, pc);
    //                 pc = next;
    //             }
    //         }
    //     };
    //     status
    // }

    // fn reset_from(&mut self, start: usize, end: usize)
    // where
    //     AS: BehaviorActionState,
    // {
    //     let mut pc = start;
    //     loop {
    //         let block = &self.block_sections[pc];
    //         println!("PC: {} RESET: {:?}", pc, block);
    //         match block.block_type {
    //             BlockType::Action {
    //                 action_idx,
    //                 next: _,
    //             } => {
    //                 self.action_sections[action_idx].reset();
    //             }
    //             BlockType::InvertStart { next } => {}
    //             BlockType::InvertEnd { next } => {}
    //             BlockType::SequenceStart { next } => {}
    //             BlockType::SequenceEnd { next } => {}
    //             BlockType::LoopStart { next } => {}
    //             BlockType::LoopEnd { next } => {}
    //         }
    //         if pc == end {
    //             break;
    //         }
    //         pc = pc + 1;
    //     }
    // }
}

pub struct TapeFuture<AS> {
    entry: usize,
    exit: usize,
    block_sections: Vec<Block>,
    action_sections: Vec<AsyncAction<AS>>,

    // state
    pc: usize,
    current_status: bool,
}

impl<AS> TapeFuture<AS>
where
    AS: BehaviorActionState,
{
    pub fn new<A, R>(behavior: Behavior<A>, runner: &mut R) -> Self
    where
        A: IntoBehaviorActionState<AS, R>,
        AS: BehaviorActionState,
    {
        let mut tape_builder = TapeBuilder::new();
        let (start, end) = tape_builder.compile(behavior, runner);
        Self {
            entry: start,
            exit: end,
            block_sections: tape_builder.block_sections,
            action_sections: tape_builder.action_sections,
            pc: start,
            current_status: false,
        }
    }

    pub fn reset(&mut self) {
        let mut pc = self.entry;
        loop {
            let block = &self.block_sections[pc];
            println!("PC: {} RESET: {:?}", pc, block);
            match block.block_type {
                BlockType::Action {
                    action_idx,
                    next: _,
                } => {
                    self.action_sections[action_idx].reset();
                }
                BlockType::InvertStart { next } => {}
                BlockType::InvertEnd { next } => {}
                BlockType::SequenceStart { next } => {}
                BlockType::SequenceEnd { next } => {}
                BlockType::LoopStart { next } => {}
                BlockType::LoopEnd { next } => {}
            }
            if pc == self.exit {
                break;
            }
            pc = pc + 1;
        }

        // Reset state
        self.pc = self.entry;
        self.current_status = false;
    }

    fn reset_from(&mut self, start: usize, end: usize) {
        let mut pc = start;
        loop {
            let block = &self.block_sections[pc];
            println!("PC: {} RESET: {:?}", pc, block);
            match block.block_type {
                BlockType::Action {
                    action_idx,
                    next: _,
                } => {
                    self.action_sections[action_idx].reset();
                }
                BlockType::InvertStart { next } => {}
                BlockType::InvertEnd { next } => {}
                BlockType::SequenceStart { next } => {}
                BlockType::SequenceEnd { next } => {}
                BlockType::LoopStart { next } => {}
                BlockType::LoopEnd { next } => {}
            }
            if pc == end {
                break;
            }
            pc = pc + 1;
        }
    }
}

impl<AS> TapeFuture<AS> {
    pub fn to_flat_graph(&self) -> petgraph::graph::DiGraph<String, &str> {
        let mut nodes = std::collections::HashMap::new();
        let mut graph = petgraph::graph::DiGraph::<_, _>::new();
        for (index, block) in self.block_sections.iter().enumerate() {
            match block.block_type {
                BlockType::Action { action_idx, next } => {
                    let action = &self.action_sections[action_idx];
                    // let name = action.name();
                    let name = "Action";
                    let node_index = graph.add_node(name.to_string());
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

impl<AS> std::future::Future for TapeFuture<AS>
where
    AS: BehaviorActionState,
{
    type Output = bool;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        loop {
            let block = &self.block_sections[self.pc];
            println!("PC: {} RUN: {:?}", self.pc, block);
            match block.block_type {
                BlockType::Action { action_idx, next } => {
                    let action = &mut self.action_sections[action_idx];
                    match std::pin::Pin::new(action).poll(cx) {
                        std::task::Poll::Ready(status) => {
                            self.current_status = status;
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
                    self.pc = next;
                }
                BlockType::InvertEnd { next } => {
                    //
                    self.current_status = !self.current_status;
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
                    self.pc = next;
                }
                BlockType::SequenceEnd { next } => {
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
                    self.pc = next;
                }
                BlockType::LoopEnd { next } => {
                    let current = self.pc;
                    self.reset_from(next, current);
                    self.pc = next;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_nodes::{TestOperation, TestOperationRunner, TestOperationState};
    use ticked_async_executor::TickedAsyncExecutor;

    use super::*;

    #[tokio::test]
    async fn test_action() {
        let behavior = Behavior::Action(TestOperation::Add(1, 2, true, 0));

        let mut runner = TestOperationRunner::new(0);
        let mut tape = TapeFuture::<TestOperationState>::new(behavior, &mut runner);

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
        let tape = TapeFuture::<TestOperationState>::new(behavior, &mut runner);

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
        let mut tape = TapeFuture::<TestOperationState>::new(behavior, &mut runner);

        let graph = tape.to_flat_graph();
        let dot = petgraph::dot::Dot::with_config(&graph, &[]);
        println!("{}", dot);

        let status = tape.await;
        println!("Status: {status}");
    }

    #[test]
    fn test_invert_sequence() {
        let behavior = Behavior::Sequence(vec![
            Behavior::Action(TestOperation::Add(1, 2, true, 1)),
            Behavior::Action(TestOperation::Add(3, 4, false, 1)),
            Behavior::Action(TestOperation::Add(5, 6, true, 1)),
        ]);
        let behavior = Behavior::Invert(behavior.into());
        let behavior = Behavior::Loop(behavior.into());

        let mut runner = TestOperationRunner::new(0);
        let mut tape = TapeFuture::<TestOperationState>::new(behavior, &mut runner);

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
            println!("BEFORE: {}", i);
            executor.tick(1.0, None);
            println!("AFTER: {}", i);
        }
    }
}
