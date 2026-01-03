use itertools::Itertools;

use crate::sim::*;

/// Metadata about a particular solution.
///
/// This mainly includes info about what order we are using to solve grips,
/// which helps when figuring out the next blocks to solve.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SolutionMetadata {
    stage: u16,
    first_block: Block,
    second_block: Block,
    third_block: Block,

    last_layer: Option<GripId>, // last grip (last layer)
    right_grip: Option<GripId>, // second-to-last grip (right block in typical 3-block)
    front_grip: Option<GripId>, // third-to-last grip (front grip in typical 3-block)
}

impl SolutionMetadata {
    fn next_stage(mut self) -> Self {
        self.stage += 1;
        self
    }

    fn with_last_layer(mut self, grip: GripId) -> Self {
        self.last_layer = Some(grip);
        self
    }
    pub fn last_layer(self) -> GripId {
        self.last_layer.unwrap()
    }

    fn with_right_grip(mut self, grip: GripId) -> Self {
        self.right_grip = Some(grip);
        self
    }
    fn right_grip(self) -> GripId {
        self.right_grip.unwrap()
    }

    fn with_front_grip(mut self, grip: GripId) -> Self {
        self.front_grip = Some(grip);
        self
    }
    fn front_grip(self) -> GripId {
        self.front_grip.unwrap()
    }

    fn with_first_block(mut self, block: Block) -> (Block, Self) {
        self.first_block = block;
        (block, self)
    }
    fn with_second_block(mut self, block: Block) -> (Block, Self) {
        self.second_block = block;
        (block, self)
    }
    fn with_third_block(mut self, block: Block) -> (Block, Self) {
        self.third_block = block;
        (block, self)
    }

    pub fn stage1(self) -> impl IntoIterator<Item = (Block, Self)> {
        assert_eq!(self.stage, 0);
        itertools::iproduct!([R, L], [U, D], [F, B], [I, O])
            .map(|(x, y, z, w)| Block::new_solved([], [x, y, z, w]).unwrap())
            .map(move |block| self.next_stage().with_first_block(block))
    }
    pub fn stage2(self) -> impl IntoIterator<Item = (Block, Self)> {
        assert_eq!(self.stage, 1);
        (self.first_block.inactive_grips().iter())
            .map(move |grip| self.first_block.expand_to_active_grip(grip))
            .map(move |block| self.next_stage().with_first_block(block))
    }
    pub fn stage3(self) -> impl IntoIterator<Item = (Block, Self)> {
        assert_eq!(self.stage, 2);
        (self.first_block.inactive_grips().iter())
            .map(move |grip| self.first_block.expand_to_active_grip(grip))
            .map(move |block| self.next_stage().with_first_block(block))
    }
    pub fn stage4(self) -> impl IntoIterator<Item = (Block, Self)> {
        assert_eq!(self.stage, 3);
        let [g1, g2] = self.first_block.inactive_grips().unwrap_exactly_two();
        let [ax1, ax2] = [0, 1, 2, 3]
            .into_iter()
            .filter(|&axis| self.first_block.is_fully_blocked_on_axis(axis))
            .collect_array()
            .unwrap()
            .map(GripId::pair_on_axis_deprecated);
        itertools::iproduct!([[g1, g2], [g2, g1]], ax1, ax2).map(move |([g1, g2], g3, g4)| {
            self.next_stage()
                .with_right_grip(g1)
                .with_last_layer(g2)
                .with_second_block(Block::new_solved([g1], [g2, g3, g4]).unwrap())
        })
    }
    pub fn stage5(self) -> impl IntoIterator<Item = (Block, Self)> {
        assert_eq!(self.stage, 4);
        let [g1, g2] =
            (self.second_block.blocked_grips() - self.last_layer().opposite()).unwrap_exactly_two();
        [[g1, g2], [g2, g1]].map(move |[a, b]| {
            self.next_stage()
                .with_front_grip(b.opposite())
                .with_second_block(self.second_block.expand_to_active_grip(a.opposite()))
        })
    }
    pub fn stage6(self) -> impl IntoIterator<Item = (Block, Self)> {
        assert_eq!(self.stage, 5);
        let block =
            Block::new_solved([self.front_grip(), self.right_grip()], [self.last_layer()]).unwrap();
        [self.next_stage().with_third_block(block)]
    }
}
