use itertools::Itertools;

use crate::sim::blockbuilding::Block;
use crate::sim::common::*;

const _STAGE_MIDLEFT_1: u8 = 1;
const _STAGE_MIDLEFT_2: u8 = 2;
const _STAGE_MIDLEFT_3: u8 = 3;
const _STAGE_RIGHT_1: u8 = 4;
const _STAGE_RIGHT_2: u8 = 5;
const _STAGE_FRONT_RIGHT: u8 = 6;

pub type Continuation = (Block, SolutionMetadata);

/// Metadata about a particular solution.
///
/// This mainly includes info about what order we are using to solve grips,
/// which helps when figuring out the next blocks to solve.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SolutionMetadata {
    /// Stage of the solution.
    stage: u8,
    /// Grips blocked on the mid+left block.
    ///
    /// Once there are 6 blocked grips, the next step is to begin the right
    /// block.
    left_blocked_grips: GripSet,
    /// Right block active grip.
    ///
    /// This is undefined before [`_STAGE_RIGHT_1`].
    right_grip: Grip,
    /// Grips blocked on the right block.
    ///
    /// Once there are 4 blocked grips, the next step is to begin the last
    /// layer.
    right_blocked_grips: GripSet,
    /// Last layer grip.
    ///
    /// This is undefined before [`_STAGE_RIGHT_1`].
    last_layer: Grip,
}

impl SolutionMetadata {
    fn next_stage(mut self) -> Self {
        self.stage += 1;
        self
    }

    pub fn last_layer(self) -> Grip {
        assert!(self.stage >= _STAGE_RIGHT_1);
        self.last_layer
    }

    fn with_left_blocked_grips(mut self, grips: GripSet) -> Continuation {
        self.left_blocked_grips |= grips;
        (Block::CORE.expand(self.left_blocked_grips), self)
    }
    fn with_right_grip(mut self, g: Grip) -> Self {
        self.right_grip = g;
        self
    }
    fn with_last_layer(mut self, g: Grip) -> Self {
        self.last_layer = g;
        self
    }
    fn with_right_blocked_grips(mut self, grips: GripSet) -> Continuation {
        self.right_blocked_grips |= grips;
        let right_center = Piece::new_solved([self.right_grip]);
        let right_block = Block::from(right_center).expand(self.right_blocked_grips);
        (right_block, self)
    }

    fn next_stage_expand_left_block(self) -> impl IntoIterator<Item = Continuation> {
        ((!self.left_blocked_grips).iter())
            .map(move |g| self.next_stage().with_left_blocked_grips(GripSet::from(g)))
    }

    fn next_stage_expand_right_block(self) -> impl IntoIterator<Item = Continuation> {
        (self.right_blocked_grips.opposites()
            & !self.right_blocked_grips
            & !GripSet::from(self.last_layer))
        .iter()
        .map(move |g| self.next_stage().with_right_blocked_grips(GripSet::from(g)))
    }

    pub fn stage1(self) -> impl IntoIterator<Item = Continuation> {
        use axes::*;

        assert_eq!(self.stage, 0);
        itertools::iproduct!(X.grips(), Y.grips(), Z.grips(), W.grips())
            .map(|(x, y, z, w)| GripSet::from_iter([x, y, z, w]))
            .map(move |blocked_grips| self.next_stage().with_left_blocked_grips(blocked_grips))
    }
    pub fn stage2(self) -> impl IntoIterator<Item = Continuation> {
        assert_eq!(self.stage, 1);
        self.next_stage_expand_left_block()
    }
    pub fn stage3(self) -> impl IntoIterator<Item = Continuation> {
        assert_eq!(self.stage, 2);
        self.next_stage_expand_left_block()
    }
    pub fn stage4(self) -> impl IntoIterator<Item = Continuation> {
        assert_eq!(self.stage, 3);
        let [g1, g2] = (!self.left_blocked_grips).unwrap_two();
        let [ax1, ax2] = Axis::ALL
            .into_iter()
            .filter(|&axis| (GripSet::from(axis) & !self.left_blocked_grips).is_empty())
            .collect_array()
            .unwrap()
            .map(Axis::grips);
        itertools::iproduct!([[g1, g2], [g2, g1]], ax1, ax2).map(move |([g1, g2], g3, g4)| {
            self.next_stage()
                .with_right_grip(g1)
                .with_last_layer(g2)
                .with_right_blocked_grips(GripSet::from_iter([g2.opposite(), g3, g4]))
        })
    }
    pub fn stage5(self) -> impl IntoIterator<Item = Continuation> {
        assert_eq!(self.stage, 4);
        self.next_stage_expand_right_block()
    }
    pub fn stage6(self) -> impl IntoIterator<Item = Continuation> {
        assert_eq!(self.stage, 5);
        self.next_stage_expand_right_block()
    }
}
