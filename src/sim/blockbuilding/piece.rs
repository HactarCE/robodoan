use std::fmt;
use std::ops::Mul;

use crate::sim::common::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Piece {
    /// Active grip set (changes as the piece moves around)
    pub grips: GripSet,
    pub attitude: ElemId,
}
impl Piece {
    pub fn new_solved(grips: impl IntoIterator<Item = GripId>) -> Self {
        Self {
            grips: GripSet::from_iter(grips),
            attitude: IDENT,
        }
    }
}
impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.grips)
    }
}

impl Mul<Piece> for ElemId {
    type Output = Piece;

    #[inline]
    fn mul(self, rhs: Piece) -> Self::Output {
        Piece {
            grips: self * rhs.grips,
            attitude: self * rhs.attitude,
        }
    }
}

impl Mul<Piece> for Twist {
    type Output = Piece;

    fn mul(self, rhs: Piece) -> Self::Output {
        self.assert_is_valid();

        // GRIP THEORY
        if rhs.grips.contains(self.grip) {
            self.transform * rhs
        } else {
            rhs
        }
    }
}
