use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use itertools::Itertools;
use rand::Rng;
use rand::seq::IndexedRandom;

use super::element::*;
use super::grip::*;

#[static_init::dynamic]
static TWIST_NAMES: HashMap<Twist, String> = twist_names_4d();

#[static_init::dynamic]
static TWIST_FROM_NAME: HashMap<String, Twist> =
    TWIST_NAMES.iter().map(|(t, s)| (s.clone(), *t)).collect();

#[static_init::dynamic]
pub static ALL_TWISTS: Vec<Twist> = Grip::ALL.into_iter().flat_map(|g| g.twists()).collect();

/// Parses a space-separated list of twist names.
pub fn parse_twists(s: &str) -> Vec<Twist> {
    s.split_whitespace()
        .map(|word| word.parse().expect("unknown twist"))
        .collect()
}

/// Returns a sequence of random twists.
pub fn random_twists(rng: &mut impl Rng, count: usize) -> Vec<Twist> {
    (0..count)
        .map(move |_| ALL_TWISTS.choose(rng).copied().unwrap())
        .collect()
}

/// Twist of a 4-dimensional Rubik's cube
#[derive(
    Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, bytemuck::Zeroable, bytemuck::Pod,
)]
#[repr(C)]
pub struct Twist {
    /// Grip affected by the twist.
    pub grip: Grip,
    /// Transform applied to pieces by the twist.
    pub transform: Elem,
}

impl FromStr for Twist {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        TWIST_FROM_NAME.get(s).copied().ok_or(())
    }
}

impl Twist {
    /// Constructs a twist.
    ///
    /// Panics if the twist transform does not fix the grip.
    pub fn new(grip: Grip, transform: Elem) -> Self {
        assert_eq!(transform * grip, grip, "twist transform does not fix grip");
        Self { grip, transform }
    }

    /// Returns the grip affected by the twist.
    pub fn grip(self) -> Grip {
        self.grip
    }

    /// Returns the transform applied to pieces by the twist.
    pub fn transform(self) -> Elem {
        self.transform
    }

    /// Returns the inverse twist.
    #[must_use]
    pub fn inv(self) -> Self {
        Self::new(self.grip, self.transform.inv())
    }
}

impl fmt::Debug for Twist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{:?}]", self.grip, self.transform)
    }
}

impl fmt::Display for Twist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match TWIST_NAMES.get(self) {
            Some(s) => write!(f, "{s}"),
            None => write!(f, "{self:?}"),
        }
    }
}

impl TransformByElem for Twist {
    fn transform_by(self, elem: Elem) -> Self {
        Twist {
            grip: elem * self.grip,
            transform: elem.transform(self.transform),
        }
    }
}

fn twist_names_4d() -> HashMap<Twist, String> {
    use super::elements::*;
    use super::grips::*;

    let iu = Twist {
        grip: I,
        transform: XZ,
    };
    let iu2 = Twist {
        grip: I,
        transform: XZ * XZ,
    };
    let iur = Twist {
        grip: I,
        transform: YX * XZ * XZ,
    };
    let iurf = Twist {
        grip: I,
        transform: ZY * YX,
    };

    let mut ret = HashMap::new();
    for offset in Elem::iter_all() {
        let i = offset * I;
        let u = offset * U;
        let r = offset * R;
        let f = offset * F;
        ret.entry(offset.transform(iu)).or_insert(format!("{i}{u}"));
        ret.entry(offset.transform(iu2))
            .or_insert(format!("{i}{u}2"));
        let [a, b] = hsc1_sort([u, r]);
        ret.entry(offset.transform(iur))
            .or_insert(format!("{i}{a}{b}"));
        let [a, b, c] = hsc1_sort([u, r, f]);
        ret.entry(offset.transform(iurf))
            .or_insert(format!("{i}{a}{b}{c}"));
    }
    ret
}

/// Sort a list of unique grips according to the order used in HSC1 log files.
fn hsc1_sort<const N: usize>(grips: [Grip; N]) -> [Grip; N] {
    use super::grips::*;

    [U, D, F, B, R, L, O, I]
        .into_iter()
        .filter(|g| grips.contains(g))
        .collect_array()
        .expect("duplicate grips in twist name")
}
