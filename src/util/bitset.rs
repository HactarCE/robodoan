//! Bitsets of fixed sizes.

use std::fmt;

use itertools::Itertools;

/// Array of 32 booleans, packed into a `u32`.
///
/// # Panics
///
/// All methods that take an index panic in debug mode if the index is out of
/// range.
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct BitSet32(u32);

impl fmt::Debug for BitSet32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BitSet32")
            .field(&self.clone().into_iter().collect_vec())
            .finish()
    }
}

impl BitSet32 {
    /// Empty bitset
    pub const EMPTY: Self = Self(0);

    /// Returns whether the bitset is empty.
    pub fn is_empty(&self) -> bool {
        *self == Self::EMPTY
    }
    /// Returns a bit from the bitset.
    pub fn get(&self, index: u8) -> bool {
        debug_assert!(index < 32);
        self.0 & (1 << index) != 0
    }
    /// Sets a bit to 1 in the bitset.
    pub fn set(&mut self, index: u8) {
        debug_assert!(index < 32);
        self.0 |= 1 << index;
    }
    /// Clears a bit to 0 in the bitset.
    pub fn clear(&mut self, index: u8) {
        debug_assert!(index < 32);
        self.0 &= !(1 << (index % 32));
    }

    /// Clears a bit, panicking in debug mode if it was already cleared.
    pub fn clear_from_1(&mut self, index: u8) {
        debug_assert!(self.get(index));
        self.clear(index);
    }
    /// Sets a bit, panicking in debug mode if it was already set.
    pub fn set_from_0(&mut self, index: u8) {
        debug_assert!(!self.get(index));
        self.set(index);
    }

    /// Returns the number of set bits.
    pub fn count_ones(&self) -> u32 {
        self.0.count_ones()
    }

    /// Returns the number of set bits before `index`.
    pub fn bits_before(&mut self, index: u8) -> u8 {
        (self.0 & mask_lowest_n_bits(index)).count_ones() as u8
    }

    /// Returns an iterator over the set bits in the bit set.
    pub fn iter(&self) -> BitSetIter<Self> {
        self.clone().into_iter()
    }
}

impl IntoIterator for &BitSet32 {
    type Item = u8;

    type IntoIter = BitSetIter<BitSet32>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for BitSet32 {
    type Item = u8;

    type IntoIter = BitSetIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        BitSetIter {
            b: self,
            f: |this| {
                (!this.is_empty()).then(|| {
                    // Return and clear the next set index
                    let i = this.0.trailing_zeros() as u8;
                    this.clear(i);
                    i
                })
            },
        }
    }
}

/// Array of 96 booleans, packed into 3 `u32`s.
///
/// # Panics
///
/// All methods that take an index panic in debug mode if the index is out of
/// range.
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct BitSet96([u32; 3]);

impl fmt::Debug for BitSet96 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BitSet96")
            .field(&self.clone().into_iter().collect_vec())
            .finish()
    }
}

impl BitSet96 {
    /// Empty bitset
    pub const EMPTY: Self = Self([0; 3]);

    /// Returns whether the bitset is empty.
    pub fn is_empty(&self) -> bool {
        *self == Self::EMPTY
    }
    /// Returns a bit from the bitset.
    pub fn get(&self, index: u8) -> bool {
        debug_assert!(index < 96);
        self.0[index as usize / 32] & (1 << (index % 32)) != 0
    }
    /// Sets a bit to 1 in the bitset.
    pub fn set(&mut self, index: u8) {
        debug_assert!(index < 96);
        self.0[index as usize / 32] |= 1 << (index % 32);
    }
    /// Clears a bit to 0 in the bitset.
    pub fn clear(&mut self, index: u8) {
        debug_assert!(index < 96);
        self.0[index as usize / 32] &= !(1 << (index % 32));
    }

    /// Clears a bit, panicking in debug mode if it was already cleared.
    pub fn clear_from_1(&mut self, index: u8) {
        debug_assert!(self.get(index));
        self.clear(index);
    }
    /// Sets a bit, panicking in debug mode if it was already set.
    pub fn set_from_0(&mut self, index: u8) {
        debug_assert!(!self.get(index));
        self.set(index);
    }

    /// Returns the number of set bits.
    pub fn count_ones(&self) -> u32 {
        let [b0, b1, b2] = self.0;
        b0.count_ones() + b1.count_ones() + b2.count_ones()
    }

    /// Returns the number of set bits before `index`.
    pub fn bits_before(&mut self, index: u8) -> u8 {
        let i0 = index;
        let i1 = index.max(32) - 32;
        let i2 = index.max(64) - 64;
        (self.0[0] & mask_lowest_n_bits(i0)).count_ones() as u8
            + (self.0[1] & mask_lowest_n_bits(i1)).count_ones() as u8
            + (self.0[2] & mask_lowest_n_bits(i2)).count_ones() as u8
    }

    /// Returns an iterator over the set bits in the bit set.
    pub fn iter(&self) -> BitSetIter<Self> {
        self.clone().into_iter()
    }
}

impl IntoIterator for &BitSet96 {
    type Item = u8;

    type IntoIter = BitSetIter<BitSet96>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for BitSet96 {
    type Item = u8;

    type IntoIter = BitSetIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        BitSetIter {
            b: self,
            f: |this| {
                (!this.is_empty()).then(|| {
                    // Return and clear the next set index
                    let [b0, b1, b2] = this.0;
                    let i = b0.trailing_zeros() as u8
                        + if b0 == 0 {
                            b1.trailing_zeros() as u8
                                + if b1 == 0 {
                                    b2.trailing_zeros() as u8
                                } else {
                                    0
                                }
                        } else {
                            0
                        };
                    this.clear(i);
                    i
                })
            },
        }
    }
}

/// Iterator over bit indices in a bitset.
#[derive(Copy, Clone)]
pub struct BitSetIter<B> {
    b: B,
    f: fn(&mut B) -> Option<u8>,
}

impl<B> Iterator for BitSetIter<B> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        (self.f)(&mut self.b)
    }
}

fn mask_lowest_n_bits(n: u8) -> u32 {
    if n < u32::BITS as u8 {
        (1 << n) - 1
    } else {
        u32::MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_array_96() {
        let expected = [0, 1, 2, 5, 17, 54, 80, 95];
        let mut bits = BitSet96::default();
        for &n in &expected {
            bits.set(n);
        }

        let actual = bits.iter().collect_vec();
        assert_eq!(actual, expected);

        for i in 0..96 {
            let expected_filtered = expected.iter().filter(|&&j| j < i).count() as u8;
            assert_eq!(expected_filtered, bits.bits_before(i as u8));
        }
    }

    #[test]
    fn test_bool_array_32() {
        let expected = [0, 1, 2, 5, 17, 22, 31];
        let mut bits = BitSet32::default();
        for &n in &expected {
            bits.set(n);
        }

        let actual = bits.iter().collect_vec();
        assert_eq!(actual, expected);
    }
}
