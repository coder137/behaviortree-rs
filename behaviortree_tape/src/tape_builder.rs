use crate::{
    Behavior, BehaviorActionState, BehaviorObserverBuilder, IntoBehaviorActionState, RootBehavior,
    async_action::AsyncAction,
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
    SelectStart {
        next: usize,
    },
    SelectEnd {
        next: Option<(usize, usize)>,
    },
    SubtreeStart {
        next: usize,
    },
    SubtreeEnd {
        next: Option<(usize, usize)>,
    },
    // TODO, can combine both of these into 1 block
    Yield {
        next: usize,
    },
    Reset {
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

    pub(crate) fn root_compile<A, R, O: BehaviorObserverBuilder<A>>(
        &mut self,
        root_behavior: RootBehavior<A>,
        runner: &mut R,
    ) -> (usize, usize, O::Node)
    where
        A: IntoBehaviorActionState<AS, R>,
        AS: BehaviorActionState,
    {
        match root_behavior {
            RootBehavior::Once(behavior) => {
                let (start, end, observer_node) = self.compile::<A, R, O>(behavior, runner);
                (start, end, observer_node)
            }
            RootBehavior::Loop(behavior) => {
                let block_id = self.reserve_block_id();
                let (start, end, child_observer_node) = self.compile::<A, R, O>(behavior, runner);
                let yield_block_idx = self.reserve_block(block_id);
                let reset_block_idx = self.reserve_block(block_id);

                // UPDATE
                self.update_end_block(end, (yield_block_idx, yield_block_idx));
                self.block_sections[yield_block_idx].block_type = BlockType::Yield {
                    next: reset_block_idx,
                };
                self.block_sections[reset_block_idx].block_type = BlockType::Reset { next: start };
                (start, reset_block_idx, child_observer_node)
            }
        }
    }

    fn compile<A, R, O: BehaviorObserverBuilder<A>>(
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
                let (child_idxs, child_observer_nodes): (Vec<_>, Vec<_>) = behaviors
                    .into_iter()
                    .map(|behavior| {
                        //
                        let (s, e, t) = self.compile::<A, R, O>(behavior, runner);
                        ((s, e), t)
                    })
                    .unzip();
                let sequence_end_idx = self.reserve_block(block_id);

                // UPDATE
                self.block_sections[sequence_start_idx].block_type = BlockType::SequenceStart {
                    next: child_idxs.first().map_or(sequence_end_idx, |c| c.0),
                };

                let current_iter = child_idxs.iter().map(|c| c.1);
                let next_iter = child_idxs
                    .iter()
                    .skip(1)
                    .map(|c| c.0)
                    .chain([sequence_end_idx]);

                for (current, next) in current_iter.zip(next_iter) {
                    self.update_end_block(current, (next, sequence_end_idx));
                }

                self.block_sections[sequence_end_idx].block_type =
                    BlockType::SequenceEnd { next: None };

                let observer_node = O::sequence(block_id, child_observer_nodes);
                (sequence_start_idx, sequence_end_idx, observer_node)
            }
            Behavior::Select(behaviors) => {
                // CREATE
                let block_id = self.reserve_block_id();
                let select_start_idx = self.reserve_block(block_id);
                let (child_idxs, child_observer_nodes): (Vec<_>, Vec<_>) = behaviors
                    .into_iter()
                    .map(|behavior| {
                        //
                        let (s, e, t) = self.compile::<A, R, O>(behavior, runner);
                        ((s, e), t)
                    })
                    .unzip();
                let select_end_idx = self.reserve_block(block_id);

                // UPDATE
                self.block_sections[select_start_idx].block_type = BlockType::SelectStart {
                    next: child_idxs.first().map_or(select_end_idx, |c| c.0),
                };

                let current_iter = child_idxs.iter().map(|c| c.1);
                let next_iter = child_idxs
                    .iter()
                    .skip(1)
                    .map(|c| c.0)
                    .chain([select_end_idx]);

                for (current, next) in current_iter.zip(next_iter) {
                    self.update_end_block(current, (select_end_idx, next));
                }

                self.block_sections[select_end_idx].block_type =
                    BlockType::SelectEnd { next: None };

                let observer_node = O::select(block_id, child_observer_nodes);
                (select_start_idx, select_end_idx, observer_node)
            }
            Behavior::Subtree(name, behavior) => {
                // CREATE
                let block_id = self.reserve_block_id();
                let subtree_start_idx = self.reserve_block(block_id);
                let (start, end, child_observer_node) = self.compile::<A, R, O>(*behavior, runner);
                let subtree_end_idx = self.reserve_block(block_id);

                // UPDATE
                self.block_sections[subtree_start_idx].block_type =
                    BlockType::SubtreeStart { next: start };
                self.update_end_block(end, (subtree_end_idx, subtree_end_idx));
                self.block_sections[subtree_end_idx].block_type =
                    BlockType::SubtreeEnd { next: None };

                let observer_node =
                    O::subtree(std::rc::Rc::from(name), block_id, child_observer_node);
                (subtree_start_idx, subtree_end_idx, observer_node)
            }
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
            BlockType::SelectEnd { next } => {
                *next = Some(to);
            }
            BlockType::SubtreeEnd { next } => {
                *next = Some(to);
            }
            _ => unreachable!("Should be unreachable for block_type: {:?}", block_type),
        }
    }
}
