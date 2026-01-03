//! MC4D log file compat

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use cgmath::vec4;
use itertools::Itertools;

use crate::StackVec;
use crate::sim::*;

const MAGIC_STRING: &str = "MagicCube4D";
const LOG_VERSION: &str = "3";
const RUBIKS_4D_SCHALFLI_SYMBOL: &str = "{4,3,3}";
const LAYER_COUNT: &str = "3";

#[static_init::dynamic]
static TWIST_FROM_MC4D_STICKER_ID: Vec<Option<Twist>> = mc4d_twist_order();

#[static_init::dynamic]
static TWIST_TO_MC4D: HashMap<Twist, Mc4dTwist> = {
    let mc4d_twists = TWIST_FROM_MC4D_STICKER_ID
        .iter()
        .enumerate()
        .filter_map(|(i, &twist)| Some((i, twist?)));

    let mut twist_to_mc4d: HashMap<Twist, Mc4dTwist> = mc4d_twists
        .clone()
        .map(|(i, twist)| (twist, Mc4dTwist::new(i, 1, 1)))
        .collect();

    // Add 180-degree ridge turns
    for (i, twist) in mc4d_twists {
        twist_to_mc4d
            .entry(Twist {
                grip: twist.grip,
                transform: twist.transform * twist.transform,
            })
            .or_insert(Mc4dTwist::new(i, 2, 1));
    }

    twist_to_mc4d
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Mc4dTwist {
    sticker: usize,
    multiplier: i8,
    layer_mask: u8,
}
impl fmt::Display for Mc4dTwist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            sticker,
            multiplier,
            layer_mask,
        } = self;
        write!(f, "{sticker},{multiplier},{layer_mask}")
    }
}
impl FromStr for Mc4dTwist {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let [sticker_str, multiplier_str, layer_mask_str] =
            s.split(",").collect_array().ok_or(())?;
        Ok(Self {
            sticker: sticker_str.parse().map_err(|_| ())?,
            multiplier: multiplier_str.parse().map_err(|_| ())?,
            layer_mask: layer_mask_str.parse().map_err(|_| ())?,
        })
    }
}
impl Mc4dTwist {
    pub fn new(sticker: usize, multiplier: i8, layer_mask: u8) -> Self {
        Self {
            sticker,
            multiplier,
            layer_mask,
        }
    }
    pub fn to_layered_twist(self) -> LayeredTwist {
        let Twist { grip, transform: t } =
            TWIST_FROM_MC4D_STICKER_ID[self.sticker].expect("bad MC4D twist");

        let transform = std::iter::repeat_n(
            if self.multiplier < 0 { t.inv() } else { t },
            self.multiplier.unsigned_abs() as usize,
        )
        .fold(IDENT, |a, b| a * b);

        LayeredTwist {
            twist: Twist { grip, transform },
            layer_mask: self.layer_mask,
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
struct LayeredTwist {
    twist: Twist,
    layer_mask: u8,
}
impl LayeredTwist {
    pub fn to_twists(self, puzzle_offset: &mut ElemId) -> StackVec<Twist, 2> {
        let Self {
            mut twist,
            mut layer_mask,
        } = self;

        // middle slice
        let middle_slice = layer_mask & 0b010 != 0;
        if middle_slice {
            // Rotate the whole puzzle
            *puzzle_offset = *puzzle_offset * self.twist.transform.inv();

            // Do the opposite twist on the other layers
            twist = twist.inv();
            layer_mask ^= 0b111;
        }

        let mut ret = StackVec::new();

        // original layer
        if layer_mask & 0b001 != 0 {
            ret = ret.push(twist).unwrap();
        }

        // opposite layer
        if layer_mask & 0b100 != 0 {
            let twist_on_opposite = Twist {
                grip: twist.grip.opposite(),
                transform: twist.transform,
            };
            ret = ret.push(twist_on_opposite).unwrap();
        }

        ret.map(|t| puzzle_offset.transform(t))
    }
}

pub struct Mc4dScramble {
    scramble_state: String,
    view_matrix: String,
    mc4d_scramble: Vec<Mc4dTwist>,

    scramble: Vec<Twist>,
    puzzle_offset_from_scramble: ElemId,
}
impl FromStr for Mc4dScramble {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = match s.rsplit_once("m|") {
            Some((before_boundary, _after_boundary)) => before_boundary,
            None => s,
        };

        let mut lines = s.lines();
        let header = lines.next().ok_or("missing header")?;
        let segments = header.split_whitespace().collect_vec();

        if segments.len() != 6 || segments[0] != MAGIC_STRING {
            return Err("bad header");
        }

        if segments[1] != LOG_VERSION {
            return Err("unsupported log version");
        }

        let scramble_state = segments[2].to_string();

        // Ignore move count (`segments[3]`)

        // Check puzzle Schlafli symbol and edge length
        if segments[4] != RUBIKS_4D_SCHALFLI_SYMBOL || segments[5] != LAYER_COUNT {
            return Err("unsupport puzzle; only 3x3x3x3 is supported");
        }

        let view_matrix = (&mut lines).take(4).join("\n");

        if lines.next() != Some("*") {
            return Err("missing `*` separator");
        }

        let mut mc4d_scramble = vec![];
        for line in lines {
            for move_str in line
                .split_whitespace()
                .map(|s| s.trim_end_matches('.').trim())
                .filter(|s| !s.is_empty())
            {
                mc4d_scramble
                    .push(Mc4dTwist::from_str(move_str).map_err(|()| "error parsing move")?);
            }
        }

        let mut scramble = vec![];
        let mut puzzle_offset = IDENT;
        for &mc4d_twist in &mc4d_scramble {
            scramble.extend(mc4d_twist.to_layered_twist().to_twists(&mut puzzle_offset));
        }

        Ok(Self {
            scramble_state,
            view_matrix,
            mc4d_scramble,

            scramble,
            puzzle_offset_from_scramble: puzzle_offset,
        })
    }
}
impl Mc4dScramble {
    pub fn to_string(&self, solved: bool, solve_twists: Vec<Twist>) -> String {
        let move_count = solve_twists.len();
        let state = if solved { "3" } else { &self.scramble_state };
        let mut log_file_string = format!(
            "{MAGIC_STRING} {LOG_VERSION} {state} {move_count} {RUBIKS_4D_SCHALFLI_SYMBOL} {LAYER_COUNT}\n"
        );
        log_file_string += &self.view_matrix;
        log_file_string += "\n*";

        let mut n = 0;
        let mut add_twist_sep = |s: &mut String| {
            *s += if n % 10 == 0 { "\n" } else { " " };
            n += 1;
        };

        for &mc4d_twist in &self.mc4d_scramble {
            add_twist_sep(&mut log_file_string);
            log_file_string += &mc4d_twist.to_string();
        }

        log_file_string += " m|";

        let offset = self.puzzle_offset_from_scramble.inv();
        for twist in solve_twists {
            add_twist_sep(&mut log_file_string);
            log_file_string += &TWIST_TO_MC4D[&offset.transform(twist)].to_string();
        }

        log_file_string + "."
    }

    pub fn scramble(&self) -> &[Twist] {
        &self.scramble
    }
}

const UNIT_VECTORS: [Vec4; 4] = [X, Y, Z, W];

fn mc4d_twist_order() -> Vec<Option<Twist>> {
    let seed_twists = [
        ((I, vec4(-1, 0, 0, 0)), TWISTS_FROM_NAME["IR"]),
        ((I, vec4(-1, -1, 0, 0)), TWISTS_FROM_NAME["IUR"]),
        ((I, vec4(-1, -1, -1, 0)), TWISTS_FROM_NAME["IUFR"]),
    ];
    let twist_from_grip_and_fixed_vec: HashMap<(GripId, Vec4), Twist> =
        itertools::iproduct!(*HYPERCUBE_ROTATIONS, seed_twists)
            .map(|(elem, ((grip, sticker_vector), twist))| {
                ((elem * grip, elem * sticker_vector), elem.transform(twist))
            })
            .collect();

    // ported from HSC1
    [I, B, D, L, R, U, F, O]
        .into_iter()
        .flat_map(|grip| {
            let twist_from_grip_and_fixed_vec = &twist_from_grip_and_fixed_vec;

            let mut basis = basis_faces(grip);
            basis.sort_by_key(|f| f.axis_deprecated()); // order: X, Y, Z, W
            basis.reverse(); // order: W, Z, Y, X
            let mc4d_basis_1 = UNIT_VECTORS[basis[0].axis_deprecated()];
            let mc4d_basis_2 = UNIT_VECTORS[basis[1].axis_deprecated()];
            let mc4d_basis_3 = UNIT_VECTORS[basis[2].axis_deprecated()];

            let piece_locations =
                itertools::iproduct!([-1, 0, 1], [-1, 0, 1], [-1, 0, 1]).map(|(x, y, z)| [x, y, z]);
            let corners = piece_locations.clone().filter(|v| abs_sum(v) == 3);
            let edges = piece_locations.clone().filter(|v| abs_sum(v) == 2);
            let ridges = piece_locations.filter(|v| abs_sum(v) == 1);
            let center = std::iter::once([0, 0, 0]);
            let mc4d_order_piece_locations = corners.chain(edges).chain(ridges).chain(center);

            mc4d_order_piece_locations.map(move |mc4d_coords_of_sticker_within_face: [i8; 3]| {
                let fixed_vector = ZERO
                    + mc4d_basis_1 * mc4d_coords_of_sticker_within_face[0]
                    + mc4d_basis_2 * mc4d_coords_of_sticker_within_face[1]
                    + mc4d_basis_3 * mc4d_coords_of_sticker_within_face[2];
                twist_from_grip_and_fixed_vec
                    .get(&(grip, fixed_vector))
                    .copied()
            })
        })
        .collect()
}

fn abs_sum<const N: usize>(xs: &[i8; N]) -> i8 {
    xs.map(|x| x.abs()).iter().sum()
}

fn basis_faces(g: GripId) -> [GripId; 3] {
    let w = match g.signum() {
        1 => O,
        -1 => I,
        _ => unreachable!(),
    };

    [
        if g.axis_deprecated() == 0 { w } else { R },
        if g.axis_deprecated() == 1 { w } else { U },
        if g.axis_deprecated() == 2 { w } else { F },
    ]
}
