use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Mul, Not};

use super::{Axis, Elem};

/// Axis set (4 bits)
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AxisSet(u8);

impl AxisSet {
    /// Set containing no axes
    pub const EMPTY: Self = Self(0);
    /// Set containing all axes
    pub const ALL: Self = Self(0xF);

    /// Constructs an axis set from a bitmask.
    ///
    /// # Panics
    ///
    /// Panics if `bits > 0xF`.
    #[track_caller]
    pub const fn from_bits(bits: u8) -> Self {
        assert!(bits <= 0xF, "axis set bitmask out of range");
        Self(bits)
    }

    /// Returns the bitmask of the axis set.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Returns the number of axes in the set.
    pub fn len(self) -> u32 {
        self.0.count_ones()
    }

    /// Returns whether the axis set is empty.
    pub fn is_empty(self) -> bool {
        self == Self::EMPTY
    }

    /// Returns whether an axis is in the set.
    pub fn contains(self, axis: Axis) -> bool {
        self.0 & (1 << axis.id()) != 0
    }

    /// Iterates over axes in the set.
    pub fn iter(self) -> impl Iterator<Item = Axis> {
        Axis::ALL.into_iter().filter(move |&ax| self.contains(ax))
    }

    /// Returns the only axis in the set, or `None` if there is not exactly one
    /// axis in the set.
    pub fn exactly_one(self) -> Option<Axis> {
        (self.len() == 1).then(|| Axis::new(self.0.trailing_zeros() as u8))
    }

    /// Returns the only axis in the set.
    ///
    /// # Panics
    ///
    /// Panics if there is not exactly one axis in the set.
    #[track_caller]
    pub fn unwrap_one(self) -> Axis {
        self.exactly_one().expect("expected one axis")
    }
}

/// Constructs an axis set containing a single axis.
impl From<Axis> for AxisSet {
    fn from(value: Axis) -> Self {
        Self(1 << value.id())
    }
}

impl FromIterator<Axis> for AxisSet {
    fn from_iter<T: IntoIterator<Item = Axis>>(iter: T) -> Self {
        iter.into_iter()
            .map(Self::from)
            .fold(Self::EMPTY, |a, b| a | b)
    }
}

impl BitOr for AxisSet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for AxisSet {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for AxisSet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for AxisSet {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitXor for AxisSet {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for AxisSet {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Not for AxisSet {
    type Output = Self;

    fn not(self) -> Self::Output {
        self ^ AxisSet::ALL
    }
}

impl Mul<AxisSet> for Elem {
    type Output = AxisSet;

    fn mul(self, rhs: AxisSet) -> Self::Output {
        AxisSet(rhs.iter().map(|g| 1 << (self * g).id()).sum())
    }
}
