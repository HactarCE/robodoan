use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Mul, Not};

use super::{Axis, Elem, Grip};

/// Grip set (8 bits)
#[derive(Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GripSet(u8);

impl fmt::Debug for GripSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl fmt::Display for GripSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for g in self.iter() {
            g.fmt(f)?;
        }
        if self.is_empty() {
            write!(f, "∅")?;
        }
        Ok(())
    }
}

impl GripSet {
    /// Set containing no grips
    pub const EMPTY: GripSet = GripSet(0);
    /// Set containing all grips
    pub const ALL: GripSet = GripSet(0xFF);

    /// Constructs an grip set from a bitmask.
    #[track_caller]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    /// Returns the bitmask of the grip set.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Returns the number of grips in the set.
    pub fn len(self) -> u32 {
        self.0.count_ones()
    }

    /// Returns whether the grip set is empty.
    pub fn is_empty(self) -> bool {
        self == Self::EMPTY
    }

    /// Returns whether a grip is in the set.
    pub fn contains(self, grip: Grip) -> bool {
        self.0 & (1 << grip.id()) != 0
    }

    /// Iterates over grips in the set.
    pub fn iter(self) -> impl Iterator<Item = Grip> {
        Grip::ALL.into_iter().filter(move |&g| self.contains(g))
    }

    /// Returns the opposite grip set.
    ///
    /// Think of this as a central inversion of the grip set.
    #[must_use]
    pub fn opposites(self) -> Self {
        Self((self.0 & 0x55) << 1 | (self.0 & 0xAA) >> 1)
    }

    /// Returns the only grip in the set, or `None` if there is not exactly one
    /// grip in the set.
    pub fn exactly_one(self) -> Option<Grip> {
        (self.len() == 1).then(|| Grip::new(self.0.trailing_zeros() as u8))
    }

    /// Returns the only two grips in the set, or `None` if there are not
    /// exactly two grips in the set.
    pub fn exactly_two(self) -> Option<[Grip; 2]> {
        (self.len() == 2).then(|| {
            let first = self.0.trailing_zeros();
            let second = (self.0 & (GripSet::ALL.0 << (first + 1))).trailing_zeros();
            [first, second].map(|id| Grip::new(id as u8))
        })
    }

    /// Returns the only grip in the set.
    ///
    /// # Panics
    ///
    /// Panics if there is not exactly one grip in the set.
    #[track_caller]
    pub fn unwrap_one(self) -> Grip {
        self.exactly_one().expect("expected one grip")
    }

    /// Returns the only two grips in the set.
    ///
    /// # Panics
    ///
    /// Panics if there are not exactly two grips in the set.
    #[track_caller]
    pub fn unwrap_two(self) -> [Grip; 2] {
        self.exactly_two().expect("expected two grips")
    }
}

/// Constructs a grip set containing the two grips on an axis.
impl From<Axis> for GripSet {
    fn from(value: Axis) -> Self {
        Self::from_bits(0b11 << (value.id() * 2))
    }
}

/// Constructs a grip set containing a single grip.
impl From<Grip> for GripSet {
    fn from(value: Grip) -> Self {
        Self(1 << value.id())
    }
}

impl FromIterator<Grip> for GripSet {
    fn from_iter<T: IntoIterator<Item = Grip>>(iter: T) -> Self {
        iter.into_iter()
            .map(Self::from)
            .fold(Self::EMPTY, |a, b| a | b)
    }
}

impl BitOr for GripSet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GripSet {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GripSet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GripSet {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitXor for GripSet {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for GripSet {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Not for GripSet {
    type Output = Self;

    fn not(self) -> Self::Output {
        self ^ GripSet::ALL
    }
}

impl Mul<GripSet> for Elem {
    type Output = GripSet;

    fn mul(self, rhs: GripSet) -> Self::Output {
        GripSet(rhs.iter().map(|g| 1 << (self * g).id()).sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grip_set_unwrap() {
        for g1 in Grip::ALL {
            let one = GripSet::from_iter([g1]);
            assert_eq!(one.exactly_one(), Some(g1));
            assert_eq!(one.exactly_two(), None);
            for g2 in Grip::ALL {
                if g1 < g2 {
                    let two = GripSet::from_iter([g1, g2]);
                    assert_eq!(two.exactly_one(), None);
                    assert_eq!(two.exactly_two(), Some([g1, g2]));
                }
            }
        }
    }

    #[test]
    fn test_grip_set_opposite() {
        for bits in u8::MIN..=u8::MAX {
            let grips = GripSet::from_bits(bits);
            let expected = grips.iter().map(|g| g.opposite()).collect();
            let actual = grips.opposites();
            assert_eq!(actual, expected)
        }
    }
}
