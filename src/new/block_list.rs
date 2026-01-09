use std::{collections::BTreeSet, fmt, ops::Index};

use itertools::Itertools;

use crate::{
    ElemId, GripId, Piece, Twist,
    new::{block::Block, common::AxisSet},
};

const MAX_BLOCK_COUNT: u32 = 23;

#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct BlockList {
    /// List of blocks.
    blocks: [Block; MAX_BLOCK_COUNT as usize],
    /// Bitmask indicating, for each possible inner rank value, the indices of
    /// blocks with that inner rank.
    ranks: [BitSet32; 5],
    /// Bitmask indicating which radix sort keys exist.
    radix_sort_keys: BitSet96,
    /// Number of blocks.
    ///
    /// If this exceeds `MAX_BLOCK_COUNT`, then overflow has occurred.
    len: u32,
}

impl fmt::Debug for BlockList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockList")
            .field("blocks", &self.blocks())
            .field("ranks", &self.ranks)
            .field("radix_sort_keys", &self.radix_sort_keys)
            .field("len", &self.len)
            .finish()
    }
}

impl Index<u32> for BlockList {
    type Output = Block;

    fn index(&self, index: u32) -> &Self::Output {
        debug_assert!(index < self.len);
        &self.blocks[index as usize]
    }
}
impl Index<u8> for BlockList {
    type Output = Block;

    fn index(&self, index: u8) -> &Self::Output {
        &self[index as u32]
    }
}

impl BlockList {
    pub const EMPTY: Self = Self {
        blocks: [Block::EMPTY; MAX_BLOCK_COUNT as usize],
        ranks: [BitSet32::EMPTY; 5],
        radix_sort_keys: BitSet96::EMPTY,
        len: 0,
    };

    #[deprecated]
    pub fn radix_sort_keys(&self) -> String {
        format!("{:?}", self.radix_sort_keys)
    }

    #[deprecated]
    pub fn trunc(&mut self, n: usize) {
        for i in n as _..self.len {
            self.remove_block(i);
        }
        self.len = n as _;
    }

    /// Assertion that `std::mem::size_of::<Self>() == 128`.
    ///
    /// It doesn't matter that much, but it's nice to keep it small if we can.
    #[allow(unused)]
    const SIZE_ASSERT: [u8; 128] = [0; std::mem::size_of::<Self>()];

    /// Returns the number of blocks.
    pub fn len(&self) -> u32 {
        self.len
    }

    /// Returns whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns `Some(self)` if nonempty or `None` if empty.
    pub fn if_nonempty(self) -> Option<Self> {
        (!self.is_empty()).then_some(self)
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks[..self.len as usize]
    }

    pub(crate) fn possible_pairings(&self) -> impl '_ + Iterator<Item = [Block; 2]> {
        let body_ranks = (0..4).rev();
        body_ranks
            .flat_map(|body_rank| {
                let head_rank = body_rank + 1;
                let body_candidates = &self.ranks[body_rank as usize];
                let head_candidates = &self.ranks[head_rank as usize];
                itertools::iproduct!(body_candidates, head_candidates)
            })
            .map(|(body_index, head_index)| [self[body_index], self[head_index]])
            .filter(|&[body, head]| !Block::merge(body.at_solved(), head.at_solved()).is_empty())
    }

    pub(crate) fn blocks_with_rank(&self, rank: u8) -> impl '_ + Iterator<Item = Block> {
        debug_assert!(rank <= 4);
        self.ranks[rank as usize].iter().map(|i| self[i])
    }

    /// Adds a block to the end of the list and updates various bookkeeping
    /// fields.
    ///
    /// Returns an error and sets `self.len > MAX_BLOCK_COUNT` in case of
    /// overflow.
    pub(crate) fn push(&mut self, block: Block) -> Result<(), ()> {
        let i = self.len;

        // Update len
        self.len += 1;
        if self.len > MAX_BLOCK_COUNT {
            return Err(());
        }

        // Update ranks
        self.ranks[block.inner_rank() as usize].set_from_0(i as u8);

        // Update radix sort keys
        self.radix_sort_keys.set_from_0(block.radix_sort_key());

        // Update blocks
        self.blocks[i as usize] = block;

        Ok(())
    }

    /// Sets the block at the given index and updates the radix sort keys.
    ///
    /// The new block must have the same rank as the old block.
    fn set_block_with_same_rank(&mut self, index: u32, new: Block) {
        let old = self.blocks[index as usize];

        debug_assert_ne!(old, new);
        debug_assert_eq!(old.inner_rank(), new.inner_rank());

        self.radix_sort_keys.clear_from_1(old.radix_sort_key());
        self.radix_sort_keys.set_from_0(new.radix_sort_key());

        self.blocks[index as usize] = new;
    }
    /// Sets the block at the given index and updates all assorted fields.
    fn set_block(&mut self, index: u32, new: Block) {
        let old = self.blocks[index as usize];

        // Update ranks
        self.ranks[old.inner_rank() as usize].clear_from_1(index as u8);
        self.ranks[new.inner_rank() as usize].set_from_0(index as u8);

        // Update radix sort keys
        self.radix_sort_keys.clear_from_1(old.radix_sort_key());
        self.radix_sort_keys.set_from_0(new.radix_sort_key());

        // Update blocks
        self.blocks[index as usize] = new;
    }
    /// Sets a block to empty and updates all assorted fields except `self.len`.
    ///
    /// This leaves the block list in an invalid state which must be cleaned up
    /// using [`Self::cleanup()`].
    fn remove_block(&mut self, index: u32) {
        let old = self.blocks[index as usize];

        // Update ranks
        self.ranks[old.inner_rank() as usize].clear_from_1(index as u8);

        // Update radix sort keys
        self.radix_sort_keys.clear_from_1(old.radix_sort_key());

        // Update blocks
        self.blocks[index as usize] = Block::EMPTY;
    }

    /// Swaps two blocks, updating the various metadata.
    ///
    /// Panics in debug mode if `i != j`.
    fn swap_blocks(&mut self, i: u32, j: u32) {
        debug_assert_ne!(i, j);
        let ri = self[i].inner_rank();
        let rj = self[j].inner_rank();
        let i_is_empty = self[i].is_empty();
        let j_is_empty = self[j].is_empty();
        self.blocks.swap(i as usize, j as usize);
        if !i_is_empty {
            self.ranks[ri as usize].clear_from_1(i as u8);
        }
        if !j_is_empty {
            self.ranks[rj as usize].clear_from_1(j as u8);
        }
        if !i_is_empty {
            self.ranks[ri as usize].set_from_0(j as u8);
        }
        if !j_is_empty {
            self.ranks[rj as usize].set_from_0(i as u8);
        }
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
    pub fn add_block_with_setup_moves(&self, setup_moves: &[Twist], block: Block) -> Option<Self> {
        let mut ret = self.clone();

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

        // TODO: extract into function on Block
        let pieces_from_block_at_solved = |block: Block| {
            let mut blocks = vec![block];
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

        let mut new_pieces = pieces_from_block_at_solved(block).collect::<BTreeSet<Piece>>();
        for i in 0..self.len {
            for piece in pieces_from_block_at_solved(self[i]) {
                new_pieces.remove(&piece);
            }
        }

        let init_piece = |new_piece| setup_moves.iter().fold(new_piece, |p, &twist| twist * p);

        for piece in new_pieces {
            let transformed_piece = init_piece(piece);
            ret.push(transformed_piece.attitude * Block::from_piece(piece))
                .ok()?;
        }

        ret.cleanup();

        #[cfg(debug_assertions)]
        {
            let old = ret.clone();
            ret.cleanup();
            assert_eq!(ret, old);
        }

        ret.if_nonempty()
    }

    /// Applies a twist.
    ///
    /// Returns [`BlockList::EMPTY`] if there are too many blocks.
    #[must_use]
    pub fn twist(&self, twist: Twist) -> BlockList {
        debug_assert_ne!(twist.transform, ElemId::IDENT);

        let mut ret = self.clone();

        // Split blocks and apply twist.
        for i in 0..ret.len {
            let block = ret[i];
            let [active, inactive] = block.split(twist.grip);
            if active.is_empty() {
                continue; // no change
            } else {
                let active = twist.transform * active;
                if inactive.is_empty() {
                    ret.set_block_with_same_rank(i, active); // all active
                } else {
                    ret.set_block_with_same_rank(i, inactive); // inactive has same inner rank
                    if ret.push(active).is_err() {
                        return Self::EMPTY; // indicate error
                    }
                }
            }
        }

        ret.cleanup();

        #[cfg(debug_assertions)]
        {
            let old = ret.clone();
            ret.cleanup();
            assert_eq!(ret, old);
        }

        ret
    }

    /// Merges blocks and sorts them, canonicalizing the whole list.
    fn cleanup(&mut self) {
        // Merge blocks until we reach a fixed point
        while self.merge_blocks() {}

        // Sort blocks by `radix_sort_key`.
        #[cfg(debug_assertions)]
        let mut max_index = 0;
        for i in 0..self.len {
            loop {
                let block = self[i];
                if block.is_empty() {
                    break;
                }
                let j = self.radix_sort_keys.bits_before(block.radix_sort_key()) as u32;
                #[cfg(debug_assertions)]
                {
                    max_index = std::cmp::max(max_index, j);
                }
                if i == j {
                    break;
                } else {
                    self.swap_blocks(i, j);
                }
            }
        }

        #[cfg(debug_assertions)]
        {
            self.len = max_index + 1;
            debug_assert_eq!(self.len, self.radix_sort_keys.count_ones());
            debug_assert_eq!(
                self.len,
                self.ranks.iter().map(|r| r.count_ones()).sum::<u32>(),
            );
        }
        #[cfg(not(debug_assertions))]
        {
            self.len = self.radix_sort_keys.count_ones()
        }

        debug_assert!(self.blocks().is_sorted_by_key(|b| b.radix_sort_key()));
    }

    /// Merges all blocks that can be merged.
    ///
    /// Returns whether any blocks were merged.
    #[inline]
    fn merge_blocks(&mut self) -> bool {
        let mut any_merged = false;

        // It's important to iterate from largest to smallest rank, so that we
        // prioritize blocks where attitudes must match exactly (like
        // corner+edge) instead of blocks where many attitudes are
        // indistinguishable (like center+core).
        for body_rank in (0..4).rev() {
            let head_rank = body_rank + 1;
            let body_candidates = self.ranks[body_rank as usize].clone();
            for body_index in body_candidates {
                let body = self[body_index];

                let head_candidates = self.ranks[head_rank as usize].clone();
                'loop_per_head: for head_index in head_candidates {
                    let head = self[head_index];

                    let merged = Block::merge(body, head);

                    if !merged.is_empty() {
                        any_merged = true;
                        // Remove head
                        self.remove_block(head_index as u32);
                        // Replace body (inner rank stays the same)
                        self.set_block_with_same_rank(body_index as u32, merged);

                        break 'loop_per_head;
                    }
                }
            }
        }
        any_merged
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
    pub const EMPTY: Self = Self(0);

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
    pub fn get(&self, index: u8) -> bool {
        debug_assert!(index < 32);
        self.0 & (1 << index) != 0
    }
    pub fn set(&mut self, index: u8) {
        debug_assert!(index < 32);
        self.0 |= 1 << index;
    }
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

    /// Returns the number of bits set.
    pub fn count_ones(&self) -> u32 {
        self.0.count_ones()
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
    pub const EMPTY: Self = Self([0; 3]);

    pub fn is_empty(&self) -> bool {
        self.0 == [0; 3]
    }
    pub fn get(&self, index: u8) -> bool {
        debug_assert!(index < 96);
        self.0[index as usize / 32] & (1 << (index % 32)) != 0
    }
    pub fn set(&mut self, index: u8) {
        debug_assert!(index < 96);
        self.0[index as usize / 32] |= 1 << (index % 32);
    }
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

    /// Returns the number of bits set.
    pub fn count_ones(&self) -> u32 {
        let [b0, b1, b2] = self.0;
        b0.count_ones() + b1.count_ones() + b2.count_ones()
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
            f: |this| (!this.is_empty()).then(|| this.pop_index()),
        }
    }
}

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

fn get_ridge_indistinguishable_subgroup(active_axes: AxisSet) -> [ElemId; 4] {
    match active_axes.bits() {
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
        let init_block = Block::from_layer_bits(0x0FF);
        let state = BlockList::default()
            .add_block_with_setup_moves(&[], init_block)
            .unwrap();
        assert_eq!(state.len(), 1);
        assert_eq!(state[0_u32], init_block);

        const RU: Twist = Twist::new(crate::R, crate::WZ);
        const IR: Twist = Twist::new(crate::I, crate::ZY);
        let state = BlockList::default()
            .add_block_with_setup_moves(&[RU, IR], init_block)
            .unwrap();
        assert_eq!(state.len(), 3);
    }

    #[test]
    fn test_merge_2223() {
        let head = Block::from_layer_bits(0xc73);
        let body = Block::from_layer_bits(0x4fb);
        dbg!(head, body);
        let mut list = BlockList::default();
        list.push(body).unwrap();
        list.push(head).unwrap();
        list.cleanup();
        dbg!(Block::merge(body, head));
        assert_eq!(list.len(), 1);
    }
}
