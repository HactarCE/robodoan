use crate::{ElemId, GripId, GripSet, Twist, new::block_layer_mask::BlockLayerMask};

const MAX_BLOCK_COUNT: usize = 20;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
struct LastTwist(GripSet);
impl LastTwist {
    #[inline]
    fn grip_twisted(self, g: GripId) -> bool {
        self.0.contains(g)
    }
    /// Applies a twist and returns the new [`LastTwist`] and whether to add 1
    /// STM.
    #[must_use]
    pub fn twist(self, g: GripId) -> (Self, bool) {
        (
            Self((self.0 & GripSet::from_axis(g.axis_deprecated())) | GripSet::from(g)),
            !self.grip_twisted(g),
        )
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BlocksState {
    move_count: u16,
    last_twist: LastTwist,
    block_count: u8,
    block_attitudes: [ElemId; MAX_BLOCK_COUNT],
    block_layer_masks: [BlockLayerMask; MAX_BLOCK_COUNT],
}

impl BlocksState {
    #[must_use]
    pub fn twist(self, twist: Twist) -> Option<Self> {
        let (new_last_twist, new_move) = self.last_twist.twist(twist.grip);

        // Recombine blocks

        // Some(Self {
        //     move_count: self.move_count + new_move as u16,
        //     last_twist: new_last_twist,
        //     block_count: new_block_count,
        //     block_attitudes: new_block_attitudes,
        //     block_layer_masks: new_block_layer_masks,
        // })

        todo!()
    }
}
