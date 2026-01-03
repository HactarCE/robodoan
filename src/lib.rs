#[macro_use]
mod macros;
pub mod mc4d;
pub mod new;
pub mod search;
pub mod sim;
pub mod stackvec;

pub use sim::*;
pub use stackvec::StackVec;

pub use crate::search::*;

/// Maximum number of blocks representable in a puzzle.
pub const MAX_BLOCKS: usize = 21; // 63 bytes (+1 for length)

/// If we get more than this many solutions, truncate to this many.
const MAX_SOLUTION_COUNT: usize = 1_000_000;

/// Number of moves allowed in a single segment.
const MAX_SOLUTION_SEGMENT_LEN: usize = 11;

// If we get fewer than this many solutions, try searching deeper to generate
// more solutions.
//
// Multiply these by 2 to get better solutions slower
const MIN_SOLUTION_COUNT_DEPTH_1: usize = 10_000;
const MIN_SOLUTION_COUNT_DEPTH_2: usize = 5_000;
const MIN_SOLUTION_COUNT_DEPTH_3: usize = 500;
const MIN_SOLUTION_COUNT_DEPTH_4: usize = 50;

/// Whether to print 3D twist names when possible (e.g., `R` instead of `RO`).
pub const USE_3D_TWIST_NAMES: bool = false;

fn extend_vec_to_index<T: Default>(v: &mut Vec<T>, index: usize) {
    if v.len() <= index {
        v.resize_with(index + 1, T::default);
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Profile {
    /// Take less time to generate a slightly longer solution.
    #[default]
    Fast,
    /// Take longer to generate a shorter solution.
    ///
    /// ~50% slower, but ~3.5 moves shorter
    Short,
}
impl Profile {
    pub fn select<T>(self, fast: T, short: T) -> T {
        match self {
            Profile::Fast => fast,
            Profile::Short => short,
        }
    }
}

pub fn parse_twists(s: &str) -> Vec<Twist> {
    s.split_whitespace()
        .map(|word| TWISTS_FROM_NAME[word])
        .collect()
}
