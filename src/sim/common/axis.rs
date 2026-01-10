use std::ops::Mul;

use itertools::Itertools;

use super::{Elem, Grip, group};

/// Non-identity transforms fixing each axis.
#[static_init::dynamic]
static AXIS_STABILIZER_WITHOUT_IDENTITY: [[Elem; 23]; 4] = Axis::ALL.map(|ax| {
    let g = ax.pos_grip();
    Elem::iter_all()
        .filter(|&e| e != Elem::IDENT && e * g == g)
        .collect_array()
        .unwrap()
});

/// Geometric axis: X, Y, Z, W
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Axis(u8);

impl Axis {
    /// List of all axes.
    pub const ALL: [Self; 4] = [Self(0), Self(1), Self(2), Self(3)];

    /// Constructs an axis from an ID.
    ///
    /// # Panics
    ///
    /// Panics if `id >= 4`.
    #[track_caller]
    pub const fn new(id: u8) -> Self {
        assert!(id < 4, "axis ID out of range");
        Self(id)
    }

    /// Returns the ID of the axis. (0..4)
    pub const fn id(self) -> u8 {
        self.0
    }

    /// Returns a lowercase character representing the axis.
    pub const fn char(self) -> char {
        b"xyzw"[self.0 as usize] as char
    }

    /// Returns the positive grip on the axis.
    pub const fn pos_grip(self) -> Grip {
        Grip::new(self.0 << 1)
    }

    /// Returns the negative grip on the axis.
    pub const fn neg_grip(self) -> Grip {
        self.pos_grip().opposite()
    }

    /// Returns the pair of grips `[positive, negative]` on the axis.
    pub const fn grips(self) -> [Grip; 2] {
        [self.pos_grip(), self.neg_grip()]
    }

    /// Returns a list of all transforms fixing the axis, excluding the
    /// identity.
    pub fn stabilizer_without_identity(self) -> [Elem; 23] {
        AXIS_STABILIZER_WITHOUT_IDENTITY[self.id() as usize]
    }
}

impl Mul<Axis> for Elem {
    type Output = Axis;

    fn mul(self, rhs: Axis) -> Self::Output {
        group::mul_elem_axis(self, rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mul_elem_axis() {
        for e in Elem::iter_all() {
            for a in Axis::ALL {
                assert_eq!(e * a, (e * a.grips()[0]).axis())
            }
        }
    }
}
