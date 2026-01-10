use crate::sim::blockbuilding::*;

/// Heuristic for pruning search branches.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub enum Heuristic {
    /// Use a trivially correct heuristic that only does cheap computations.
    TrivialCorrect,
    /// Prune aggressively; prune branches that are unlikely to result in a
    /// solution.
    #[default]
    Fast,
    /// Prune conservatively; never prune a branch that could possibly result in
    /// a solution.
    Correct,
}

impl Heuristic {
    /// Returns whether the heuristic believes that `state` can be reduced to
    /// `expected_blocks` blocks within `remaining_moves`.
    pub fn might_be_solvable(
        self,
        state: &BlockList,
        expected_blocks: usize,
        remaining_moves: usize,
    ) -> bool {
        let remaining_pairings_needed = state.len() as usize - expected_blocks;

        remaining_pairings_needed <= self.combinatoric_limit(expected_blocks, remaining_moves)
            && remaining_pairings_needed <= self.grip_theoretic_limit(state, remaining_moves)
    }

    /// Returns the maximum number of block pairings possible using a naive
    /// combinatoric approach.
    fn combinatoric_limit(self, expected_blocks: usize, remaining_moves: usize) -> usize {
        match self {
            Heuristic::TrivialCorrect => (1 << remaining_moves) * expected_blocks,
            Heuristic::Fast => 1 << remaining_moves,
            Heuristic::Correct => (1 << remaining_moves) * expected_blocks,
        }
    }
    /// Returns the maximum number of block pairings using a grip-theoretic
    /// approach.
    fn grip_theoretic_limit(self, state: &BlockList, remaining_moves: usize) -> usize {
        if self == Heuristic::TrivialCorrect {
            return usize::MAX;
        }

        let mut max_pairings_possible = 0;

        for body_rank in 0..4 {
            let head_rank = body_rank + 1;
            for body in state.blocks_with_inner_rank(body_rank) {
                // assume `body` can be made into a larger block within 3 moves.
                // assume other moves are used to make more blocks.
                let mut max_blocks_solvable_using_body = remaining_moves.saturating_sub(2);

                'each_head: for head in state.blocks_with_inner_rank(head_rank) {
                    match Block::moves_needed_to_pair(body, head) {
                        None => continue,
                        Some(1) => {
                            // `body` and `head` can be paired in 1 move.
                            // assume other moves are used to make more blocks.
                            max_blocks_solvable_using_body = remaining_moves;
                            break 'each_head; // not gonna do better than that
                        }
                        Some(2) => {
                            // `body` and `head` can be paired in 2 moves.
                            // assume other moves are used to make more blocks
                            max_blocks_solvable_using_body = remaining_moves - 1;
                        }
                        Some(3) => (),
                        _ => unreachable!(), // never takes more than 3 moves
                    }
                }

                max_pairings_possible += max_blocks_solvable_using_body;
            }
        }

        max_pairings_possible
    }
}
