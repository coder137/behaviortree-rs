use crate::{AsyncAction, Behavior, BehaviorActionState, IntoBehaviorActionState, Tape};

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

#[derive(Debug)]
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

    pub fn compile<A, R>(&mut self, behavior: Behavior<A>, runner: &mut R) -> (usize, usize)
    where
        A: IntoBehaviorActionState<AS, R>,
        AS: BehaviorActionState,
    {
        match behavior {
            Behavior::Action(action) => {
                // ACTION
                let action_state = action.to_state(runner);
                let async_action = AsyncAction::new(action_state);
                let action_idx = self.action_sections.len();
                self.action_sections.push(async_action);

                // ACTION BLOCK
                let block_id = self.reserve_block_id();
                let action_block_idx = self.reserve_block(block_id);
                self.block_sections[action_block_idx].block_type = BlockType::Action {
                    action_idx,
                    next: None,
                };
                (action_block_idx, action_block_idx)
            }
            Behavior::Invert(behavior) => {
                // CREATE
                let block_id = self.reserve_block_id();
                let invert_start_idx = self.reserve_block(block_id);
                let (start, end) = self.compile(*behavior, runner);
                let invert_end_idx = self.reserve_block(block_id);

                // UPDATE
                self.block_sections[invert_start_idx].block_type =
                    BlockType::InvertStart { next: start };
                self.update_end_block(end, (invert_end_idx, invert_end_idx));
                self.block_sections[invert_end_idx].block_type =
                    BlockType::InvertEnd { next: None };
                (invert_start_idx, invert_end_idx)
            }
            Behavior::Sequence(behaviors) => {
                // CREATE
                let block_id = self.reserve_block_id();
                let sequence_start_idx = self.reserve_block(block_id);
                let child_idxs = behaviors
                    .into_iter()
                    .map(|behavior| self.compile(behavior, runner))
                    .collect::<Vec<_>>();
                let sequence_end_idx = self.reserve_block(block_id);

                // UPDATE
                let first_idx = 0;
                let first = child_idxs[first_idx];
                self.block_sections[sequence_start_idx].block_type =
                    BlockType::SequenceStart { next: first.0 };

                for s in child_idxs.windows(2) {
                    let current = s[0];
                    let next = s[1];
                    self.update_end_block(current.1, (next.0, sequence_end_idx));
                }

                let last_idx = child_idxs.len() - 1;
                let last = child_idxs[last_idx];
                self.update_end_block(last.1, (sequence_end_idx, sequence_end_idx));
                self.block_sections[sequence_end_idx].block_type =
                    BlockType::SequenceEnd { next: None };
                (sequence_start_idx, sequence_end_idx)
            }
            Behavior::Select(behaviors) => todo!(),
            Behavior::Loop(behavior) => {
                // CREATE
                let block_id = self.reserve_block_id();
                let loop_start_idx = self.reserve_block(block_id);
                let (start, end) = self.compile(*behavior, runner);
                let loop_end_idx = self.reserve_block(block_id);

                // UPDATE
                self.block_sections[loop_start_idx].block_type =
                    BlockType::LoopStart { next: start };
                self.update_end_block(end, (loop_end_idx, loop_end_idx));
                self.block_sections[loop_end_idx].block_type = BlockType::LoopEnd {
                    next: loop_start_idx,
                };
                (loop_start_idx, loop_end_idx)
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
