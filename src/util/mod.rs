//! Utilities

pub mod bitset;
pub mod stackvec;

/// Extends `v` with default values until it contains `index`.
pub fn extend_vec_to_index<T: Default>(v: &mut Vec<T>, index: usize) {
    if v.len() <= index {
        v.resize_with(index + 1, T::default);
    }
}
