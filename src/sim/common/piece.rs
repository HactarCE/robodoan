use std::fmt;
use std::ops::Mul;

use crate::sim::common::*;

/// Piece of the puzzle
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Piece {
    /// Current active grip set (changes as the piece moves around)
    pub grips: GripSet,
    /// Attitude of the piece
    pub attitude: Elem,
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.grips)
    }
}

impl Piece {
    /// Constructs a solved piece with the given active grips.
    pub fn new_solved(grips: impl IntoIterator<Item = Grip>) -> Self {
        Self {
            grips: GripSet::from_iter(grips),
            attitude: Elem::IDENT,
        }
    }

    /// Returns the piece with solved attitude.
    pub fn at_solved(self) -> Self {
        self.attitude.inv() * self
    }
}

impl Mul<Piece> for Elem {
    type Output = Piece;

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
        // GRIP THEORY
        if rhs.grips.contains(self.grip) {
            self.transform * rhs
        } else {
            rhs
        }
    }
}
