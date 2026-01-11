use std::fmt;
use std::ops::Index;

use super::SolutionMetadata;
use crate::sim::blockbuilding::{Block, BlockList};
use crate::sim::common::*;
use crate::util::stackvec::StackVec;

/// Maximum number of moves allowed in a single segment.
const MAX_SOLUTION_SEGMENT_LEN: usize = 11;

/// ID for a [`Segment`].
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SegmentId(u32);

impl SegmentId {
    /// ID of the initial segment.
    const INIT: Self = Self(0);
}

/// Segment of a solution.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[repr(align(32))]
pub struct Segment {
    pub state: BlockList,                                          // 128 bytes
    pub segment_twists: StackVec<Twist, MAX_SOLUTION_SEGMENT_LEN>, // 15 bytes
    pub meta: SolutionMetadata,                                    // 5 bytes
    pub previous_segment: SegmentId,                               // 4 bytes
}

/// Assertion of `std::mem::size_of::<Segment>()`.
///
/// It doesn't matter that much, but it's nice to keep it small if we can.
const _SIZE_ASSERT: [u8; 128 + 32] = [0; std::mem::size_of::<Segment>()];

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let block_count = self.state.len();
        let twist_count = self.state.twist_count();
        write!(f, "{block_count} blocks in {twist_count} ETM")
    }
}

impl PartialOrd for Segment {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Segment {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl Segment {
    /// Adds a twist to the segment.
    #[must_use]
    pub fn push_twist(&self, twist: Twist) -> Option<Self> {
        crate::gpu_test::add_example(&self.state, twist);
        Some(Self {
            state: self.state.twist(twist).if_nonempty()?,
            segment_twists: self.segment_twists.push(twist)?,
            previous_segment: self.previous_segment,
            meta: self.meta,
        })
    }

    /// Adds a block to the segment.
    pub fn push_block(
        &self,
        setup_moves: &[Twist],
        new_block: Block,
        new_meta: SolutionMetadata,
    ) -> Option<Self> {
        // TODO: how bad is it if we recreate the blocklist each time we search it?
        Some(Self {
            state: self
                .state
                .add_block_with_setup_moves(setup_moves, new_block)
                .if_nonempty()?,
            meta: new_meta,
            ..self.clone()
        })
    }

    pub fn next_step(&self, previous_segment: SegmentId) -> Self {
        Self {
            state: self.state.clone(),
            segment_twists: StackVec::new(),
            previous_segment,
            meta: self.meta,
        }
    }

    fn sort_key(&self) -> impl Ord {
        (
            self.state.twist_count(),
            self.state.len(),
            self.previous_segment,
            self.segment_twists,
        )
    }
}

pub struct SegmentStore {
    pub scramble: Vec<Twist>,

    segments: Vec<Segment>,
    steps: Vec<Vec<SegmentId>>,
}

impl Index<SegmentId> for SegmentStore {
    type Output = Segment;

    fn index(&self, index: SegmentId) -> &Self::Output {
        &self.segments[index.0 as usize]
    }
}

impl SegmentStore {
    pub fn new(scramble: Vec<Twist>) -> Self {
        Self {
            scramble,
            segments: vec![Segment::default()],
            steps: vec![vec![SegmentId::INIT]],
        }
    }

    pub fn add_segments(&mut self, step: usize, segments: Vec<Segment>) {
        let start = self.segments.len() as u32;
        self.segments.extend(segments);
        let end = self.segments.len() as u32;
        self.push_batch(step, (start..end).map(SegmentId));
    }

    pub fn push_batch(&mut self, step: usize, segment_ids: impl Iterator<Item = SegmentId>) {
        crate::util::extend_vec_to_index(&mut self.steps, step);
        self.steps[step].extend(segment_ids);
    }

    pub fn segment_ids_for_step(&mut self, step: usize) -> &[SegmentId] {
        match self.steps.get(step) {
            Some(ids) => ids,
            None => &[],
        }
    }

    pub fn solution_twists_for_segment(&self, id: SegmentId) -> Vec<Twist> {
        self.twists_for_segment(&[], id)
    }

    pub fn all_prior_twists_for_segment(&self, id: SegmentId) -> Vec<Twist> {
        self.twists_for_segment(&self.scramble, id)
    }

    fn twists_for_segment(&self, init: &[Twist], mut id: SegmentId) -> Vec<Twist> {
        let mut reversed_twists = vec![];
        while id != SegmentId::INIT {
            let segment = &self[id];
            reversed_twists.extend(segment.segment_twists.into_iter().rev());
            id = segment.previous_segment;
        }
        init.iter()
            .chain(reversed_twists.iter().rev())
            .copied()
            .collect()
    }

    pub fn best_solutions_so_far(&self) -> Option<&Vec<SegmentId>> {
        self.steps.last()
    }

    pub fn next_step(&self) -> usize {
        self.steps.len()
    }
}
