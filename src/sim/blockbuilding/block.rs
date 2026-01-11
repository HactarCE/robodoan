use std::fmt;
use std::fmt::Write;
use std::ops::Mul;

use crate::sim::common::*;

/// Bit offset for the 12-bit layer mask.
const OFS_LAYERS: u32 = 0;
/// Bit offset for the 8-bit attitude.
const OFS_ATT: u32 = 16;

/// Bitmask for the 12-bit layer mask.
const _MASK_LAYERS: u32 = 0xFFF << OFS_LAYERS;
/// Bitmask for the 8-bit attitude.
const _MASK_ATT: u32 = 0xFF << OFS_ATT;

/// Block of pieces on a 3x3x3x3 puzzle.
///
/// Bits are assigned as follows:
///
/// - 0..12 = layers when solved
///     - 0..4 = positive layers
///     - 4..8 = middle layers
///     - 8..12 = negative layers
/// - 12..16 = unused
/// - 16..24 = attitude
/// - 24..32 = unused
///
/// An empty block is valid and always contains all zeros.
///
/// Only the attitude changes when moving a block around the puzzle.
#[derive(
    Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, bytemuck::Zeroable, bytemuck::Pod,
)]
#[repr(C)]
pub struct Block(u32);

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut layers_str = ":".to_string();
        for ax in Axis::ALL {
            let bits = self.layer_bits_for_grip(ax.pos_grip());
            for i in [8, 4, 0] {
                let bit = bits & (1 << i) != 0;
                write!(&mut layers_str, "{}", if bit { ax.char() } else { '_' })?;
            }
            write!(&mut layers_str, ":")?;
        }

        if self.is_empty() {
            write!(f, "Block::EMPTY")
        } else {
            f.debug_struct("Block")
                .field("layers", &layers_str)
                .field("inner_rank", &self.inner_rank())
                .field("outer_rank", &self.outer_rank())
                .field("attitude", &self.attitude())
                .field("bits", &format!("0x{:08x}", self.0))
                .finish()
        }
    }
}

impl Block {
    /// Empty block
    pub const EMPTY: Self = Self(0);
    /// Block containing only the core
    pub const CORE: Self = Self::from_layer_bits(0x0F0);

    /// Returns whether the block is empty.
    pub fn is_empty(self) -> bool {
        self == Self::EMPTY
    }

    /// Constructs a block with solved attitude from layer bits.
    ///
    /// # Panics
    ///
    /// Panics if `layers >= 1 << 12`.
    pub const fn from_layer_bits(layer_bits: u16) -> Self {
        assert!(layer_bits < 1 << 12, "layer mask out of range");

        if layer_bits | (layer_bits >> 4) | (layer_bits >> 8) == 0 {
            Self::EMPTY
        } else {
            Self::from_layer_bits_nonempty(layer_bits)
        }
    }

    pub const fn from_bits_unchecked(bits: u32) -> Self {
        Self(bits as u32)
    }

    /// Constructs a block with solved attitude from layer bits, assuming they
    /// are valid and nonempty.
    const fn from_layer_bits_nonempty(layer_bits: u16) -> Self {
        Self((layer_bits as u32) << OFS_LAYERS)
    }

    /// Returns layer bits for an axis.
    ///
    /// - bit 0 = positive layer
    /// - bit 4 = middle layer
    /// - bit 8 = negative layer
    const fn layer_bits_for_axis(self, ax: Axis) -> u32 {
        self.0 >> (OFS_LAYERS + ax.id() as u32) & 0x111
    }

    /// Returns layer bits for an grip.
    ///
    /// - bit 0 = `g`
    /// - bit 4 = middle layer
    /// - bit 8 = `g.opposite()`
    const fn layer_bits_for_grip(self, g: Grip) -> u32 {
        let axis_bits = self.layer_bits_for_axis(g.axis());
        if g.is_pos() {
            axis_bits
        } else {
            rev9(axis_bits)
        }
    }

    const fn layer_bits(self) -> u16 {
        ((self.0 >> OFS_LAYERS) & 0xFFF) as u16
    }

    /// Returns the inner rank of the block, which is the number of stickers on
    /// the innermost piece. This is in the range `0..=4`.
    pub const fn inner_rank(self) -> u8 {
        let layer_bits = self.layer_bits();
        4 - (layer_bits & 0x0F0).count_ones() as u8
    }

    /// Returns the outer rank of the block, which is the number of stickers on
    /// the outermost piece. This is in the range `0..=4`.
    pub const fn outer_rank(self) -> u8 {
        let layer_bits = self.layer_bits();
        ((layer_bits | (layer_bits >> 8)) & 0xF).count_ones() as u8
    }

    /// Returns the attitude of the block.
    pub fn attitude(self) -> Elem {
        // TODO: consider storing the inverse attitude instead
        Elem::new(((self.0 >> OFS_ATT) & 0xFF) as u8)
    }

    /// Returns the inverse of the attitude of the block.
    fn inv_attitude(self) -> Elem {
        self.attitude().inv()
    }

    /// Returns the same block with a different attitude.
    #[must_use]
    fn with_attitude(self, attitude: Elem) -> Self {
        Self(self.0 & !_MASK_ATT | (attitude.id() as u32) << OFS_ATT)
    }

    /// Returns the same block with identity attitude.
    #[must_use]
    pub fn at_solved(self) -> Self {
        self.with_attitude(Elem::IDENT)
    }

    /// Splits the block along the grip and returns two new blocks `[active,
    /// inactive]`.
    ///
    /// - `active` is the block that would be affected by the twist.
    /// - `inactive` is the block that would **not** be affected by the twist.
    ///
    /// Either block may be [`Block::EMPTY`].
    #[must_use]
    pub fn split(self, grip: Grip) -> [Self; 2] {
        let g = self.inv_attitude() * grip;
        let layer_bits = self.layer_bits();
        let axis_mask = layer_bits_for_axis(g.axis());
        let grip_mask = layer_bit_for_grip(g);
        debug_assert_eq!(grip_mask & !axis_mask, 0);

        let active = if layer_bits & grip_mask == 0 {
            Self::EMPTY
        } else {
            Self::from_layer_bits_nonempty(layer_bits & (grip_mask | !axis_mask))
                .with_attitude(self.attitude())
        };

        let inactive = if layer_bits & axis_mask & !grip_mask == 0 {
            Self::EMPTY
        } else {
            Self::from_layer_bits_nonempty(layer_bits & !grip_mask).with_attitude(self.attitude())
        };

        [active, inactive]
    }

    /// Returns whether `grip` is inactive on the block at its current location.
    pub fn is_grip_inactive(self, grip: Grip) -> bool {
        let g = self.inv_attitude() * grip;
        self.layer_bits() & layer_bit_for_grip(g) == 0
    }

    /// Returns a bitmask of the active or blocked axes, which are the axes
    /// where the block has at least one sticker when solved.
    pub fn active_or_blocked_axes(self) -> AxisSet {
        let layer_bits = self.layer_bits();
        AxisSet::from_bits(((layer_bits | (layer_bits >> 8)) & 0xF) as u8)
    }

    /// Returns the grips that are active and not blocked on the block when
    /// solved.
    pub fn active_grips(self) -> GripSet {
        let layer_bits = self.layer_bits();
        let x = layer_bits & 0x111;
        let y = layer_bits & 0x222;
        let z = layer_bits & 0x444;
        let w = layer_bits & 0x888;
        GripSet::from_bits(
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

    /// Returns the grips that are active or blocked on the block when solved.
    pub fn active_or_blocked_grips(self) -> GripSet {
        let layer_bits = self.layer_bits();
        GripSet::from_bits(
            (layer_bits & 0x001 != 0) as u8
                | ((layer_bits & 0x100 != 0) as u8) << 1
                | ((layer_bits & 0x002 != 0) as u8) << 2
                | ((layer_bits & 0x200 != 0) as u8) << 3
                | ((layer_bits & 0x004 != 0) as u8) << 4
                | ((layer_bits & 0x400 != 0) as u8) << 5
                | ((layer_bits & 0x008 != 0) as u8) << 6
                | ((layer_bits & 0x800 != 0) as u8) << 7,
        )
    }

    /// Returns the grips active when solved.
    pub fn current_active_grips(self) -> GripSet {
        self.attitude() * self.active_grips()
    }

    /// Merges two blocks. Returns [`Block::EMPTY`] if the blocks cannot be
    /// merged.
    ///
    /// Panics in debug mode if `body == head`, if either input is empty, or if
    /// `body.rank() + 1 != head.rank()`.
    #[track_caller]
    pub fn merge(body: Self, head: Self) -> Block {
        // TODO: try with & without branching

        // Check preconditions
        debug_assert_ne!(body, head, "body and head are equal");
        debug_assert!(!body.is_empty(), "body is empty");
        debug_assert!(!head.is_empty(), "head is empty");
        debug_assert_eq!(
            body.inner_rank() + 1,
            head.inner_rank(),
            "body & head ranks are incompatible",
        );

        // Check layers
        let diff = body.layer_bits() ^ head.layer_bits();
        let merge_axis = diff.trailing_zeros() as u8 % 4;
        if diff & !(0x111 << merge_axis) != 0 {
            return Block::EMPTY; // differ along multiple axes
        }
        if diff & (0x010 << merge_axis) == 0 {
            return Block::EMPTY; // disconnected blocks not allowed
        }

        // Check attitudes
        let attitude_matches = body.attitude() == head.attitude() // always works
            || match body.outer_rank() {
                // core + center always matches
                0 => true,

                // center + ridge matches if the ridge attitude preserves the grip of the center
                1 => {
                    // assume that centers do not move (core is always stationary)
                    let g = body.active_or_blocked_axes().unwrap_one().pos_grip();
                    head.attitude() * g == g
                }

                // ridge + edge has 4 indistinguishable ridge attitudes
                2 => {
                    let delta = body.inv_attitude() * head.attitude(); // either can be inverted
                    ridge_indistinguishable_subgroup(body.active_or_blocked_axes()).contains(&delta)
                }

                // edge + corner requires exact attitude match
                3 | 4 => body.attitude() == head.attitude(),

                _ => unreachable!(),
            };
        if !attitude_matches {
            return Self::EMPTY;
        }

        Self::from_layer_bits_nonempty(body.layer_bits() | head.layer_bits())
            .with_attitude(head.attitude())
    }

    /// Returns the number of moves needed to pair `body` and `head`, or `None`
    /// if they cannot be paried.
    ///
    /// Panics in debug mode if the blocks can already be paired.
    pub fn moves_needed_to_pair(body: Self, head: Self) -> Option<usize> {
        if Self::merge(body.at_solved(), head.at_solved()).is_empty() {
            return None;
        }

        let diff = body.layer_bits() ^ head.layer_bits();
        let merge_axis = Axis::new(diff.trailing_zeros() as u8 % 4);

        debug_assert!(
            Self::merge(body, head).is_empty(),
            "blocks should already be merged",
        );

        match body.outer_rank() {
            // core + center always pair instantly
            0 => Some(0),

            // center + ridge
            1 => {
                let body_axis = body.active_or_blocked_axes().unwrap_one();
                if head.attitude() * merge_axis == body_axis {
                    Some(2) // "bad" ridge
                } else {
                    Some(1) // "good" ridge
                }
            }

            // ridge + edge
            2 => {
                debug_assert_eq!(body.active_or_blocked_axes().len(), 2);
                if body
                    .active_or_blocked_axes()
                    .contains(head.attitude() * merge_axis)
                {
                    Some(2)
                } else {
                    Some(1)
                }
            }

            // edge + corner or corner + corner requires exact attitude match
            3 | 4 => {
                let merge_grip =
                    (head.active_grips() & !body.active_or_blocked_grips()).unwrap_one();
                if body.attitude() * merge_grip == head.attitude() * merge_grip {
                    Some(1)
                } else {
                    // Reframe everything as though the body is already solved.
                    let delta = head.attitude() * body.inv_attitude();
                    // Which grips affect the head but not the body?
                    let free_grips = head.with_attitude(delta).current_active_grips()
                        & !body.active_or_blocked_grips();
                    // If we are able to solve this piece in 2 moves, then the
                    // first move has to take the head's merge grip to the
                    // body's merge grip without affecting the body. That means
                    // the twist can't be on an active grip of the body, and it
                    // can't be on the current merge axis of the body or the
                    // head.
                    if (free_grips
                        & !GripSet::from(merge_axis)
                        & !GripSet::from(delta * merge_grip))
                    .is_empty()
                    {
                        Some(3) // no such grips! requires 3 moves
                    } else {
                        Some(2)
                    }
                }
            }

            _ => unreachable!(),
        }
    }

    /// Returns the number of pieces in the block.
    pub fn piece_count(self) -> u32 {
        Axis::ALL
            .map(|ax| self.layer_bits_for_axis(ax).count_ones())
            .into_iter()
            .product()
    }

    /// Returns a block that is like this one, but is expanded to include one or
    /// more new grips.
    pub fn expand(self, new_grips: GripSet) -> Self {
        let mut layer_bits = self.layer_bits();
        for g in new_grips.iter() {
            layer_bits |= if layer_bits & layer_bit_for_grip(g.opposite()) != 0 {
                layer_bits_for_axis(g.axis()) // add middle layer as well
            } else {
                layer_bit_for_grip(g)
            }
        }
        Self::from_layer_bits(layer_bits)
    }
}

impl From<Piece> for Block {
    fn from(piece: Piece) -> Self {
        let mut ret = 0;
        for g in piece.at_solved().grips.iter() {
            ret |= layer_bit_for_grip(g);
        }
        ret |= 0x0F0 & !(ret << 4) & !(ret >> 4);
        Self::from_layer_bits(ret)
    }
}

impl Mul<Block> for Elem {
    type Output = Block;

    fn mul(self, rhs: Block) -> Self::Output {
        rhs.with_attitude(self * rhs.attitude())
    }
}

const fn layer_bits_for_axis(axis: Axis) -> u16 {
    0x111 << axis.id()
}

const fn layer_bit_for_grip(grip: Grip) -> u16 {
    1 << (grip.axis().id() + 8 * grip.sign_bit())
}

fn ridge_indistinguishable_subgroup(active_axes: AxisSet) -> [Elem; 4] {
    match active_axes.bits() {
        0b0011 => *elements::XY_STABILIZER,
        0b0110 => *elements::YZ_STABILIZER,
        0b1100 => *elements::ZW_STABILIZER,
        0b0101 => *elements::XZ_STABILIZER,
        0b1010 => *elements::YW_STABILIZER,
        0b1001 => *elements::XW_STABILIZER,
        _ => panic!("not a ridge"),
    }
}

/// Reverses the lowest 9 bits of a `u32` and zeros the rest.
#[must_use]
const fn rev9(x: u32) -> u32 {
    x.reverse_bits() >> (u32::BITS - 9)
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

    #[test]
    fn test_merge_core_center() {
        let core = Block::from_layer_bits(0x0F0); // core
        let mut center = Block::from_layer_bits(0x0E1); // R center
        let merged = Block::from_layer_bits(0xF1);
        assert_eq!(Block::merge(core, center), merged);
        center = elements::YZ * center;
        assert_eq!(
            Block::merge(core, center),
            merged.with_attitude(elements::YZ),
        );
    }

    #[test]
    fn test_merge_center_ridge() {
        let mut center = Block::from_layer_bits(0x0F1); // core + R center
        let mut ridge = Block::from_layer_bits(0xD3); // U center + RU ridge
        let merged = Block::from_layer_bits(0xF3);
        assert_eq!(Block::merge(center, ridge), merged);
        center = elements::YZ * center; // keeps X fixed
        ridge = elements::ZW * ridge; // keeps XY fixed
        assert_eq!(Block::merge(center, ridge), elements::ZW * merged);
        ridge = elements::XW * ridge; // keeps Y fixed, but moves X
        assert_eq!(Block::merge(center, ridge), Block::EMPTY);
    }

    #[test]
    fn test_merge_ridge_edge() {
        let mut ridge = Block::from_layer_bits(0xD3); // U center + RU ridge
        let mut edge = Block::from_layer_bits(0x5B); // UI ridge + RUI edge
        let merged = Block::from_layer_bits(0xDB);
        assert_eq!(Block::merge(ridge, edge), merged);
        ridge = elements::WZ * ridge; // ZW is indistinguishable on ridge
        assert_eq!(Block::merge(ridge, edge), merged);
        edge = elements::ZW * edge; // ZW is indistinguishable on ridge
        assert_eq!(Block::merge(ridge, edge), elements::ZW * merged);
        ridge = elements::XW * ridge; // keeps Z (previously Y) fixed
        edge = elements::XW * edge;
        assert_eq!(
            Block::merge(ridge, edge),
            elements::XW * elements::ZW * merged,
        );
    }
}
