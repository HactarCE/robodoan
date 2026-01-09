// struct Block {
//     attitude:
// }

use std::{
    fmt,
    hint::assert_unchecked,
    ops::{BitOr, Mul},
};

use crate::{
    ElemId, F, GripId, GripSet, O, PackedLayers, Piece, R, U,
    new::common::{Axis, AxisSet},
};

/// Layer mask of a block, represented using 12 bits: `0000_wzyx_wzyx_wzyx`
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
#[derive(Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockLayerMask(u16);
impl fmt::Debug for BlockLayerMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ":")?;
        for ax in Axis::ALL {
            let bits = self.bits_for_grip(ax.grips()[0]);
            for i in [8, 4, 0] {
                let bit = bits & (1 << i) != 0;
                write!(f, "{}", if bit { ax.char() } else { '_' })?;
            }
            write!(f, ":")?;
        }
        Ok(())
    }
}
impl BlockLayerMask {
    pub const EMPTY: Self = Self(0);

    pub fn is_empty(self) -> bool {
        self == Self::EMPTY
    }

    pub fn from_piece(p: Piece) -> Self {
        let mut ret = 0;
        for g in p.grips.iter() {
            ret |= 1 << (g.axis().id() + 8 * g.sign_bit() as u8) // TODO: factor out this bit math
        }
        ret |= 0x0F0 & !(ret << 4) & !(ret >> 4);
        Self(ret)
    }

    /// Constructs a [`BlockLayers`] from raw bits, converting one empty axis
    /// into a whole empty block.
    ///
    /// The only assumption this makes is that no axis has the `101` bit
    /// pattern.
    pub fn from_bits_handle_empty(bits: u16) -> Self {
        if (bits | (bits >> 4) | (bits >> 8)) & 0xF == 0xF {
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

    /// Splits the block on the grip and returns `[active, inactive]`.
    ///
    /// Either block may be [`BlockLayers::EMPTY`].
    pub fn split(self, grip: GripId) -> [BlockLayerMask; 2] {
        let axis_mask = axis_mask(grip.axis_deprecated());
        let grip_mask = grip_mask(grip);
        let active = if self.0 & grip_mask == 0 {
            Self::EMPTY
        } else {
            Self(self.0 & (grip_mask | !axis_mask))
        };
        let inactive = if self.0 & axis_mask & !grip_mask == 0 {
            Self::EMPTY
        } else {
            Self(self.0 & !grip_mask)
        };
        [active, inactive]
    }

    fn bits_for_grip(self, grip: GripId) -> u16 {
        let bits = (self.0 >> grip.axis().id()) & 0x111;
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

    /// Returns the grips that are active and not blocked on the block.
    pub fn active_grips(self) -> GripSet {
        let x = self.0 & 0x111;
        let y = self.0 & 0x222;
        let z = self.0 & 0x444;
        let w = self.0 & 0x888;
        GripSet(
            (x == 0x001) as u8
                | ((x == 0x100) as u8) << 1
                | ((y == 0x002) as u8) << 2
                | ((y == 0x200) as u8) << 3
                | ((z == 0x004) as u8) << 4
                | ((z == 0x400) as u8) << 5
                | ((w == 0x008) as u8) << 6
                | ((w == 0x800) as u8) << 7,
        )
    }

    pub fn is_grip_inactive(self, g: GripId) -> bool {
        self.0 & (1 << (g.axis().id() + g.sign_bit() as u8 * 8)) == 0
    }

    pub fn piece_count(self) -> usize {
        let x = (self.0 & 0x111).count_ones();
        let y = (self.0 & 0x222).count_ones();
        let z = (self.0 & 0x444).count_ones();
        let w = (self.0 & 0x888).count_ones();
        (x * y * z * w) as usize
    }
}

#[inline]
fn axis_mask(axis: usize) -> u16 {
    0x111 << axis
}

#[inline]
fn grip_mask(grip: GripId) -> u16 {
    (1 << grip.axis_deprecated()) << (grip.sign_bit() as u8 * 8)
}

impl BitOr for BlockLayerMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
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

impl From<PackedLayers> for BlockLayerMask {
    fn from(value: PackedLayers) -> Self {
        let mut ret = 0;
        for i in 0..12 {
            let j = i / 4 + (i % 4) * 4;
            ret |= ((value.to_u16() >> j) & 1) << i
        }
        Self(ret)
    }
}

/// Reverses the lowest 9 bits of a `u16`.
const fn rev9(x: u16) -> u16 {
    x.reverse_bits() >> 7
}

#[cfg(test)]
impl proptest::arbitrary::Arbitrary for BlockLayerMask {
    type Parameters = ();

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        let axis_strategy =
            (0..6_usize).prop_map(|i| [0x001, 0x010, 0x100, 0x011, 0x110, 0x111][i]);
        let x = axis_strategy.clone();
        let y = axis_strategy.clone();
        let z = axis_strategy.clone();
        let w = axis_strategy.clone();
        [x, y, z, w]
            .prop_map(|[x, y, z, w]| Self(x | (y << 1) | (z << 2) | (w << 3)))
            .boxed()
    }

    type Strategy = proptest::strategy::BoxedStrategy<Self>;
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

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

    proptest! {
        #[test]
        fn proptest_active_grips(layer_mask: BlockLayerMask) {
            test_active_grips(layer_mask);
        }
    }

    fn test_active_grips(layer_mask: BlockLayerMask) {
        let expected = GripSet::from_iter(
            GripId::ALL
                .into_iter()
                .filter(|&g| layer_mask.bits_for_grip(g) == 0x001),
        );
        let actual = layer_mask.active_grips();
        assert_eq!(expected, actual);
    }
}
