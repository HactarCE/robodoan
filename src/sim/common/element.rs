use std::fmt;
use std::ops::Mul;

use super::{Grip, Vec4, group};

/// Element from the grip group.
#[derive(
    Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, bytemuck::Zeroable, bytemuck::Pod,
)]
#[repr(C)]
pub struct Elem(u8);

impl fmt::Debug for Elem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl fmt::Display for Elem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let initial_grips = super::grips::RUFO;
        let permuted_grips = initial_grips.map(|g| *self * g);

        fn display_grips(grips: [Grip; 4]) -> String {
            grips.into_iter().map(|g| g.char()).collect()
        }

        let id = self.id();
        let initial_grips = display_grips(initial_grips);
        let permuted_grips = display_grips(permuted_grips);

        write!(f, "{id}=[{initial_grips}->{permuted_grips}]")
    }
}

impl Elem {
    /// Identity element.
    pub const IDENT: Self = super::elements::IDENT;

    /// Constructs an element from an ID.
    ///
    /// # Panics
    ///
    /// Panics if `id >= group::ELEM_COUNT`.
    #[track_caller]
    pub const fn new(id: u8) -> Self {
        assert!(id < group::ELEM_COUNT as u8, "element ID out of range");
        Self(id)
    }

    /// Returns the internal ID.
    pub const fn id(self) -> u8 {
        self.0
    }

    /// Returns the inverse element.
    pub fn inv(self) -> Elem {
        group::invert_elem(self)
    }

    /// Transforms an object by the element.
    pub fn transform<T: TransformByElem>(self, obj: T) -> T {
        obj.transform_by(self)
    }

    /// Returns an iterator over all elements in the group.
    pub fn iter_all() -> impl Iterator<Item = Self> {
        (0..group::ELEM_COUNT as u8).map(Self)
    }
}

impl Mul for Elem {
    type Output = Elem;

    fn mul(self, rhs: Self) -> Self::Output {
        group::mul_elem_elem(self, rhs)
    }
}

impl Mul<Vec4> for Elem {
    type Output = Vec4;

    fn mul(self, rhs: Vec4) -> Self::Output {
        group::mul_elem_vec(self, rhs)
    }
}

/// Type that can be transformed by an element.
///
/// Most types can be transformed by multiplying [`Elem`] by them. Some types
/// (such as [`Elem`] itself) already have multiplication defined, so this trait
/// is used instead.
///
/// Prefer calling [`Elem::transform()`] instead of directly calling
/// [`TransformByElem::transform_by()`].
pub trait TransformByElem {
    /// Transform `self` by `elem`.
    ///
    /// Prefer calling [`Elem::transform()`] instead of directly calling
    /// [`TransformByElem::transform_by()`].
    fn transform_by(self, elem: Elem) -> Self;
}

impl TransformByElem for Elem {
    fn transform_by(self, elem: Elem) -> Self {
        elem * self * elem.inv()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn test_simple_rotations() {
        use super::super::elements::*;
        use super::super::vectors::*;

        for (elem, init, expected) in [
            (IDENT, [X, Y, Z, W], [X, Y, Z, W]),
            (XY, [X, Y, Z, W], [Y, -X, Z, W]),
            (XZ, [X, Y, Z, W], [Z, Y, -X, W]),
            (XW, [X, Y, Z, W], [W, Y, Z, -X]),
            (YX, [X, Y, Z, W], [-Y, X, Z, W]),
            (ZX, [X, Y, Z, W], [-Z, Y, X, W]),
            (WX, [X, Y, Z, W], [-W, Y, Z, X]),
            (YZ, [X, Y, Z, W], [X, Z, -Y, W]),
            (YW, [X, Y, Z, W], [X, W, Z, -Y]),
            (ZY, [X, Y, Z, W], [X, -Z, Y, W]),
            (WY, [X, Y, Z, W], [X, -W, Z, Y]),
            (ZW, [X, Y, Z, W], [X, Y, W, -Z]),
            (WZ, [X, Y, Z, W], [X, Y, -W, Z]),
        ] {
            for (a, b) in init.into_iter().zip(expected) {
                assert_eq!(elem * a, b);
            }
        }
    }

    impl Arbitrary for Elem {
        type Parameters = ();

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            (0..group::ELEM_COUNT as u8).prop_map(Self::new)
        }

        type Strategy = proptest::strategy::Map<std::ops::Range<u8>, fn(u8) -> Elem>;
    }
}
