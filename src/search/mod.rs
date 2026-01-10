use std::sync::atomic::AtomicIsize;

use crate::sim::*;

mod heuristic;
mod meta;
mod params;
mod segment;

pub use heuristic::Heuristic;
use itertools::Itertools;
pub use meta::{Continuation, SolutionMetadata};
pub use params::{BlockBuildingSearchParams, Targets};
use rayon::iter::{
    IntoParallelIterator, IntoParallelRefIterator, ParallelExtend, ParallelIterator,
};
pub use segment::{Segment, SegmentId, SegmentStore};

pub struct Solver {
    params: BlockBuildingSearchParams,
    segments: SegmentStore,
}

impl Solver {
    pub fn new(params: BlockBuildingSearchParams, scramble: impl Into<Vec<Twist>>) -> Self {
        Self {
            params,
            segments: SegmentStore::new(scramble.into()),
        }
    }

    pub fn solve(mut self) -> Vec<Twist> {
        let start = std::time::Instant::now();

        let targets = self.params.targets;

        println!("\nSTAGE 1: mid + left, 2x2x2x2 block");
        self.do_blockbuilding_stage(targets.select(1, 5), |meta| meta.stage1());

        println!("\nSTAGE 2: mid + left, 2x2x3x2 block");
        self.do_blockbuilding_stage(targets.select(1, 5), |meta| meta.stage2());

        println!("\nSTAGE 3: mid + left, 2x3x3x2 block");
        self.do_blockbuilding_stage(targets.select(2, 6), |meta| meta.stage3());

        println!("\nSTAGE 4: right (mid + left), 2x2x2x1 block");
        self.do_blockbuilding_stage(targets.select(2, 6), |meta| meta.stage4());

        println!("\nSTAGE 5: right (mid + left), 2x2x3x1 block");
        self.do_blockbuilding_stage(targets.select(2, 5), |meta| meta.stage5());

        println!("\nSTAGE 6: F2L");
        self.do_blockbuilding_stage(targets.select(1, 1), |meta| meta.stage6());

        println!("\nTotal elapsed time: {:?}", start.elapsed());

        println!();
        let best_solution = *self
            .segments
            .best_solutions_so_far()
            .unwrap()
            .first()
            .unwrap();
        println!("Best solution: {}", self.segments[best_solution]);
        let twists_of_best_solution = self.segments.solution_twists_for_segment(best_solution);
        println!("{}", twists_of_best_solution.iter().join(" "));

        // let mut initial_state = PuzzleState::default();
        // initial_state.do_twists(&self.segments.scramble);

        let all_solutions = self
            .segments
            .best_solutions_so_far()
            .unwrap()
            .iter()
            .map(|&id| {
                let twists = self.segments.solution_twists_for_segment(id);
                (twists.len(), twists)
            })
            //     .map(|&id| {
            //         let segment = &self.segments[id];
            //         let twists = self.segments.solution_twists_for_segment(id);
            //         let mut state = initial_state.clone();
            //         state.do_twists(&twists);
            //         // let orientation_score =
            // state.unoriented_pieces(segment.meta.last_layer());         let
            // orientation_score = [0; 3];         (twists.len(), orientation_score,
            // twists)     })
            .sorted();

        let out_file_name = "out.txt";
        std::fs::write(
            out_file_name,
            all_solutions
                .into_iter()
                .map(|(twist_count, twists)| {
                    let twists_str = twists.iter().join(" ");
                    format!("{twist_count:3}    {twists_str}")
                })
                //         .map(|(twist_count, orientation_score, twists)| {
                //             let twists_str = twists.iter().join(" ");
                //             format!("{twist_count:3} {orientation_score:2?}    {twists_str}")
                //         })
                .join("\n"),
        )
        .unwrap();
        println!("All solutions written to {out_file_name}");

        twists_of_best_solution
    }

    fn do_blockbuilding_stage<I: IntoIterator<Item = Continuation>>(
        &mut self,
        target_block_count: usize,
        make_target_blocks: impl Send + Sync + Fn(SolutionMetadata) -> I,
    ) {
        let t = std::time::Instant::now();

        let step = self.segments.next_step();

        // Add pieces
        let new_segments = self.do_step(|this, prev_segments| {
            prev_segments
                .par_iter()
                .flat_map(|&prev_segment_id| {
                    let prev_segment = &this.segments[prev_segment_id];
                    let mut results = vec![];
                    for (new_block, new_meta) in make_target_blocks(prev_segment.meta) {
                        let setup_moves =
                            this.segments.all_prior_twists_for_segment(prev_segment_id);
                        if let Some(new_segment) =
                            prev_segment.push_block(&setup_moves, new_block, new_meta)
                        {
                            results.push(new_segment);
                        }
                    }
                    results
                })
                .collect()
        });
        let init_blocks = new_segments
            .iter()
            .map(|segment| segment.state.len())
            .max()
            .unwrap_or(0);

        println!(
            "  Added pieces ({} options with {} blocks each)",
            new_segments.len(),
            init_blocks,
        );
        if new_segments.is_empty() {
            println!("  WARNING: NO OPTIONS. You may need to increase `MAX_BLOCKS`");
        }

        self.segments.add_segments(step, new_segments);

        // Blockbuild
        for target in (target_block_count..init_blocks as usize).rev() {
            self.do_blockbuilding_step(target);
            // if self.steps.last().unwrap().is_empty() {
            //     log!(self.params, 1, "No solutions! Giving up ...");
            //     std::process::exit(1);
            // }
        }

        println!("  Completed stage in {:?}", t.elapsed());
    }

    fn do_blockbuilding_step(&mut self, block_target: usize) {
        let step = self.segments.next_step(); // TODO: bad

        let mut max_depth = 0;

        let new_segments = self.do_step(|this, prev_segments| {
            let mut new_segments = vec![];
            for depth in 0..=this.params.max_depth {
                let desired_solution_count = this.params.solution_count_targets
                    [depth.min(this.params.solution_count_targets.len() - 1)];
                let solutions_left_to_find =
                    desired_solution_count.saturating_sub(new_segments.len());
                overprint!("  Blockbuilding to {block_target} at depth {depth} ...");

                new_segments.par_extend(
                    prev_segments
                        .par_iter()
                        .flat_map(|&prev_segment| {
                            let mut results = vec![];
                            dfs_blockbuild(
                                this.params,
                                block_target,
                                depth,
                                &mut results,
                                this.segments[prev_segment].next_step(prev_segment),
                                None,
                                if depth > this.params.parallel_depth {
                                    this.params.parallel_depth
                                } else {
                                    0
                                },
                            );
                            results
                        })
                        .take_any(solutions_left_to_find),
                );

                max_depth = depth;
                if new_segments.len() >= desired_solution_count {
                    break;
                }
            }
            new_segments
        });

        let min_twist_count = new_segments
            .iter()
            .map(|s| s.state.twist_count())
            .min()
            .unwrap_or(0);
        overprintln!(
            "  Blockbuilt to {block_target} with max depth {max_depth} ({} solutions; best is {} ETM)",
            new_segments.len(),
            min_twist_count,
        );

        self.segments.add_segments(step, new_segments);
    }

    #[must_use]
    fn do_step(
        &mut self,
        continue_solutions: impl Send + Sync + FnOnce(&Self, &[SegmentId]) -> Vec<Segment>,
    ) -> Vec<Segment> {
        let t = std::time::Instant::now();

        let step = self.segments.next_step();
        let last_step = step - 1;
        let segments_to_search_from = self.segments.segment_ids_for_step(last_step).to_vec();

        let mut new_solution_segments = continue_solutions(self, &segments_to_search_from);

        // Sort by twist count and remove duplicates
        new_solution_segments.sort();
        new_solution_segments.dedup();

        log!(
            self.params,
            2,
            "Completed step in {:?} with {} solutions",
            t.elapsed(),
            new_solution_segments.len(),
        );

        if let Some(best) = new_solution_segments.first() {
            log!(self.params, 3, "Best solution: {best}");
            let twist_count_sums = new_solution_segments
                .iter()
                .map(|s| s.state.twist_count())
                .counts()
                .into_iter()
                .sorted()
                .map(|(len, count)| format!("{len}: {count}"))
                .join(", ");
            log!(self.params, 4, "By twist count: {{{twist_count_sums}}}");
        }

        let max_solution_count = self.params.solution_count_targets[0];

        if new_solution_segments.len() > max_solution_count {
            log!(
                self.params,
                3,
                "Truncating to {max_solution_count} solutions"
            );
            new_solution_segments.truncate(max_solution_count);
        }

        new_solution_segments
    }
}

/// Runs a depth-first search to `remaining_depth` for sequences of moves that
/// results in at most `expected_blocks` blocks.
///
/// Results are accumulated into `solutions_buffer`.
#[allow(clippy::too_many_arguments)]
pub fn dfs_blockbuild(
    params: BlockBuildingSearchParams,
    expected_blocks: usize,
    remaining_depth: usize,
    solutions_buffer: &mut Vec<Segment>,
    solution_so_far: Segment,
    solutions_left_to_find: Option<&AtomicIsize>,
    remaining_parallel_depth: usize,
) {
    let Segment {
        ref state,
        segment_twists,
        ..
    } = solution_so_far;

    if state.len() <= expected_blocks as u32 {
        // found a solution!
        if let Some(count) = solutions_left_to_find {
            count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        }
        solutions_buffer.push(solution_so_far);
        return;
    }
    if remaining_depth == 0 {
        return; // no more to search; give up
    }

    if solutions_left_to_find.is_some_and(|n| n.load(std::sync::atomic::Ordering::Relaxed) <= 0) {
        return; // give up
    }

    if !params
        .heuristic
        .might_be_solvable(state, expected_blocks, remaining_depth)
    {
        return; // probably not solvable; give up
    }

    let mut last_grips = segment_twists.iter().rev().map(|twist| twist.grip);
    let last_grip = last_grips.next();
    let second_to_last_grip = last_grips.next();
    let grip_is_worth_testing = |&grip: &Grip| {
        if last_grip == Some(grip) {
            return false; // same grip as last move
        }
        if last_grip == Some(grip.opposite()) && second_to_last_grip == Some(grip) {
            return false; // opposite grip already moved
        }
        if state.blocks().iter().all(|b| b.is_grip_inactive(grip)) {
            return false; // doesn't move any block
        }
        // TODO: don't check opposite if it was 2nd-to-last move
        true
    };

    let explore_twist = |twist, solutions_buffer: &mut Vec<Segment>| {
        if let Some(new_partial_solution) = solution_so_far.push_twist(twist) {
            dfs_blockbuild(
                params,
                expected_blocks,
                remaining_depth - 1,
                solutions_buffer,
                new_partial_solution,
                solutions_left_to_find,
                remaining_parallel_depth.saturating_sub(1),
            );
        }
    };

    if remaining_parallel_depth > 0 {
        let grips = Grip::ALL.into_par_iter().filter(grip_is_worth_testing);
        let twists = grips.flat_map(|grip| grip.twists());
        solutions_buffer.par_extend(twists.flat_map_iter(|twist| {
            let mut solutions_buffer = vec![];
            explore_twist(twist, &mut solutions_buffer);
            solutions_buffer
        }));
    } else {
        let grips = Grip::ALL.into_iter().filter(grip_is_worth_testing);
        let twists = grips.flat_map(|grip| grip.twists());
        twists.for_each(|twist| explore_twist(twist, solutions_buffer));
    }
}
