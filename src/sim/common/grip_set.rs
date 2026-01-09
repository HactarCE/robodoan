use std::fmt;
use std::ops::{Add, AddAssign, BitAnd, BitOr, BitXor, Mul, Not, Sub, SubAssign};

use crate::sim::common::*;

#[derive(Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GripSet(pub u8);
impl GripSet {
    pub const NONE: GripSet = GripSet(0);
    pub const ALL: GripSet = GripSet(0xFF);

    pub fn len(self) -> u32 {
        self.0.count_ones()
    }

    pub fn contains(self, grip: GripId) -> bool {
        grip.hint_assert_in_bounds();
        self.0 & (1 << grip.id()) != 0
    }
    pub fn iter(self) -> impl Iterator<Item = GripId> {
        HYPERCUBE_GRIPS
            .into_iter()
            .filter(move |&g| self.contains(g))
    }
    pub fn is_empty(self) -> bool {
        self == GripSet::NONE
    }

    pub fn exactly_one(self) -> Option<GripId> {
        (self.len() == 1).then(|| self.unwrap_exactly_one())
    }
    pub fn exactly_two(self) -> Option<[GripId; 2]> {
        (self.len() == 2).then(|| self.unwrap_exactly_two())
    }

    // TODO: extract into `impl From<Axis>`
    pub fn from_axis(axis: usize) -> Self {
        debug_assert!(axis < 4);
        Self(0b11 << (axis << 1))
    }

    #[track_caller]
    pub fn unwrap_exactly_one(self) -> GripId {
        assert_eq!(1, self.len(), "expected one grip");
        GripId::new(self.0.trailing_zeros() as u8)
    }
    #[track_caller]
    pub fn unwrap_exactly_two(self) -> [GripId; 2] {
        assert_eq!(2, self.len(), "expected two grips");
        let first = self.0.trailing_zeros();
        let second = (self.0 & (GripSet::ALL.0 << (first + 1))).trailing_zeros();
        [first, second].map(|id| GripId::new(id as u8))
    }
}
impl From<GripId> for GripSet {
    fn from(value: GripId) -> Self {
        Self(1 << value.id())
    }
}
impl FromIterator<GripId> for GripSet {
    fn from_iter<T: IntoIterator<Item = GripId>>(iter: T) -> Self {
        iter.into_iter()
            .map(Self::from)
            .fold(Self::NONE, |a, b| a | b)
    }
}
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
impl Add<GripId> for GripSet {
    type Output = Self;

    fn add(mut self, rhs: GripId) -> Self::Output {
        self += rhs;
        self
    }
}
impl AddAssign<GripId> for GripSet {
    fn add_assign(&mut self, rhs: GripId) {
        rhs.hint_assert_in_bounds();
        self.0 |= 1 << rhs.id();
    }
}
impl Sub<GripId> for GripSet {
    type Output = Self;

    fn sub(mut self, rhs: GripId) -> Self::Output {
        self -= rhs;
        self
    }
}
impl SubAssign<GripId> for GripSet {
    fn sub_assign(&mut self, rhs: GripId) {
        rhs.hint_assert_in_bounds();
        self.0 &= !(1 << rhs.id());
    }
}
impl BitOr for GripSet {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl BitAnd for GripSet {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl BitXor for GripSet {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}
impl Not for GripSet {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}
impl Mul<GripSet> for ElemId {
    type Output = GripSet;

    #[inline]
    fn mul(self, rhs: GripSet) -> Self::Output {
        self.hint_assert_in_bounds();
        GripSet(rhs.iter().map(|g| 1 << (self * g).id()).sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grip_set_unwrap() {
        for g1 in HYPERCUBE_GRIPS {
            let one = GripSet::from_iter([g1]);
            assert_eq!(one.exactly_one(), Some(g1));
            assert_eq!(one.exactly_two(), None);
            for g2 in HYPERCUBE_GRIPS {
                if g1 < g2 {
                    let two = GripSet::from_iter([g1, g2]);
                    assert_eq!(two.exactly_one(), None);
                    assert_eq!(two.exactly_two(), Some([g1, g2]));
                }
            }
        }
    }
}
