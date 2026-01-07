use std::collections::HashMap;
use std::fmt;

use itertools::Itertools;

use super::elements::*;
use super::grips::*;

#[static_init::dynamic]
pub static TWIST_NAMES_3D: HashMap<Twist, String> = twist_names_3d();
#[static_init::dynamic]
pub static TWIST_NAMES_4D: HashMap<Twist, String> = twist_names_4d();

#[static_init::dynamic]
pub static TWISTS_FROM_NAME: HashMap<String, Twist> =
    itertools::chain(&*TWIST_NAMES_3D, &*TWIST_NAMES_4D)
        .map(|(t, s)| (s.clone(), *t))
        .collect();

#[derive(Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Twist {
    pub grip: GripId,
    pub transform: ElemId,
}
impl Twist {
    pub const fn new(grip: GripId, transform: ElemId) -> Self {
        Self { grip, transform }
    }

    #[must_use]
    pub fn inv(self) -> Self {
        Self::new(self.grip, self.transform.inv())
    }

    pub(crate) fn assert_is_valid(&self) {
        #[cfg(debug_assertions)]
        assert_eq!(
            self.grip,
            self.transform * self.grip,
            "transform does not fix grip",
        );
    }
}
impl fmt::Debug for Twist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]", self.grip, self.transform)
    }
}
impl fmt::Display for Twist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if crate::USE_3D_TWIST_NAMES
            && let Some(s) = TWIST_NAMES_3D.get(self)
        {
            write!(f, "{s}")
        } else if let Some(s) = TWIST_NAMES_4D.get(self) {
            write!(f, "{s}")
        } else {
            write!(f, "{self:?}")
        }
    }
}

impl TransformByElem for Twist {
    #[inline]
    fn transform_by(self, elem: ElemId) -> Self {
        Twist {
            grip: elem * self.grip,
            transform: elem.transform(self.transform),
        }
    }
}

fn twist_names_3d() -> HashMap<Twist, String> {
    let r = Twist::new(R, ZY);
    let r2 = Twist::new(R, ZY * ZY);
    let r3 = Twist::new(R, YZ);

    let mut ret = HashMap::new();
    for offset in *CUBE_ROTATIONS {
        let r_grip = offset * R;
        ret.entry(offset.transform(r))
            .or_insert(format!("{r_grip}"));
        ret.entry(offset.transform(r2))
            .or_insert(format!("{r_grip}2"));
        ret.entry(offset.transform(r3))
            .or_insert(format!("{r_grip}'"));
    }
    ret
}

fn twist_names_4d() -> HashMap<Twist, String> {
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
    for offset in *HYPERCUBE_ROTATIONS {
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
fn hsc1_sort<const N: usize>(grips: [GripId; N]) -> [GripId; N] {
    [U, D, F, B, R, L, O, I]
        .into_iter()
        .filter(|g| grips.contains(g))
        .collect_array()
        .expect("duplicate grips in twist name")
}
