use std::ops::{BitAnd, BitOr};

use crate::GripId;

/// Axis (0..4)
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Axis(u8);
impl Axis {
    pub const X: Self = Self(0);
    pub const Y: Self = Self(1);
    pub const Z: Self = Self(2);
    pub const W: Self = Self(3);

    pub const ALL: [Self; 4] = [Self(0), Self(1), Self(2), Self(3)];

    pub const fn id(self) -> u8 {
        self.0
    }

    /// Constructs an axis from an ID.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if `id >= 4`.
    #[track_caller]
    pub const fn new(id: u8) -> Self {
        debug_assert!(id < 4, "axis ID out of range");
        Self(id)
    }

    /// Returns the pair of grips `[positive, negative]` on the axis.
    #[inline]
    pub const fn grips(self) -> [GripId; 2] {
        let g1 = GripId::new(self.0 << 1);
        [g1, g1.opposite()]
    }

    pub const fn char(self) -> char {
        ['x', 'y', 'z', 'w'][self.0 as usize]
    }
}

/// Axis set (4 bits)
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AxisSet(u8);
impl AxisSet {
    pub fn bits(self) -> u8 {
        self.0
    }

    /// Constructs an axis set from a bitmask.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if `bits >= 0xF`.
    #[inline]
    #[track_caller]
    pub const fn from_bits(bits: u8) -> Self {
        debug_assert!(bits < 0xF, "axis set bitmask out of range");
        Self(bits)
    }

    #[inline]
    pub fn len(self) -> u32 {
        self.0.count_ones()
    }

    /// Returns the only axis in the set.
    ///
    /// Panics if there is not exactly one axis in the set.
    #[track_caller]
    pub fn unwrap_one(self) -> Axis {
        assert_eq!(self.len(), 1, "multiple active axes");
        Axis(self.0.trailing_zeros() as u8)
    }
}
impl From<Axis> for AxisSet {
    fn from(value: Axis) -> Self {
        Self(1 << value.0)
    }
}
impl BitOr for AxisSet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl BitAnd for AxisSet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
