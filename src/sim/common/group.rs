use cgmath::Transform;
use itertools::Itertools;

use super::elements::ElemId;
use super::grips::{GripId, HYPERCUBE_GRIPS};
use super::space::*;

pub const ELEM_COUNT: usize = 192;

/// Chiral subgroup of Coxeter group BC4 (i.e., the group of rotations of a
/// hypercube).
#[static_init::dynamic]
pub static CHIRAL_BC4: Group = Group::bc4();

/// Grip group of the puzzle.
pub struct Group {
    pub inv_elem: [ElemId; ELEM_COUNT],
    pub mul_elem_elem: [[ElemId; ELEM_COUNT]; ELEM_COUNT],
    pub mul_elem_vec: [[Vec4; 4]; ELEM_COUNT],
    pub mul_elem_grip: [[GripId; 8]; ELEM_COUNT],
}
impl Group {
    pub fn bc4() -> Self {
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

        let elem_from_matrix =
            |q| ElemId::new(matrices.iter().position(|&m| m == q).unwrap() as u8);

        let mul_elem_elem: [[ElemId; ELEM_COUNT]; ELEM_COUNT] = matrices
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
            HYPERCUBE_GRIPS.map(|g| {
                HYPERCUBE_GRIPS
                    .iter()
                    .position(|g2| g2.vec() == mat[g.axis_deprecated()] * g.signum())
                    .map(|i| GripId::new(i as u8))
                    .unwrap()
            })
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
        }
    }
}

pub fn matmul(a: Mat4, b: Mat4) -> Mat4 {
    (a.cast::<f32>().unwrap() * b.cast::<f32>().unwrap())
        .cast()
        .unwrap()
}

pub fn vecmul(a: Mat4, b: Vec4) -> Vec4 {
    (a.cast::<f32>().unwrap() * b.cast::<f32>().unwrap())
        .cast()
        .unwrap()
}

pub fn fixes_vector(e: ElemId, v: Vec4) -> bool {
    e * v == v
}

pub fn stabilizer(
    group: impl IntoIterator<Item = ElemId>,
    v: Vec4,
) -> impl Iterator<Item = ElemId> {
    group.into_iter().filter(move |&e| fixes_vector(e, v))
}
