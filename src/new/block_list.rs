use crate::{ElemId, Twist, new::block_layer_mask::BlockLayerMask};

const MAX_BLOCK_COUNT: usize = 20;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BlockList {
    /// Number of blocks.
    ///
    /// If this exceeds `MAX_BLOCK_COUNT`, then overflow has occurred.
    len: u8,
    attitudes: [ElemId; MAX_BLOCK_COUNT],
    layer_masks: [BlockLayerMask; MAX_BLOCK_COUNT],
}
impl BlockList {
    pub const EMPTY: Self = Self {
        len: 0,
        attitudes: [ElemId::IDENT; MAX_BLOCK_COUNT],
        layer_masks: [BlockLayerMask::EMPTY; MAX_BLOCK_COUNT],
    };

    /// Applies a twist.
    ///
    /// Returns [`BlockList::EMPTY`] if there are too many blocks.
    #[must_use]
    pub fn twist(self, twist: Twist) -> Self {
        let mut builder = BlockListBuilder::default();

        // Split blocks and apply twist
        for i in 0..self.len as usize {
            let attitude = self.attitudes[i];
            let [inside, outside] = self.layer_masks[i].split(twist.grip);
            if !outside.is_empty() {
                builder.push(attitude, outside);
            }
            if !inside.is_empty() {
                builder.push(twist.transform * attitude, twist.transform * inside);
            }
        }

        builder.build()
    }

    #[must_use]
    fn replace_overflow_with_empty(self) -> Self {
        if self.len as usize > MAX_BLOCK_COUNT {
            Self::EMPTY
        } else {
            self
        }
    }

    #[must_use]
    fn push_unchecked(mut self, attitude: ElemId, layer_mask: BlockLayerMask) -> Self {
        let i = self.len as usize;
        debug_assert!(i < MAX_BLOCK_COUNT);
        self.attitudes[i] = attitude;
        self.layer_masks[i] = layer_mask;
        self.len += 1;
        self
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct BlockListBuilder {
    /// List of blocks, unsorted.
    inner: BlockList,
    /// Bitmask indicating the rank of each block.
    rank_masks: [BitSet32; 5],
}
impl BlockListBuilder {
    /// Adds a block to the list and records its rank.
    fn push(&mut self, attitude: ElemId, layer_mask: BlockLayerMask) {
        self.inner.len += 1;
        if self.inner.len as usize > MAX_BLOCK_COUNT {
            return;
        }

        // Sort blocks by rank.
        let rank = layer_mask.rank() as usize;
        let i = self.inner.len.min(MAX_BLOCK_COUNT as u8);
        self.inner = self.inner.push_unchecked(attitude, layer_mask);
        self.rank_masks[rank].set(i);
    }

    /// Builds the list and merges blocks.
    fn build(mut self) -> BlockList {
        // Handle overflow
        if self.inner.len as usize > MAX_BLOCK_COUNT {
            return BlockList::EMPTY;
        }

        // Merge blocks
        self.merge_blocks();

        // Canonicalize by sorting blocks
        self.sort_blocks()
    }

    /// Merges all blocks that can be merged.
    #[inline]
    fn merge_blocks(&mut self) {
        for body_rank in 0..4 {
            let head_rank = body_rank + 1;
            let mut body_candidates = BitSet32::default();
            while !body_candidates.is_empty() {
                let body_index = body_candidates.pop_index();
                let body_attitude = self.inner.attitudes[body_index as usize];
                let body_layer_mask = self.inner.layer_masks[body_index as usize];

                let mut head_candidates = self.rank_masks[head_rank as usize].clone();
                'loop_per_head: while !head_candidates.is_empty() {
                    let head_index = head_candidates.pop_index();
                    let head_attitude = self.inner.attitudes[head_index as usize];
                    let head_layer_mask = self.inner.layer_masks[head_index as usize];

                    // TODO: try with & without branching

                    let attitude_matches = match body_rank {
                        // core + center always matches
                        0 => true,

                        // center + ridge matches if the ridge attitude preserves the grip of the center
                        1 => {
                            let g = body_layer_mask.active_axes().unwrap_one().grips()[0];
                            body_attitude * g == g
                        }

                        // ridge + edge has 4 indistinguishable ridge attitudes
                        2 => get_ridge_indistinguishable_subgroup(body_layer_mask)
                            .into_iter()
                            .any(|e| body_attitude * e == head_attitude),

                        // edge + corner requires exact attitude match
                        3 => body_attitude == head_attitude,

                        _ => unreachable!(),
                    };

                    let merged_layer_mask = body_layer_mask.merge(head_layer_mask);
                    if attitude_matches && !merged_layer_mask.is_empty() {
                        // Replace head (attitude stays the same)
                        self.inner.layer_masks[head_index as usize] = merged_layer_mask;
                        // Remove body
                        self.rank_masks[body_rank].clear(body_index);
                        self.inner.layer_masks[body_index as usize] = BlockLayerMask::EMPTY;
                        break 'loop_per_head;
                    }
                }
            }
        }
    }

    /// Sorts blocks by bit pattern using radix sort and removes empty blocks.
    ///
    /// Returns the result instead of modifying `self`.
    #[inline]
    #[must_use]
    fn sort_blocks(self) -> BlockList {
        let mut ret = BlockList::default();

        // Assemble a list of blocks, indexed by radix sort key.
        let mut blocks_present = BitSet96::default();
        for i in 0..self.inner.len {
            let layer_mask = self.inner.layer_masks[i as usize];
            if !layer_mask.is_empty() {
                blocks_present.set(layer_mask.radix_sort_key());
                ret.len += 1;
            }
        }

        // Add blocks in order.
        for i in 0..self.inner.len {
            let layer_mask = self.inner.layer_masks[i as usize];
            if !layer_mask.is_empty() {
                let j = blocks_present.bits_before(i) as usize;
                ret.attitudes[j] = self.inner.attitudes[i as usize];
                ret.layer_masks[j] = self.inner.layer_masks[i as usize];
            }
        }

        ret
    }
}

/// Array of 32 booleans, packed into a `u32`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
struct BitSet32(u32);
impl BitSet32 {
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
    pub fn set(&mut self, index: u8) {
        debug_assert!(index < 32);
        self.0 |= 1 << index;
    }
    pub fn clear(&mut self, index: u8) {
        debug_assert!(index < 32);
        self.0 &= !(1 << (index % 32));
    }
    /// Returns and clears the next set index.
    ///
    /// Panics if empty.
    pub fn pop_index(&mut self) -> u8 {
        let i = self.0.trailing_zeros() as u8;
        debug_assert!(i < 96);
        self.clear(i);
        i
    }
}

/// Array of 96 booleans, packed into 3 `u32`s
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
struct BitSet96([u32; 3]);
impl BitSet96 {
    pub fn is_empty(&self) -> bool {
        self.0 == [0; 3]
    }
    pub fn set(&mut self, index: u8) {
        debug_assert!(index < 96);
        self.0[index as usize / 32] |= 1 << (index % 32);
    }
    pub fn get(&self, index: u8) -> bool {
        debug_assert!(index < 96);
        self.0[index as usize / 32] & (1 << (index % 32)) != 0
    }
    pub fn clear(&mut self, index: u8) {
        debug_assert!(index < 96);
        self.0[index as usize / 32] &= !(1 << (index % 32));
    }
    /// Returns and clears the next set index.
    ///
    /// Panics if empty.
    pub fn pop_index(&mut self) -> u8 {
        let [b0, b1, b2] = self.0;
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
        self.clear(i);
        i
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
}

fn mask_lowest_n_bits(n: u8) -> u32 {
    if n < u32::BITS as u8 {
        (1 << n) - 1
    } else {
        u32::MAX
    }
}

fn get_ridge_indistinguishable_subgroup(layer_mask: BlockLayerMask) -> [ElemId; 4] {
    match layer_mask.active_axes().bits() {
        0b0011 => *crate::XY_STABILIZER,
        0b0110 => *crate::YZ_STABILIZER,
        0b1100 => *crate::ZW_STABILIZER,
        0b0101 => *crate::XZ_STABILIZER,
        0b1010 => *crate::YW_STABILIZER,
        0b1001 => *crate::XW_STABILIZER,
        _ => panic!("not a ridge"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_array_96() {
        let contents = [0, 1, 2, 5, 17, 54, 80, 95];
        let mut bits = BitSet96::default();
        for &n in &contents {
            bits.set(n);
        }
        {
            let mut bits = bits.clone();
            let mut actual = vec![];
            while !bits.is_empty() {
                actual.push(bits.pop_index());
            }
            assert_eq!(actual, contents);
        }

        for i in 0..96 {
            let expected = contents.iter().filter(|&&j| j < i).count() as u8;
            assert_eq!(expected, bits.bits_before(i as u8));
        }
    }

    #[test]
    fn test_bool_array_32() {
        let expected = [0, 1, 2, 5, 17, 22, 31];
        let mut bits = BitSet32::default();
        for &n in &expected {
            bits.set(n);
        }
        let mut actual = vec![];
        while !bits.is_empty() {
            actual.push(bits.pop_index());
        }
        assert_eq!(actual, expected);
    }
}
