use std::fmt;
use std::ops::Mul;

use itertools::Itertools;

use super::group;
use super::space::*;

/// Identity group element.
pub const IDENT: ElemId = ElemId::new(0);
/// Rotation from +X to +Y.
pub const XY: ElemId = ElemId::new(1);
/// Rotation from +X to +Z.
pub const XZ: ElemId = ElemId::new(2);
/// Rotation from +X to +W.
pub const XW: ElemId = ElemId::new(3);

/// Rotation from +Y to +X.
pub const YX: ElemId = ElemId::new(13); // XY * XY * XY
/// Rotation from +Z to +X.
pub const ZX: ElemId = ElemId::new(25); // XZ * XZ * XZ
/// Rotation from +W to +X.
pub const WX: ElemId = ElemId::new(36); // XW * XW * XW

/// Rotation from +Y to +Z.
pub const YZ: ElemId = ElemId::new(97); // XZ * YX * ZX
/// Rotation from +Y to +W.
pub const YW: ElemId = ElemId::new(110); // XW * YX * WX
/// Rotation from +Z to +Y.
pub const ZY: ElemId = ElemId::new(84); // XY * ZX * YX
/// Rotation from +W to +Y.
pub const WY: ElemId = ElemId::new(86); // XY * WX * YX

/// Rotation from +Z to +W.
pub const ZW: ElemId = ElemId::new(133); // XW * ZX * WX
/// Rotation from +W to +Z.
pub const WZ: ElemId = ElemId::new(128); // XZ * WX * ZX

#[static_init::dynamic]
pub static X_STABILIZER: [ElemId; 24] = vector_stabilizer(X);
#[static_init::dynamic]
pub static Y_STABILIZER: [ElemId; 24] = vector_stabilizer(Y);
#[static_init::dynamic]
pub static Z_STABILIZER: [ElemId; 24] = vector_stabilizer(Z);
#[static_init::dynamic]
pub static W_STABILIZER: [ElemId; 24] = vector_stabilizer(W);

#[static_init::dynamic]
pub static XY_STABILIZER: [ElemId; 4] = plane_stabilizer(X, Y);
#[static_init::dynamic]
pub static XZ_STABILIZER: [ElemId; 4] = plane_stabilizer(X, Z);
#[static_init::dynamic]
pub static XW_STABILIZER: [ElemId; 4] = plane_stabilizer(X, W);
#[static_init::dynamic]
pub static YZ_STABILIZER: [ElemId; 4] = plane_stabilizer(Y, Z);
#[static_init::dynamic]
pub static YW_STABILIZER: [ElemId; 4] = plane_stabilizer(Y, W);
#[static_init::dynamic]
pub static ZW_STABILIZER: [ElemId; 4] = plane_stabilizer(Z, W);

/// Elements in the group of rotations of a hypercube.
#[static_init::dynamic]
pub static HYPERCUBE_ROTATIONS: [ElemId; 192] = elems_iter().collect_array().unwrap();
/// Elements in the group of rotations of a cube (as a subgroup of BC4).
#[static_init::dynamic]
pub static CUBE_ROTATIONS: [ElemId; 24] = *W_STABILIZER;

/// Element from the grip group.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ElemId(u8);

impl ElemId {
    pub const IDENT: Self = IDENT;

    /// Returns the inverse element.
    #[inline]
    pub fn inv(self) -> ElemId {
        group::CHIRAL_BC4.inv_elem[self.0 as usize]
    }

    #[inline]
    pub fn transform<T: TransformByElem>(self, obj: T) -> T {
        obj.transform_by(self)
    }

    /// Constructs an element from an ID.
    ///
    /// # Panics
    ///
    /// Panics if `id` is out of range (must be less [`group::ELEM_COUNT`]).
    #[inline]
    pub const fn new(id: u8) -> Self {
        assert!(id < group::ELEM_COUNT as u8, "element ID out of range");
        Self(id)
    }
    /// Hint to the compiler that the grip ID is within bounds.
    #[inline]
    pub const fn hint_assert_in_bounds(self) {
        // SAFETY: `ElemId` is only ever constructed using `ElemId::new()`,
        // which panics if the ID is greater than or equal to
        // [`group::ELEM_COUNT`].
        unsafe { std::hint::assert_unchecked(self.0 < group::ELEM_COUNT as u8) }
    }

    /// Returns the internal ID.
    #[inline]
    pub const fn id(self) -> u8 {
        self.0
    }

    #[inline]
    pub fn from_id(id: u8) -> Option<Self> {
        (id < group::ELEM_COUNT as u8).then_some(Self(id))
    }

    /// Constructs an element from `id`.
    ///
    /// # Safety
    ///
    /// `id` must be strictly less than [`group::ELEM_COUNT`].
    #[inline]
    pub unsafe fn from_id_unchecked(id: u8) -> Self {
        Self(id)
    }
}

impl fmt::Display for ElemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut grips_moved = super::grips::HYPERCUBE_GRIPS
            .into_iter()
            .filter(|&g| *self * g != g);
        if let Some(g1) = grips_moved.next() {
            let g2 = *self * g1;
            let g3 = *self * g2;
            write!(f, "{g1} -> {g2} -> {g3}")?;
            if g1 == g3
                && let Some(g4) = grips_moved.find(|&g| g != g2)
            {
                let g5 = *self * g4;
                let g6 = *self * g5;
                write!(f, ", {g4} -> {g5} -> {g6}")?;
            }
            Ok(())
        } else {
            write!(f, "{self:?}")
        }
    }
}

impl Mul for ElemId {
    type Output = ElemId;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        self.hint_assert_in_bounds();
        rhs.hint_assert_in_bounds();
        group::CHIRAL_BC4.mul_elem_elem[self.id() as usize][rhs.id() as usize]
    }
}

impl Mul<Vec4> for ElemId {
    type Output = Vec4;

    #[inline]
    fn mul(self, rhs: Vec4) -> Self::Output {
        self.hint_assert_in_bounds();
        let mat = group::CHIRAL_BC4.mul_elem_vec[self.id() as usize];
        mat[0] * rhs.x + mat[1] * rhs.y + mat[2] * rhs.z + mat[3] * rhs.w
    }
}

pub trait TransformByElem {
    fn transform_by(self, elem: ElemId) -> Self;
}

impl TransformByElem for ElemId {
    #[inline]
    fn transform_by(self, elem: ElemId) -> Self {
        elem * self * elem.inv()
    }
}

fn elems_iter() -> impl Iterator<Item = ElemId> {
    (0..group::ELEM_COUNT).map(|i| ElemId::new(i as u8))
}

fn vector_stabilizer(v: Vec4) -> [ElemId; 24] {
    group::stabilizer(elems_iter(), v).collect_array().unwrap()
}
fn plane_stabilizer(v1: Vec4, v2: Vec4) -> [ElemId; 4] {
    group::stabilizer(vector_stabilizer(v1), v2)
        .collect_array()
        .unwrap()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn test_simple_rotations() {
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

    impl Arbitrary for ElemId {
        type Parameters = ();

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            (0..group::ELEM_COUNT as u8).prop_map(|id| Self::from_id(id).unwrap())
        }

        type Strategy = proptest::strategy::Map<std::ops::Range<u8>, fn(u8) -> ElemId>;
    }
}
