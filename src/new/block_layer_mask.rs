// struct Block {
//     attitude:
// }

use std::{hint::assert_unchecked, ops::Mul};

use crate::{ElemId, F, GripId, O, R, U, new::common::AxisSet};

/// Layer mask of a block, represented using 12 bits: `0000_xyzw_xyzw_xyzw`
/// (MSb..LSb)
///
/// The lowest bit corresponds to the positive axis and the highest bit
/// corresponds to the negative axis.
///
/// The top 4 bits are always zero.
///
/// Blocks are always connected. I.e., `101` is an invalid bit pattern for an
/// axis.
///
/// If any axis is `000`, then the entire block must be zero.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockLayerMask(u16);
impl BlockLayerMask {
    pub const EMPTY: Self = Self(0);

    pub fn is_empty(self) -> bool {
        self == Self::EMPTY
    }

    /// Constructs a [`BlockLayers`] from raw bits, converting one empty axis
    /// into a whole empty block.
    ///
    /// The only assumption this makes is that no axis has the `101` bit
    /// pattern.
    fn from_bits_handle_empty(bits: u16) -> Self {
        if bits | (bits >> 4) | (bits >> 8) == 0xF {
            Self(bits)
        } else {
            Self::EMPTY
        }
    }

    /// Returns the "rank" of the block, which is the number of stickers on its
    /// outermost piece.
    pub fn rank(self) -> u8 {
        ((self.0 | (self.0 >> 8)) & 0xF).count_ones() as u8
    }

    /// Merges two blocks and returns the result. If the blocks cannot be merged, then
    /// [`BlockLayerMask::EMPTY`] is returned.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        if self.merge_axis(other).is_some() {
            Self(self.0 | other.0)
        } else {
            Self::EMPTY
        }
    }

    #[must_use]
    fn merge_axis(self, other: Self) -> Option<u8> {
        unsafe { assert_unchecked(self != other && self.0 != 0 && other.0 != 0) };
        debug_assert_ne!(self, other);
        debug_assert_ne!(self.0, 0);
        debug_assert_ne!(other.0, 0);
        let diff = self.0 ^ other.0;
        let merge_axis = diff.trailing_zeros() as u8 % 4;
        if diff & !(0x111 << merge_axis) != 0 {
            return None; // differ along multiple axes
        }
        if diff & (0x010 << merge_axis) == 0 {
            return None; // disconnected blocks not allowed
        }
        Some(merge_axis)
    }

    /// Splits the block on the grip and returns `[inside, outside]`.
    ///
    /// Either block may be [`BlockLayers::EMPTY`].
    pub fn split(self, grip: GripId) -> [BlockLayerMask; 2] {
        let axis_mask = axis_mask(grip.axis_deprecated());
        let grip_mask = grip_mask(grip);
        let inside = if self.0 & axis_mask == 0 {
            Self::EMPTY
        } else {
            Self(self.0 & (axis_mask | !grip_mask))
        };
        let outside = if self.0 & grip_mask & !axis_mask == 0 {
            Self::EMPTY
        } else {
            Self(self.0 & !axis_mask)
        };
        [inside, outside]
    }

    fn bits_for_grip(self, grip: GripId) -> u16 {
        let bits = self.0 & 0x111;
        if grip.sign_bit() { rev9(bits) } else { bits }
    }

    /// Returns a bitmask of the active axes, which are the axes where the block
    /// has at least one sticker.
    pub fn active_axes(self) -> AxisSet {
        AxisSet::from_bits(((self.0 | (self.0 >> 8)) & 0xF) as u8)
    }

    /// Returns the key for sorting this block using a radix sort. The output is
    /// in the range `0..81`.
    ///
    /// This key is not unique to the block, but it will never overlap with
    /// disjoint blocks.
    pub fn radix_sort_key(self) -> u8 {
        // For each axis, apply the following mapping:
        //
        // 001 -> 1
        // 010 -> 2
        // 100 -> 0
        // 011 -> 2
        // 110 -> 2
        // 111 -> 2
        let x = ((self.0) & 0x11).min(2) as u8;
        let y = ((self.0 >> 1) & 0x11).min(2) as u8;
        let z = ((self.0 >> 2) & 0x11).min(2) as u8;
        let w = ((self.0 >> 3) & 0x11).min(2) as u8;
        // Then combine them into a base-3 number.
        x + y * 3 + z * 9 + w * 27
    }
}

#[inline]
fn axis_mask(axis: usize) -> u16 {
    0x111 << axis
}

#[inline]
fn grip_mask(grip: GripId) -> u16 {
    (0b1 << grip.axis_deprecated()) << (grip.sign_bit() as u8 * 8)
}

impl Mul<BlockLayerMask> for ElemId {
    type Output = BlockLayerMask;

    fn mul(self, rhs: BlockLayerMask) -> Self::Output {
        let inv = self.inv();
        BlockLayerMask(
            rhs.bits_for_grip(inv * R)
                | rhs.bits_for_grip(inv * U) << 1
                | rhs.bits_for_grip(inv * F) << 2
                | rhs.bits_for_grip(inv * O) << 3,
        )
    }
}

/// Reverses the lowest 9 bits of a `u16`.
const fn rev9(x: u16) -> u16 {
    x.reverse_bits() >> 7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rev9() {
        assert_eq!(rev9(0x001), 0x100);
        assert_eq!(rev9(0x011), 0x110);
        assert_eq!(rev9(0x111), 0x111);
        assert_eq!(rev9(0x110), 0x011);
        assert_eq!(rev9(0x100), 0x001);
        assert_eq!(rev9(0x010), 0x010);
    }
}
