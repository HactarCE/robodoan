use std::fmt;

use crate::sim::common::*;

/// Bit offset for the 8-bit block count.
const OFS_BLOCK_COUNT: u32 = 0;
/// Bit offset for the 8-bit last twisted grip set.
const OFS_TWISTED_GRIPS: u32 = 8;
/// Bit offset for the 16-bit twist count.
const OFS_TWIST_COUNT: u32 = 16;

/// Bitmask for the 8-bit block count.
const _MASK_BLOCK_COUNT: u32 = 0xFF << OFS_BLOCK_COUNT;
/// Bitmask for the 8-bit last twisted grip set.
const _MASK_TWISTED_GRIPS: u32 = 0xFF << OFS_TWISTED_GRIPS;
/// Bitmask for the 16-bit twist count.
const _MASK_TWIST_COUNT: u32 = 0xFFFF << OFS_TWIST_COUNT;

/// Metadata for a [`super::BlockList`].
///
/// Bits are assigned as follows:
///
/// - 0..8 = block count
/// - 8..16 = last twisted grip set
/// - 16..32 = twist count (STM)
///
/// If the block count exceeds [`super::block_list::MAX_BLOCK_COUNT`], then
/// overflow has occurred.
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub(super) struct BlockListMeta(u32);

impl fmt::Debug for BlockListMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockListMeta")
            .field("block_count", &self.block_count())
            .field("twisted_grips", &self.twisted_grips())
            .field("twist_count", &self.twist_count())
            .finish()
    }
}

impl BlockListMeta {
    pub const DEFAULT: Self = Self(0);

    pub const fn block_count(&self) -> u8 {
        ((self.0 >> OFS_BLOCK_COUNT) & 0xFF) as u8
    }
    pub const fn twisted_grips(&self) -> GripSet {
        GripSet::from_bits(((self.0 >> OFS_TWISTED_GRIPS) & 0xFF) as u8)
    }
    pub const fn twist_count(&self) -> u16 {
        ((self.0 >> OFS_TWIST_COUNT) & 0xFFFF) as u16
    }

    pub const fn set_block_count(&mut self, new_block_count: u8) {
        self.0 = self.0 & !_MASK_BLOCK_COUNT | (new_block_count as u32) << OFS_BLOCK_COUNT;
    }
    pub const fn increment_block_count(&mut self) {
        self.0 += 1 << OFS_BLOCK_COUNT;
    }

    pub fn count_twist_on_grip(&mut self, grip: Grip) {
        let count_twist: bool = !self.twisted_grips().contains(grip);
        self.0 += (count_twist as u32) << OFS_TWIST_COUNT;
        self.0 &=
            !_MASK_TWISTED_GRIPS | (GripSet::from(grip.axis()).bits() as u32) << OFS_TWISTED_GRIPS;
        self.0 |= (GripSet::from(grip).bits() as u32) << OFS_TWISTED_GRIPS;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_list_meta_twist_count() {
        use grips::*;

        let mut m = BlockListMeta::default();
        assert_eq!(m.twist_count(), 0);
        m.count_twist_on_grip(R);
        assert_eq!(m.twist_count(), 1);
        m.count_twist_on_grip(R);
        assert_eq!(m.twist_count(), 1);
        m.count_twist_on_grip(L);
        assert_eq!(m.twist_count(), 2);
        m.count_twist_on_grip(R);
        assert_eq!(m.twist_count(), 2);
        m.count_twist_on_grip(U);
        assert_eq!(m.twist_count(), 3);
        m.count_twist_on_grip(R);
        assert_eq!(m.twist_count(), 4);
        m.count_twist_on_grip(D);
        assert_eq!(m.twist_count(), 5);
        m.count_twist_on_grip(U);
        assert_eq!(m.twist_count(), 6);
        m.count_twist_on_grip(U);
        assert_eq!(m.twist_count(), 6);
        m.count_twist_on_grip(D);
        assert_eq!(m.twist_count(), 6);
    }
}
