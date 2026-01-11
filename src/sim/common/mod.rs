//! Common types and functions for puzzle simulation.

mod axis;
mod axis_set;
mod element;
mod grip;
mod grip_set;
pub mod group;
mod piece;
mod twist;

pub use axis::Axis;
pub use axis_set::AxisSet;
pub use element::{Elem, TransformByElem};
pub use grip::Grip;
pub use grip_set::GripSet;
pub use piece::Piece;
pub use twist::{ALL_TWISTS, Twist, parse_twists, random_twists};

/// 4x4 integer matrix
pub type Mat4 = cgmath::Matrix4<i8>;
/// Integer vector in 4D space
pub type Vec4 = cgmath::Vector4<i8>;

/// [`Axis`] constants
pub mod axes {
    use super::Axis;

    /// X axis
    pub const X: Axis = Axis::new(0);
    /// Y axis
    pub const Y: Axis = Axis::new(1);
    /// Z axis
    pub const Z: Axis = Axis::new(2);
    /// W axis
    pub const W: Axis = Axis::new(3);
}

/// [`Grip`] constants
pub mod grips {
    use super::Grip;

    /// Right grip (+X)
    pub const R: Grip = Grip::new(0);
    /// Left grip (-X)
    pub const L: Grip = Grip::new(1);
    /// Up grip (+Y)
    pub const U: Grip = Grip::new(2);
    /// Down grip (-Y)
    pub const D: Grip = Grip::new(3);
    /// Front grip (+Z)
    pub const F: Grip = Grip::new(4);
    /// Back grip (-Z)
    pub const B: Grip = Grip::new(5);
    /// Out grip (+W)
    pub const O: Grip = Grip::new(6);
    /// In grip (-W)
    pub const I: Grip = Grip::new(7);

    /// Positive grips (R, U, F, O) in canonical order
    pub const RUFO: [Grip; 4] = [R, U, F, O];
    /// Negative grips (L, D, B, I) in canonical order
    pub const LDBI: [Grip; 4] = [L, D, B, I];
}

/// [`Elem`] constants
pub mod elements {
    #![allow(missing_docs)]

    use super::axes::*;
    use super::{Axis, Elem};

    /// Identity group element.
    pub const IDENT: Elem = Elem::new(0);
    /// Rotation from +X to +Y.
    pub const XY: Elem = Elem::new(1);
    /// Rotation from +X to +Z.
    pub const XZ: Elem = Elem::new(2);
    /// Rotation from +X to +W.
    pub const XW: Elem = Elem::new(3);

    /// Rotation from +Y to +X.
    pub const YX: Elem = Elem::new(13); // XY * XY * XY
    /// Rotation from +Z to +X.
    pub const ZX: Elem = Elem::new(25); // XZ * XZ * XZ
    /// Rotation from +W to +X.
    pub const WX: Elem = Elem::new(36); // XW * XW * XW

    /// Rotation from +Y to +Z.
    pub const YZ: Elem = Elem::new(97); // XZ * YX * ZX
    /// Rotation from +Y to +W.
    pub const YW: Elem = Elem::new(110); // XW * YX * WX
    /// Rotation from +Z to +Y.
    pub const ZY: Elem = Elem::new(84); // XY * ZX * YX
    /// Rotation from +W to +Y.
    pub const WY: Elem = Elem::new(86); // XY * WX * YX

    /// Rotation from +Z to +W.
    pub const ZW: Elem = Elem::new(133); // XW * ZX * WX
    /// Rotation from +W to +Z.
    pub const WZ: Elem = Elem::new(128); // XZ * WX * ZX

    /// Rotations in the XY plane, including identity.
    #[static_init::dynamic]
    pub static XY_STABILIZER: [Elem; 4] = plane_stabilizer(X, Y);
    /// Rotations in the XZ plane, including identity.
    #[static_init::dynamic]
    pub static XZ_STABILIZER: [Elem; 4] = plane_stabilizer(X, Z);
    /// Rotations in the XW plane, including identity.
    #[static_init::dynamic]
    pub static XW_STABILIZER: [Elem; 4] = plane_stabilizer(X, W);
    /// Rotations in the YZ plane, including identity.
    #[static_init::dynamic]
    pub static YZ_STABILIZER: [Elem; 4] = plane_stabilizer(Y, Z);
    /// Rotations in the YW plane, including identity.
    #[static_init::dynamic]
    pub static YW_STABILIZER: [Elem; 4] = plane_stabilizer(Y, W);
    /// Rotations in the ZW plane, including identity.
    #[static_init::dynamic]
    pub static ZW_STABILIZER: [Elem; 4] = plane_stabilizer(Z, W);

    fn plane_stabilizer(u: Axis, v: Axis) -> [Elem; 4] {
        use itertools::Itertools;

        let g1 = u.pos_grip();
        let g2 = v.pos_grip();

        Elem::iter_all()
            .filter(|&e| e * g1 == g1 && e * g2 == g2)
            .collect_array()
            .unwrap()
    }
}

/// [`Vec4`] constants
pub mod vectors {
    pub use cgmath::vec4;

    use super::Vec4;

    /// Zero vector
    pub const ZERO: Vec4 = vec4(0, 0, 0, 0);
    /// Unit vector along X
    pub const X: Vec4 = vec4(1, 0, 0, 0);
    /// Unit vector along Y
    pub const Y: Vec4 = vec4(0, 1, 0, 0);
    /// Unit vector along Z
    pub const Z: Vec4 = vec4(0, 0, 1, 0);
    /// Unit vector along W
    pub const W: Vec4 = vec4(0, 0, 0, 1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mul_elem_axis() {
        for e in Elem::iter_all() {
            for a in Axis::ALL {
                assert_eq!(e * a, (e * a.pos_grip()).axis())
            }
        }
    }
}
