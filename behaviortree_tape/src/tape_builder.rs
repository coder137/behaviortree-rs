use crate::{
    AsyncAction, Behavior, BehaviorActionState, BehaviorObserverBuilder, IntoBehaviorActionState,
};

#[derive(Debug, Clone, Copy)]
pub enum BlockType {
    Action {
        action_idx: usize,
        next: Option<(usize, usize)>,
    },
    InvertStart {
        next: usize,
    },
    InvertEnd {
        next: Option<(usize, usize)>,
    },
    SequenceStart {
        next: usize,
    },
    SequenceEnd {
        next: Option<(usize, usize)>,
    },
    LoopStart {
        next: usize,
    },
    LoopEnd {
        next: usize,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Block {
    pub block_id: usize,
    pub block_type: BlockType,
}

pub struct TapeBuilder<AS> {
    pub block_sections: Vec<Block>,
    pub action_sections: Vec<AsyncAction<AS>>,
    pub block_alloc_id: usize,
}

impl<AS> TapeBuilder<AS> {
    pub fn new() -> Self {
        TapeBuilder {
            block_sections: vec![],
            action_sections: vec![],
            block_alloc_id: 0,
        }
    }

    pub(crate) fn compile<A, R, O: BehaviorObserverBuilder<A>>(
        &mut self,
        behavior: Behavior<A>,
        runner: &mut R,
    ) -> (usize, usize, O::Node)
    where
        A: IntoBehaviorActionState<AS, R>,
        AS: BehaviorActionState,
    {
        match behavior {
            Behavior::Action(action) => {
                let block_id = self.reserve_block_id();
                let observer_node = O::action(&action, block_id);

                // ACTION
                let action_state = action.to_state(runner);
                let async_action = AsyncAction::new(action_state);
                let action_idx = self.action_sections.len();
                self.action_sections.push(async_action);

                // ACTION BLOCK
                let action_block_idx = self.reserve_block(block_id);
                self.block_sections[action_block_idx].block_type = BlockType::Action {
                    action_idx,
                    next: None,
                };

                (action_block_idx, action_block_idx, observer_node)
            }
            Behavior::Invert(behavior) => {
                // CREATE
                let block_id = self.reserve_block_id();
                let invert_start_idx = self.reserve_block(block_id);
                let (start, end, child_observer_node) = self.compile::<A, R, O>(*behavior, runner);
                let invert_end_idx = self.reserve_block(block_id);

                // UPDATE
                self.block_sections[invert_start_idx].block_type =
                    BlockType::InvertStart { next: start };
                self.update_end_block(end, (invert_end_idx, invert_end_idx));
                self.block_sections[invert_end_idx].block_type =
                    BlockType::InvertEnd { next: None };

                let observer_node = O::invert(block_id, child_observer_node);
                (invert_start_idx, invert_end_idx, observer_node)
            }
            Behavior::Sequence(behaviors) => {
                // CREATE
                let block_id = self.reserve_block_id();
                let sequence_start_idx = self.reserve_block(block_id);
                let child_idxs = behaviors
                    .into_iter()
                    .map(|behavior| self.compile::<A, R, O>(behavior, runner))
                    .collect::<Vec<_>>();
                let sequence_end_idx = self.reserve_block(block_id);

                // UPDATE
                self.block_sections[sequence_start_idx].block_type = BlockType::SequenceStart {
                    next: child_idxs[0].0,
                };

                for s in child_idxs.windows(2) {
                    self.update_end_block(s[0].1, (s[1].0, sequence_end_idx));
                }

                let last_idx = child_idxs.len() - 1;
                self.update_end_block(child_idxs[last_idx].1, (sequence_end_idx, sequence_end_idx));
                self.block_sections[sequence_end_idx].block_type =
                    BlockType::SequenceEnd { next: None };

                let children_observer_nodes = child_idxs
                    .into_iter()
                    .map(|(_, _, child_observer_node)| child_observer_node)
                    .collect::<Vec<_>>();
                let observer_node = O::sequence(block_id, children_observer_nodes);
                (sequence_start_idx, sequence_end_idx, observer_node)
            }
            Behavior::Select(behaviors) => todo!(),
            Behavior::Loop(behavior) => {
                // CREATE
                let block_id = self.reserve_block_id();
                let loop_start_idx = self.reserve_block(block_id);
                let (start, end, child_observer_node) = self.compile::<A, R, O>(*behavior, runner);
                let loop_end_idx = self.reserve_block(block_id);

                // UPDATE
                self.block_sections[loop_start_idx].block_type =
                    BlockType::LoopStart { next: start };
                self.update_end_block(end, (loop_end_idx, loop_end_idx));
                self.block_sections[loop_end_idx].block_type = BlockType::LoopEnd {
                    next: loop_start_idx,
                };

                let observer_node = O::r#loop(block_id, child_observer_node);
                (loop_start_idx, loop_end_idx, observer_node)
            }
            Behavior::Subtree(_, behavior) => todo!(),
        }
    }

    fn reserve_block_id(&mut self) -> usize {
        let block_id = self.block_alloc_id;
        self.block_alloc_id += 1;
        block_id
    }

    fn reserve_block(&mut self, block_id: usize) -> usize {
        let index = self.block_sections.len();
        self.block_sections.push(Block {
            block_id,
            block_type: BlockType::Action {
                action_idx: usize::MAX,
                next: None,
            },
        });
        index
    }

    fn update_end_block(&mut self, block_idx: usize, to: (usize, usize)) {
        let block_type = &mut self.block_sections[block_idx].block_type;
        match block_type {
            BlockType::Action {
                action_idx: _,
                next,
            } => {
                *next = Some(to);
            }
            BlockType::InvertEnd { next } => {
                *next = Some(to);
            }
            BlockType::SequenceEnd { next } => {
                *next = Some(to);
            }
            _ => unreachable!("Should be unreachable for block_type: {:?}", block_type),
        }
    }
}
