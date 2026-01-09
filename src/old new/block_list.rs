use std::{
    collections::{BTreeSet, HashSet},
    fmt,
};

use itertools::Itertools;

use crate::{ElemId, GripId, Piece, StackVec, Twist, new::block_layer_mask::BlockLayerMask};

const MAX_BLOCK_COUNT: usize = 20;

#[derive(Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BlockList {
    /// Number of blocks.
    ///
    /// If this exceeds `MAX_BLOCK_COUNT`, then overflow has occurred.
    len: u8,
    attitudes: [ElemId; MAX_BLOCK_COUNT],
    layer_masks: [BlockLayerMask; MAX_BLOCK_COUNT],
}
impl fmt::Debug for BlockList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockList")
            .field(
                "blocks",
                &std::iter::zip(self.attitudes, self.layer_masks)
                    .take(self.len as usize)
                    .collect_vec(),
            )
            .finish()
    }
}
impl BlockList {
    pub const EMPTY: Self = Self {
        len: 0,
        attitudes: [ElemId::IDENT; MAX_BLOCK_COUNT],
        layer_masks: [BlockLayerMask::EMPTY; MAX_BLOCK_COUNT],
    };

    /// Returns the number of blocks.
    pub fn len(self) -> u8 {
        self.len
    }

    /// Returns whether the list is empty.
    pub fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Applies a twist.
    ///
    /// Returns [`BlockList::EMPTY`] if there are too many blocks.
    #[must_use]
    pub fn twist(self, twist: Twist) -> Self {
        let mut builder = BlockListBuilder::default();

        // Split blocks and apply twist
        for i in 0..self.len as usize {
            let attitude = self.attitudes[i];
            let [active, inactive] = self.layer_masks[i].split(twist.grip);
            if !inactive.is_empty() {
                builder.push(attitude, inactive);
            }
            if !active.is_empty() {
                builder.push(twist.transform * attitude, twist.transform * active);
            }
        }

        builder.build()
    }

    /// Applies `setup_moves` to each piece in `block` and then adds all the
    /// pieces into the puzzle state, except for the ones that are already in
    /// the puzzle state.
    ///
    /// `block` must have the identity attitude.
    ///
    /// Returns `None` if the puzzle state would have more than
    /// [`crate::MAX_BLOCKS`] blocks.
    #[must_use]
    pub fn add_block_with_setup_moves(
        self,
        setup_moves: &[Twist],
        layer_mask: BlockLayerMask,
    ) -> Option<Self> {
        // TODO: revisit this and optimize it

        // BlockListBuilder::
        // let mut blocks = setup_moves.iter().fold(vec![(ElemId::IDENT, layer_mask)], |blocks, twist| {
        //     blocks.into_iter().flat_map(|(attitude, layer_mask)|{
        //         let [inside, outside] = layer_mask.split(twist.grip);
        //         [(!inside.is_empty()).then(|| (twist.transform* attitude, twist.transform * inside)),
        //         (!outside.is_empty()).then_some( (attitude, outside))
        //         ]
        //     })
        // })

        // TODO: extract into function on BlockLayerMask
        let pieces_from_block = |layer_mask: BlockLayerMask| {
            let mut blocks = vec![layer_mask];
            for g in GripId::ALL {
                // TODO: optimize this
                blocks = blocks
                    .into_iter()
                    .flat_map(|b| b.split(g))
                    .filter(|b| !b.is_empty())
                    .collect();
            }
            blocks
                .into_iter()
                .map(|b| Piece::new_solved(b.active_grips().iter()))
        };

        let mut new_pieces = pieces_from_block(layer_mask).collect::<BTreeSet<Piece>>();
        for i in 0..self.len as usize {
            let old_block_at_solved = self.attitudes[i].inv() * self.layer_masks[i];
            for piece in pieces_from_block(old_block_at_solved) {
                new_pieces.remove(&piece);
            }
        }

        let init_piece = |new_piece| setup_moves.iter().fold(new_piece, |p, &twist| twist * p);

        let mut builder = BlockListBuilder::from_block_list(self);
        for piece in new_pieces {
            let p = init_piece(piece);
            builder.push(p.attitude, BlockLayerMask::from_piece(p));
        }

        builder.build().if_nonempty()
    }

    pub fn if_nonempty(self) -> Option<Self> {
        (!self.is_empty()).then_some(self)
    }

    pub fn combined_layer_mask(self) -> BlockLayerMask {
        self.layer_masks[..self.len as usize]
            .iter()
            .fold(BlockLayerMask::EMPTY, |a, &b| a | b)
    }

    /// Returns a list of `[body, head]` pairings. Each block contains its
    /// attitude and its layer mask when solved.
    pub fn pairings(self) -> impl Iterator<Item = [(ElemId, BlockLayerMask); 2]> {
        // self.layer_masks
        [todo!()].into_iter()
    }

    pub fn layer_masks_at_solved(self) -> StackVec<BlockLayerMask, MAX_BLOCK_COUNT> {
        let mut ret = StackVec::new();
        // TODO: maybe cache these? (doesn't change much when doing twists)
        for i in 0..self.len {
            // TODO: inline this multiplication to remove double-invert
            ret = ret
                .push(self.attitudes[i as usize].inv() * self.layer_masks[i as usize])
                .unwrap();
        }
        ret
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
        if i < MAX_BLOCK_COUNT {
            self.attitudes[i] = attitude;
            self.layer_masks[i] = layer_mask;
        }
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
    fn from_block_list(list: BlockList) -> Self {
        let mut ret = Self::default();
        for i in 0..list.len as usize {
            ret.push(list.attitudes[i], list.layer_masks[i]);
        }
        ret
    }

    /// Adds a block to the list and records its rank.
    fn push(&mut self, attitude: ElemId, layer_mask: BlockLayerMask) {
        debug_assert!(!layer_mask.is_empty());

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

        // Merge blocks until we reach a fixed point
        while self.merge_blocks() {}

        // Canonicalize by sorting blocks
        self.sort_blocks()
    }

    fn piece_count(&self) -> usize {
        let mut total = 0;
        for r in &self.rank_masks {
            for i in r.clone() {
                total += self.inner.layer_masks[i as usize].piece_count();
            }
        }
        total
    }

    /// Merges all blocks that can be merged.
    ///
    /// Returns whether any blocks were merged.
    #[inline]
    fn merge_blocks(&mut self) -> bool {
        let mut any_merged = false;

        #[cfg(debug_assertions)]
        let old_piece_count = self.piece_count();

        // It's important to iterate from largest to smallest rank, so that we
        // prioritize blocks where attitudes must match exactly (e.g.,
        // corner-edge) vs. blocks where many attitudes are indistinguishable
        // (e.g., center-core).
        for body_rank in (0..4).rev() {
            let head_rank = body_rank + 1;
            let body_candidates = self.rank_masks[body_rank as usize].clone();
            for body_index in body_candidates {
                let body_attitude = self.inner.attitudes[body_index as usize];
                let body_layer_mask = self.inner.layer_masks[body_index as usize];

                let head_candidates = self.rank_masks[head_rank as usize].clone();
                'loop_per_head: for head_index in head_candidates {
                    let head_attitude = self.inner.attitudes[head_index as usize];
                    let head_layer_mask = self.inner.layer_masks[head_index as usize];

                    // TODO: try with & without branching

                    let attitude_matches = match body_rank {
                        // core + center always matches
                        0 => true,

                        // center + ridge matches if the ridge attitude preserves the grip of the center
                        1 => {
                            let g = body_layer_mask.active_axes().unwrap_one().grips()[0];
                            head_attitude * g == g
                        }

                        // ridge + edge has 4 indistinguishable ridge attitudes
                        2 => get_ridge_indistinguishable_subgroup(body_layer_mask)
                            .into_iter()
                            .any(|e| e * body_attitude == head_attitude),

                        // edge + corner requires exact attitude match
                        3 => body_attitude == head_attitude,

                        _ => unreachable!(),
                    };

                    let merged_layer_mask = body_layer_mask.merge(head_layer_mask);
                    if attitude_matches && !merged_layer_mask.is_empty() {
                        any_merged = true;
                        // Replace head (attitude stays the same)
                        self.inner.layer_masks[head_index as usize] = merged_layer_mask;
                        // Remove body
                        self.rank_masks[body_rank].clear(body_index);
                        self.inner.layer_masks[body_index as usize] = BlockLayerMask::EMPTY;

                        #[cfg(debug_assertions)]
                        debug_assert_eq!(old_piece_count, self.piece_count(), "lost pieces");

                        break 'loop_per_head;
                    }
                }
            }
        }
        any_merged
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
                debug_assert!(
                    !blocks_present.get(layer_mask.radix_sort_key()),
                    "duplicate radix sort key",
                );
                blocks_present.set(layer_mask.radix_sort_key());
                ret.len += 1;
            }
        }

        // Add blocks in order.
        for i in 0..self.inner.len {
            let layer_mask = self.inner.layer_masks[i as usize];
            if !layer_mask.is_empty() {
                let j = blocks_present.bits_before(layer_mask.radix_sort_key()) as usize;
                ret.attitudes[j] = self.inner.attitudes[i as usize];
                ret.layer_masks[j] = layer_mask;
            }
        }

        ret
    }
}

/// Array of 32 booleans, packed into a `u32`.
#[derive(Default, Clone, PartialEq, Eq, Hash)]
struct BitSet32(u32);
impl fmt::Debug for BitSet32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BitSet32")
            .field(&self.clone().into_iter().collect_vec())
            .finish()
    }
}
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
impl IntoIterator for BitSet32 {
    type Item = u8;

    type IntoIter = BitSetIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        BitSetIter {
            b: self,
            f: |this| (!this.is_empty()).then(|| this.pop_index()),
        }
    }
}

/// Array of 96 booleans, packed into 3 `u32`s
#[derive(Default, Clone, PartialEq, Eq, Hash)]
struct BitSet96([u32; 3]);
impl fmt::Debug for BitSet96 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BitSet96")
            .field(&self.clone().into_iter().collect_vec())
            .finish()
    }
}
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
impl IntoIterator for BitSet96 {
    type Item = u8;

    type IntoIter = BitSetIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        BitSetIter {
            b: self,
            f: |this| (!this.is_empty()).then(|| this.pop_index()),
        }
    }
}

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

    #[test]
    fn test_add_block() {
        let init_block = BlockLayerMask::from_bits_handle_empty(0x0FF);
        let state = BlockList::default()
            .add_block_with_setup_moves(&[], init_block)
            .unwrap();
        assert_eq!(state.len(), 1);
        assert_eq!(state.layer_masks[0], init_block);

        const RU: Twist = Twist::new(crate::R, crate::WZ);
        const IR: Twist = Twist::new(crate::I, crate::ZY);
        let state = BlockList::default()
            .add_block_with_setup_moves(&[RU, IR], init_block)
            .unwrap();
        assert_eq!(state.len(), 3);
    }
}
