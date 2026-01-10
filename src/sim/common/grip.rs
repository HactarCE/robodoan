use std::fmt;
use std::ops::Mul;

use super::{Axis, Elem, Twist, Vec4, group};

/// Puzzle grip: R, L, U, D, F, B, O, I
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Grip(u8);

impl fmt::Display for Grip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}", self.char().to_ascii_lowercase())
        } else {
            write!(f, "{}", self.char())
        }
    }
}

impl Grip {
    /// List of all grips.
    pub const ALL: [Self; 8] = [
        Self(0),
        Self(1),
        Self(2),
        Self(3),
        Self(4),
        Self(5),
        Self(6),
        Self(7),
    ];

    /// Constructs a grip from an ID.
    ///
    /// # Panics
    ///
    /// Panics if `id >= 8`.
    #[track_caller]
    pub const fn new(id: u8) -> Self {
        assert!(id < 8, "grip ID out of range");
        Self(id)
    }

    /// Returns the ID of the grip. (0..4)
    pub const fn id(self) -> u8 {
        self.0
    }

    /// Returns an uppercase character representing the grip.
    pub const fn char(self) -> char {
        b"RLUDFBOI"[self.0 as usize] as char
    }

    /// Returns the axis of the grip.
    pub const fn axis(self) -> Axis {
        Axis::new(self.0 >> 1)
    }

    /// Returns whether the grip is positive.
    pub const fn is_pos(self) -> bool {
        self.sign_bit() == 0
    }

    /// Returns `0` if the grip is positive or `1` if the grip is negative.
    pub const fn sign_bit(self) -> u8 {
        self.0 & 1
    }

    /// Returns `1` if the grip is positive or `-1` if the grip is negative.
    pub const fn signum(self) -> i8 {
        if self.is_pos() { 1 } else { -1 }
    }

    /// Returns the opposite grip.
    pub const fn opposite(self) -> Self {
        Self(self.0 ^ 1)
    }

    /// Returns the vector pointing toward the grip in space.
    pub fn vec(self) -> Vec4 {
        let mut ret = super::vectors::ZERO;
        ret[self.axis().id() as usize] = self.signum();
        ret
    }

    /// Returns an array of all twists on the grip.
    pub fn twists(self) -> [Twist; 23] {
        let grip = self;
        self.axis()
            .stabilizer_without_identity()
            .map(|transform| Twist { grip, transform })
    }
}

impl Mul<Grip> for Elem {
    type Output = Grip;

    fn mul(self, rhs: Grip) -> Self::Output {
        group::mul_elem_grip(self, rhs)
    }
}
