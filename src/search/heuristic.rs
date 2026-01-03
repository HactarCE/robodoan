use super::*;

/// Heuristic for pruning search branches.
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub enum Heuristic {
    TrivialCorrect,
    /// Prune aggressively; prune branches that are unlikely to result in a
    /// solution.
    Fast,
    /// Prune conservatively; never prune a branch that could possibly result in
    /// a solution.
    #[default]
    Correct,
}

impl Heuristic {
    /// Returns whether the heuristic believes that `state` can be reduced to
    /// `expected_blocks` blocks within `remaining_moves`.
    pub fn might_be_solvable(
        self,
        puzzle: &Puzzle,
        state: BlockSet,
        expected_blocks: usize,
        remaining_moves: usize,
    ) -> bool {
        let remaining_pairings_needed = state.blocks.len() - expected_blocks;

        remaining_pairings_needed <= self.combinatoric_limit(expected_blocks, remaining_moves)
            && remaining_pairings_needed
                <= self.grip_theoretic_limit(puzzle, state, remaining_moves)
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
    fn grip_theoretic_limit(
        self,
        puzzle: &Puzzle,
        state: BlockSet,
        remaining_moves: usize,
    ) -> usize {
        if self == Heuristic::TrivialCorrect {
            return usize::MAX;
        }

        let ndim = puzzle.ndim;

        let blocks_when_solved = state.blocks.map(|b| b.at_solved().layers());
        let mut max_pairings_possible = 0;
        for (i, (b1, b1_when_solved)) in
            std::iter::zip(state.blocks, blocks_when_solved).enumerate()
        {
            // assume `b1` can be made into a larger block within 3 moves.
            // assume other moves are used to make more blocks.
            let mut max_blocks_solvable_using_b1 = remaining_moves.saturating_sub(2);

            'b2: for (b2, b2_when_solved) in
                std::iter::zip(state.blocks, blocks_when_solved).skip(i + 1)
            {
                if let Some((_combined_block, merge_axis)) =
                    b1_when_solved.try_merge_with(b2_when_solved)
                {
                    let ([body, head], head_when_solved) =
                        if b1.layers().has_middle_slice_on_axis(merge_axis) {
                            ([b1, b2], b1_when_solved)
                        } else {
                            ([b2, b1], b2_when_solved)
                        };

                    let [g1, g2] = GripId::pair_on_axis(merge_axis);
                    let merge_grip = if head_when_solved.grip_status(g1) == GripStatus::Active {
                        g1
                    } else {
                        g2
                    };

                    let head_attitude = head.attitude();
                    let mut body_attitudes = body.indistinguishable_attitudes(ndim);
                    debug_assert!(
                        !body_attitudes.any(|a| a == head_attitude),
                        "should already be merged",
                    );

                    let head_merge_grip = head.attitude() * merge_grip;
                    let body_merge_grips =
                        body.mul_indistinguishable_attides_by_grip(ndim, merge_grip);

                    if body_merge_grips.contains(&head_merge_grip) {
                        // `b1` and `b2` can be paired in 1 move.
                        // assume other moves are used to make more blocks.
                        max_blocks_solvable_using_b1 = remaining_moves;
                        break 'b2; // not gonna do better than that
                    }

                    if remaining_moves > 1 {
                        for g in b1.layers().separating_axes(b2.layers()).iter() {
                            if body_merge_grips
                                .iter()
                                .any(|&m| g.can_transform_grip_to_grip(head_merge_grip, m))
                            {
                                // `b1` and `b2` can be paired in 2 moves.
                                // assume other moves are used to make more blocks
                                max_blocks_solvable_using_b1 = remaining_moves - 1;
                            }
                        }
                    }
                }
            }

            max_pairings_possible += max_blocks_solvable_using_b1;
        }

        max_pairings_possible
    }
}
