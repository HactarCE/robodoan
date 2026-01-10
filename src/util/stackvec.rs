//! Short vector stored on the stack.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Deref, DerefMut, Index, IndexMut, RangeTo};

/// Vector with a limited capacity, stored on the stack.
///
/// The length is stored as a `u8` to save on space.
#[derive(Copy, Clone)]
#[must_use = "StackVec methods return a new value rather than modifying their input"]
pub struct StackVec<T, const CAP: usize> {
    len: u8,
    elems: [T; CAP],
}

impl<T: PartialOrd, const CAP: usize> PartialOrd for StackVec<T, CAP> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(self.as_slice(), other.as_slice())
    }
}

impl<T: Ord, const CAP: usize> Ord for StackVec<T, CAP> {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(self.as_slice(), other.as_slice())
    }
}

impl<T: PartialEq, const CAP: usize> PartialEq for StackVec<T, CAP> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq, const CAP: usize> Eq for StackVec<T, CAP> {}

impl<T: fmt::Debug, const CAP: usize> fmt::Debug for StackVec<T, CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: Default + Copy, const CAP: usize> Default for StackVec<T, CAP> {
    fn default() -> Self {
        assert!(CAP <= u8::MAX as usize, "capacity too big"); // somehow this encourages optimizations
        Self {
            len: 0,
            elems: [T::default(); CAP],
        }
    }
}

impl<T: Default + Copy, const CAP: usize> StackVec<T, CAP> {
    /// Constructs a new empty vector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Constructs a vector from values in a slice.
    pub fn from_slice(array: &[T]) -> Option<Self> {
        let mut ret = Self::default();
        if array.len() > CAP {
            return None; // doesn't fit
        }
        ret.elems[..array.len()].copy_from_slice(array);
        ret.len = array.len() as u8;
        Some(ret)
    }

    /// Pushes an element onto the vector and returns it, or returns `None` in
    /// case of overflow.
    pub fn push(mut self, elem: T) -> Option<Self> {
        *self.elems.get_mut(self.len as usize)? = elem;
        self.len += 1;
        Some(self)
    }

    /// Applies a function to every element in the vector.
    pub fn map<U: Default + Copy>(self, f: impl FnMut(T) -> U) -> StackVec<U, CAP> {
        StackVec::from_iter(self.into_iter().map(f)).unwrap()
    }

    /// Extends the vector with elements from an iterator.
    pub fn extend(mut self, iter: impl IntoIterator<Item = T>) -> Option<Self> {
        let iter = iter.into_iter();

        let (lo, _) = iter.size_hint();
        if self.len as usize + lo > CAP {
            return None; // definitely won't fit
        }

        for elem in iter {
            self = self.push(elem)?;
        }
        Some(self)
    }

    /// Constructs a vector from an iterator, or returns `None` in case of
    /// overflow.
    #[allow(clippy::should_implement_trait)] // can't impl FromIterator<T> for Option<Self>
    pub fn from_iter(iter: impl IntoIterator<Item = T>) -> Option<Self> {
        Self::new().extend(iter)
    }
}

impl<T, const CAP: usize> StackVec<T, CAP> {
    /// Returns a slice containing the entire vector.
    pub fn as_slice(&self) -> &[T] {
        self // via deref
    }

    /// Returns a mutable slice containing the entire vector.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self // via deref_mut
    }

    /// Sorts the vector without preserving the initial ordering.
    pub fn sorted_unstable(mut self) -> Self
    where
        T: Ord,
    {
        self.sort_unstable();
        self
    }

    /// Sorts the vector by a key without preserving the initial ordering.
    pub fn sorted_unstable_by_key<K: Ord>(mut self, f: impl FnMut(&T) -> K) -> Self {
        self.sort_unstable_by_key(f);
        self
    }

    /// Sorts the vector with a comparison function without preserving the
    /// initial ordering.
    pub fn sorted_unstable_by(mut self, compare: impl FnMut(&T, &T) -> Ordering) -> Self {
        self.sort_unstable_by(compare);
        self
    }
}

impl<T, const CAP: usize> Deref for StackVec<T, CAP> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.elems[0..self.len as usize]
    }
}

impl<T, const CAP: usize> DerefMut for StackVec<T, CAP> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.elems[0..self.len as usize]
    }
}

impl<I, T, const CAP: usize> Index<I> for StackVec<T, CAP>
where
    [T]: Index<I>,
    [T]: Index<RangeTo<usize>, Output = [T]>, // shouldn't be necessary
{
    type Output = <[T] as Index<I>>::Output;

    #[track_caller]
    fn index(&self, index: I) -> &Self::Output {
        let len = self.len();
        // SAFETY: `len <= CAP` is an invariant of the data structure
        unsafe { std::hint::assert_unchecked(len <= CAP) };
        &self.elems[..len][index]
    }
}

impl<I, T, const CAP: usize> IndexMut<I> for StackVec<T, CAP>
where
    [T]: IndexMut<I>,
    [T]: IndexMut<RangeTo<usize>, Output = [T]>, // shouldn't be necessary
{
    #[track_caller]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        let len = self.len();
        // SAFETY: `len <= CAP` is an invariant of the data structure
        unsafe { std::hint::assert_unchecked(len <= CAP) };
        &mut self.elems[..len][index]
    }
}

impl<T, const CAP: usize> IntoIterator for StackVec<T, CAP> {
    type Item = T;

    type IntoIter = std::iter::Take<std::array::IntoIter<T, CAP>>;

    fn into_iter(self) -> Self::IntoIter {
        self.elems.into_iter().take(self.len as usize)
    }
}

impl<'a, T, const CAP: usize> IntoIterator for &'a StackVec<T, CAP> {
    type Item = &'a T;

    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.elems[0..self.len as usize].iter()
    }
}

impl<'a, T, const CAP: usize> IntoIterator for &'a mut StackVec<T, CAP> {
    type Item = &'a mut T;

    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.elems[0..self.len as usize].iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stackvec_merge() {
        let mut a = StackVec::<(u8, u8), 16>::new();
        a = a.push((1, 12)).unwrap();
        a = a.push((1, 6)).unwrap();
        a = a.push((2, 1)).unwrap();
        a = a.push((6, 92)).unwrap();
        a = a.push((1, 4)).unwrap();
        a = a.push((2, 9)).unwrap();
        a = a.push((3, 14)).unwrap();
        a = a.push((1, 3)).unwrap();
        a.sort_unstable();
    }
}
