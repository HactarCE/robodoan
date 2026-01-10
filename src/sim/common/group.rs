//! Chiral subgroup of Coxeter group BC4 (i.e., the group of rotations of a
//! hypercube).

use cgmath::Transform;
use itertools::Itertools;

use super::{Axis, Elem, Grip, Mat4, Vec4};

/// Number of elements in the group.
pub const ELEM_COUNT: usize = 192;

#[static_init::dynamic]
static CHIRAL_BC4: Group = Group::bc4();

/// Returns the inverse of an element of the grip group.
pub fn invert_elem(elem: Elem) -> Elem {
    CHIRAL_BC4.inv_elem[elem.id() as usize]
}

/// Composes two elements of the grip group.
pub fn mul_elem_elem(lhs: Elem, rhs: Elem) -> Elem {
    CHIRAL_BC4.mul_elem_elem[lhs.id() as usize][rhs.id() as usize]
}

/// Transforms a vector by an element of the grip group.
pub fn mul_elem_vec(lhs: Elem, rhs: Vec4) -> Vec4 {
    let mat = CHIRAL_BC4.mul_elem_vec[lhs.id() as usize];
    mat[0] * rhs.x + mat[1] * rhs.y + mat[2] * rhs.z + mat[3] * rhs.w
}

/// Transforms a grip by an element of the grip group.
pub fn mul_elem_grip(lhs: Elem, rhs: Grip) -> Grip {
    CHIRAL_BC4.mul_elem_grip[lhs.id() as usize][rhs.id() as usize]
}

/// Transforms an axis by an element of the grip group.
pub fn mul_elem_axis(lhs: Elem, rhs: Axis) -> Axis {
    Axis::new((CHIRAL_BC4.mul_elem_axis[lhs.id() as usize] >> (rhs.id() * 2)) & 0b11)
}

/// Grip group of the puzzle.
struct Group {
    inv_elem: [Elem; ELEM_COUNT],
    mul_elem_elem: [[Elem; ELEM_COUNT]; ELEM_COUNT],
    mul_elem_vec: [[Vec4; 4]; ELEM_COUNT],
    mul_elem_grip: [[Grip; 8]; ELEM_COUNT],
    mul_elem_axis: [u8; ELEM_COUNT],
}

impl Group {
    pub fn bc4() -> Self {
        use super::vectors::*;

        let ident = Mat4::from_cols(X, Y, Z, W);
        let xy = Mat4::from_cols(Y, -X, Z, W);
        let xz = Mat4::from_cols(Z, Y, -X, W);
        let xw = Mat4::from_cols(W, Y, Z, -X);

        let mut matrices = vec![ident];
        let mut last_unproc = 0;
        while last_unproc < matrices.len() {
            let init = matrices[last_unproc];
            for g in [xy, xz, xw] {
                let new = matmul(init, g);
                if !matrices.contains(&new) {
                    matrices.push(new);
                }
            }
            last_unproc += 1;
        }

        let elem_from_matrix = |q| Elem::new(matrices.iter().position(|&m| m == q).unwrap() as u8);

        let mul_elem_elem: [[Elem; ELEM_COUNT]; ELEM_COUNT] = matrices
            .iter()
            .map(|&a| {
                matrices
                    .iter()
                    .map(|&b| elem_from_matrix(matmul(a, b)))
                    .collect_array::<ELEM_COUNT>()
                    .unwrap()
            })
            .collect_array::<ELEM_COUNT>()
            .unwrap();

        let mul_elem_vec: [[Vec4; 4]; ELEM_COUNT] = matrices
            .iter()
            .map(|&m| [X, Y, Z, W].map(|v| vecmul(m, v)))
            .collect_array::<ELEM_COUNT>()
            .unwrap();

        let mul_elem_grip = mul_elem_vec.map(|mat| {
            Grip::ALL.map(|g| {
                Grip::ALL
                    .iter()
                    .position(|g2| g2.vec() == mat[g.axis().id() as usize] * g.signum())
                    .map(|i| Grip::new(i as u8))
                    .unwrap()
            })
        });

        let mul_elem_axis = mul_elem_grip.map(|grips| {
            grips[0].axis().id()
                | (grips[2].axis().id() << 2)
                | (grips[4].axis().id() << 4)
                | (grips[6].axis().id() << 6)
        });

        let inv_elem = matrices
            .iter()
            .map(|m| {
                elem_from_matrix(
                    m.cast::<f32>()
                        .unwrap()
                        .inverse_transform()
                        .unwrap()
                        .cast::<i8>()
                        .unwrap(),
                )
            })
            .collect_array()
            .unwrap();

        Self {
            inv_elem,
            mul_elem_elem,
            mul_elem_vec,
            mul_elem_grip,
            mul_elem_axis,
        }
    }
}

fn matmul(a: Mat4, b: Mat4) -> Mat4 {
    (a.cast::<f32>().unwrap() * b.cast::<f32>().unwrap())
        .cast()
        .unwrap()
}

fn vecmul(a: Mat4, b: Vec4) -> Vec4 {
    (a.cast::<f32>().unwrap() * b.cast::<f32>().unwrap())
        .cast()
        .unwrap()
}
