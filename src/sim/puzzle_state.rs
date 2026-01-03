use std::u8;

use cgmath::{InnerSpace, vec4};
use itertools::Itertools;

use crate::{ELEM_COUNT, ElemId, GripId, GripSet, HYPERCUBE_ROTATIONS, IDENT, Twist, Vec4};

const INDICES_FOR_GRIP: [[u8; 26]; 8] = [
    indices_for_grip(GripId::new(0)),
    indices_for_grip(GripId::new(1)),
    indices_for_grip(GripId::new(2)),
    indices_for_grip(GripId::new(3)),
    indices_for_grip(GripId::new(4)),
    indices_for_grip(GripId::new(5)),
    indices_for_grip(GripId::new(6)),
    indices_for_grip(GripId::new(7)),
];

#[static_init::dynamic]
static MUL_ELEM_INDEX: [[u8; 72]; ELEM_COUNT] = gen_mul_elem_index_table();

/// Mapping from 0..81 to 0..72
const ADJUST_INDEX: [u8; 81] = gen_adjust_index_table();

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PuzzleState {
    /// Piece attitudes, excluding 1 core and 8 centers.
    ///
    /// Index corresponds to *current* piece location, not original piece
    /// location. Pieces are permuted during twists.
    piece_attitudes: [ElemId; 72],
}
impl Default for PuzzleState {
    fn default() -> Self {
        Self {
            piece_attitudes: [IDENT; 72],
        }
    }
}
impl PuzzleState {
    pub fn do_twist(&mut self, twist: Twist) {
        let mut ret = self.clone();

        // permute & reorient
        for old_index in INDICES_FOR_GRIP[twist.grip.id() as usize] {
            let new_index = MUL_ELEM_INDEX[twist.transform.id() as usize][old_index as usize];
            ret.piece_attitudes[new_index as usize] =
                twist.transform * self.piece_attitudes[old_index as usize];
        }

        *self = ret;
    }
    pub fn do_twists(&mut self, twists: &[Twist]) {
        for &twist in twists {
            self.do_twist(twist);
        }
    }
    pub fn unoriented_pieces(&self, last_layer: GripId) -> [usize; 3] {
        let piece_indices = INDICES_FOR_GRIP[last_layer.id() as usize];
        let is_piece_unoriented = |&i: &usize| {
            self.piece_attitudes[piece_indices[i] as usize] * last_layer.vec() != last_layer.vec()
        };

        let ridges = [4, 10, 12, 13, 15, 21]
            .into_iter()
            .filter(is_piece_unoriented)
            .count();
        let edges = [1, 3, 5, 7, 9, 11, 14, 16, 18, 20, 22, 24]
            .into_iter()
            .filter(is_piece_unoriented)
            .count();
        let corners = [0, 2, 6, 8, 17, 19, 23, 25]
            .into_iter()
            .filter(is_piece_unoriented)
            .count();

        [ridges, edges, corners]
    }

    pub fn triplet_score_on_axis(&self, axis: u8) -> usize {
        todo!()
        // let [g, _] = GripId::pair_on_axis(axis as usize);
        // for i in const { non_corner_non_core_pieces() } {
        //     let new_axis = (self.piece_attitudes[i as usize].inv() * g).axis();
        // }
        // for (i, piece) in self.piece_attitudes.iter().enumerate() {
        //     piece
        // }
    }
}

const fn indices_for_grip(g: GripId) -> [u8; 26] {
    let mut strides = [1, 3, 9, 27];
    strides.swap(g.axis_deprecated(), 0);
    let init = strides[0]
        * match g.signum() {
            1 => 0,
            -1 => 2,
            _ => unreachable!(),
        };
    let mut ret = [0; 26];
    let mut i = 0;
    while i < ret.len() {
        let j = i + (i >= 13) as usize; // skip center
        let x = j % 3;
        let y = (j / 3) % 3;
        let z = (j / 9) % 3;
        ret[i] = adjust_index(init + x * strides[1] + y * strides[2] + z * strides[3]).unwrap();
        i += 1;
    }
    ret
}

fn gen_mul_elem_index_table() -> [[u8; 72]; ELEM_COUNT] {
    HYPERCUBE_ROTATIONS.map(|elem| {
        itertools::iproduct!(-1..=1, -1..=1, -1..=1, -1..=1)
            .filter_map(|(w, z, y, x)| vec4_to_index(elem * vec4(x, y, z, w)))
            .collect_array()
            .unwrap()
    })
}

fn vec4_to_index(v: Vec4) -> Option<u8> {
    adjust_index((v + vec4(1, 1, 1, 1)).dot(vec4(1, 3, 9, 27)) as usize)
}

const fn adjust_index(mut i: usize) -> Option<u8> {
    let excluded_pieces = [
        13, // O center [1, 1, 1, 0]
        31, // F center [1, 1, 0, 1]
        37, // U center [1, 0, 1, 1]
        39, // R center [0, 1, 1, 1]
        40, //   core   [1, 1, 1, 1]
        41, // L center [2, 1, 1, 1]
        43, // D center [1, 2, 1, 1]
        49, // B center [1, 1, 2, 1]
        67, // I center [1, 1, 1, 2]
    ];

    let mut j = excluded_pieces.len() - 1;
    loop {
        if i == excluded_pieces[j] {
            return None;
        }
        if i > excluded_pieces[j] {
            i -= 1;
        }

        if j == 0 {
            return Some(i as u8);
        }
        j -= 1;
    }
}

const fn gen_adjust_index_table() -> [u8; 81] {
    let mut ret = [u8::MAX; 81];
    let mut i = 0;
    while i < 81 {
        if let Some(j) = adjust_index(i) {
            ret[i] = j;
        }
        i += 1;
    }
    ret
}

/// Returns a list of edges using 0..81 indexes
const fn edges() -> [u8; 32] {
    pieces_with_sticker_count(3)
}
/// Returns a list of ridges using 0..81 indexes
const fn ridges() -> [u8; 24] {
    pieces_with_sticker_count(2)
}
/// Returns a list of centers using 0..81 indexes
const fn centers() -> [u8; 8] {
    pieces_with_sticker_count(1)
}

const fn pieces_with_sticker_count<const N: usize>(sticker_count: u8) -> [u8; N] {
    let mut ret = [0; N];
    let mut i = 0;
    let mut j = 0;
    while i < 32 {
        if count_stickers_for_index_81(j) == sticker_count {
            ret[i] = j;
            i += 1;
        }
        j += 1;
    }
    ret
}

const fn count_stickers_for_index_81(i: u8) -> u8 {
    let [x, y, z, w] = [i % 3, (i / 3) % 3, (i / 9) % 3, i / 27];
    (!x & 1) + (!y & 1) + (!z & 1) + (!w & 1)
}

/// Returns a list of pieces in 0..81 that are not corner pieces or the core.
const fn non_corner_non_core_pieces() -> [u8; 64] {
    let mut ret = [0; 64];
    let mut i = 0;
    let mut j = 0;
    while i < 64 {
        let [x, y, z, w] = [j % 3, (j / 3) % 3, (j / 9) % 3, j / 27];
        j += 1;
        if (x | y | z | w) & 1 == 0 {
            continue; // corner
        }
        if j == 1 + 3 + 9 + 27 {
            continue; //core
        }
        ret[i] = j;
        i += 1;
    }
    ret
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::HYPERCUBE_GRIPS;

    #[test]
    fn test_all_pieces_indices() {
        for g1 in HYPERCUBE_GRIPS {
            for g2 in HYPERCUBE_GRIPS {
                if g1 == g2 {
                    continue;
                }
                let set1: HashSet<_> = HashSet::from_iter(INDICES_FOR_GRIP[g1.id() as usize]);
                let set2: HashSet<_> = HashSet::from_iter(INDICES_FOR_GRIP[g2.id() as usize]);
                let expected_intersection = if g1.axis_deprecated() == g2.axis_deprecated() { 0 } else { 9 };
                assert_eq!(expected_intersection, set1.intersection(&set2).count());
            }
        }
    }

    #[test]
    fn test_unoriented_pieces() {
        let mut state = PuzzleState::default();
        state.do_twists(&crate::parse_twists("R U R' U R U2 R'"));
        assert_eq!(state.unoriented_pieces(crate::D), [0, 0, 0]);
        assert_eq!(state.unoriented_pieces(crate::U), [0, 3, 6]);
    }
}
