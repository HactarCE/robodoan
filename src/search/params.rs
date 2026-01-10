use super::Heuristic;

/// Search parameters for a [`crate::Solver`].
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct BlockBuildingSearchParams {
    /// Blockbuilding targets for each stage.
    pub targets: Targets,

    /// Heuristic for pruning search branches.
    pub heuristic: Heuristic,

    /// Maximum depth for IDDFS.
    pub max_depth: usize,

    /// Maximum depth to parallelize.
    pub parallel_depth: usize,

    /// How much to print.
    pub verbosity: u8,

    /// Number of solutions to target for each depth from 0 to 4.
    pub solution_count_targets: [usize; 5],
}

impl Default for BlockBuildingSearchParams {
    fn default() -> Self {
        Self {
            targets: Targets::default(),
            heuristic: Heuristic::default(),
            max_depth: 4,
            parallel_depth: 2,
            verbosity: 1,
            solution_count_targets: [1_000_000, 10_000, 5_000, 500, 50],
        }
    }
}

/// Blockbuilding targets for each stage.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Targets {
    /// Take less time to generate a slightly longer solution.
    #[default]
    Simple,
    /// Take longer to generate a shorter solution by starting each stage
    /// slightly before finishing the previous one.
    ///
    /// ~50% slower, but ~3.5 moves shorter
    CutCorners,
}

impl Targets {
    /// Returns `simple` for [`Targets::Simple`] or `cut_corners` for
    /// [`Targets::CutCorners`].
    pub fn select<T>(self, simple: T, cut_corners: T) -> T {
        match self {
            Targets::Simple => simple,
            Targets::CutCorners => cut_corners,
        }
    }
}
