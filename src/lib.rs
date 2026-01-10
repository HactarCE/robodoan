//! Fewest-moves solver for the 4-dimensional 3x3x3x3 Rubik's cube.

#[macro_use]
mod macros;
pub mod mc4d;
pub mod search;
pub mod sim;
pub mod util;

pub use search::*;
pub use util::stackvec::StackVec;
