use std::collections::BTreeSet;
use std::fmt;
use std::ops::Index;

use super::{Block, BlockListMeta};
use crate::sim::common::*;
use crate::util::bitset::BitSet32;

/// Maxmimum number of blocks that can be stored.
const MAX_BLOCK_COUNT: u32 = 26;

/// Partial puzzle state, stored as a list of blocks.
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct BlockList {
    /// List of blocks.
    blocks: [Block; MAX_BLOCK_COUNT as usize],
    /// Bitmap indicating, for each possible inner rank value, the indices of
    /// blocks with that inner rank.
    inner_ranks: [BitSet32; 5],
    /// Packed metadata.
    meta: BlockListMeta,
}

/// Assertion of `std::mem::size_of::<BlockList>()`.
///
/// It doesn't matter that much, but it's nice to keep it small if we can.
const _SIZE_ASSERT: [u8; 128] = [0; std::mem::size_of::<BlockList>()];

impl fmt::Debug for BlockList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockList")
            .field("blocks", &self.blocks())
            .field("inner_ranks", &self.inner_ranks)
            .field("meta", &self.meta)
            .finish()
    }
}

impl Index<u32> for BlockList {
    type Output = Block;

    fn index(&self, index: u32) -> &Self::Output {
        debug_assert!(index < self.len());
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
    /// Maxmimum number of blocks that can be stored
    pub const MAX_LEN: u32 = MAX_BLOCK_COUNT;

    /// Empty block list
    pub const EMPTY: Self = Self {
        blocks: [Block::EMPTY; MAX_BLOCK_COUNT as usize],
        inner_ranks: [BitSet32::EMPTY; 5],
        meta: BlockListMeta::DEFAULT,
    };

    /// Returns the number of blocks.
    pub fn len(&self) -> u32 {
        self.meta.block_count() as u32
    }

    /// Returns whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `Some(self)` if nonempty or `None` if empty.
    pub fn if_nonempty(self) -> Option<Self> {
        (!self.is_empty()).then_some(self)
    }

    /// Returns the list of blocks.
    pub fn blocks(&self) -> &[Block] {
        &self.blocks[..self.len() as usize]
    }

    /// Returns an iterator over blocks with a specific inner rank.
    pub fn blocks_with_inner_rank(&self, inner_rank: u8) -> impl '_ + Iterator<Item = Block> {
        self.inner_ranks[inner_rank as usize]
            .iter()
            .map(|i| self[i])
    }

    /// Returns the twist count.
    pub fn twist_count(&self) -> u16 {
        self.meta.twist_count()
    }

    /// Adds a block to the end of the list and updates the rank bitmaps.
    ///
    /// Returns an error and sets `self.len > MAX_BLOCK_COUNT` in case of
    /// overflow.
    fn push(&mut self, block: Block) -> Result<(), ()> {
        let i = self.len();

        // Update len
        self.meta.increment_block_count();
        if self.len() > MAX_BLOCK_COUNT {
            return Err(());
        }

        // Update ranks
        self.inner_ranks[block.inner_rank() as usize].set_from_0(i as u8);

        // Update blocks
        self.blocks[i as usize] = block;

        Ok(())
    }

    /// Sets the block at the given index.
    ///
    /// The new block must have the same rank as the old block.
    fn set_block_with_same_rank(&mut self, index: u32, new: Block) {
        let old = self.blocks[index as usize];

        debug_assert_ne!(old, new);
        debug_assert_eq!(old.inner_rank(), new.inner_rank());

        self.blocks[index as usize] = new;
    }
    /// Sets a block to empty and updates all assorted fields except `self.len`.
    ///
    /// This leaves the block list in an invalid state which must be cleaned up
    /// using [`Self::cleanup()`].
    fn remove_block(&mut self, index: u32) {
        let old = self.blocks[index as usize];

        // Update ranks
        self.inner_ranks[old.inner_rank() as usize].clear_from_1(index as u8);

        // Update blocks
        self.blocks[index as usize] = Block::EMPTY;
    }

    /// Applies `setup_moves` to each piece in `block` and then adds all the
    /// pieces into the puzzle state, except for the ones that are already in
    /// the puzzle state.
    ///
    /// The attitude of `block` is ignored.
    ///
    /// Returns [`BlockList::EMPTY`] if there are too many blocks.
    #[must_use]
    pub fn add_block_with_setup_moves(&self, setup_moves: &[Twist], block: Block) -> Self {
        let mut ret = self.clone();

        // TODO: extract into function on Block
        let pieces_from_block_at_solved = |block: Block| {
            let mut blocks = vec![block];
            for g in Grip::ALL {
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
        for i in 0..self.len() {
            for piece in pieces_from_block_at_solved(self[i]) {
                new_pieces.remove(&piece);
            }
        }

        let init_piece = |new_piece| setup_moves.iter().fold(new_piece, |p, &twist| twist * p);

        for piece in new_pieces {
            let transformed_piece = init_piece(piece);
            if ret
                .push(transformed_piece.attitude * Block::from(piece))
                .is_err()
            {
                return Self::EMPTY; // indicate error
            };
        }

        ret.cleanup();

        ret
    }

    /// Applies a twist.
    ///
    /// Returns [`BlockList::EMPTY`] if there are too many blocks.
    #[must_use]
    pub fn twist(&self, twist: Twist) -> BlockList {
        debug_assert_ne!(twist.transform, Elem::IDENT);

        let mut ret = self.clone();

        ret.meta.count_twist_on_grip(twist.grip);

        // Split blocks and apply twist.
        for i in 0..ret.len() {
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

        ret
    }

    /// Merges blocks and sorts them, canonicalizing the whole list.
    ///
    /// This is idempotent.
    fn cleanup(&mut self) {
        self.cleanup_inner();

        #[cfg(debug_assertions)]
        {
            // Check that `cleanup_inner()` is idempotent.
            let old = self.clone();
            self.cleanup_inner();
            assert_eq!(*self, old, "cleanup() is not idempotent");
        }
    }

    fn cleanup_inner(&mut self) {
        // Merge blocks until we reach a fixed point
        while self.merge_blocks() {}

        // Filter out empty blocks
        let mut len = self.len() as usize;
        let mut i = 0;
        while i < len {
            while self.blocks[i].is_empty() && i < len {
                len -= 1;
                self.blocks.swap(i, len);
            }
            i += 1;
        }
        self.meta.set_block_count(len as u8);

        // Sort blocks by `radix_sort_key`.
        self.blocks[..len].sort_by_key(|b| b.radix_sort_key());

        // Update inner ranks
        self.inner_ranks = Default::default();
        for (i, &b) in self.blocks[..len].iter().enumerate() {
            self.inner_ranks[b.inner_rank() as usize].set_from_0(i as u8);
        }

        debug_assert!(self.blocks().is_sorted_by_key(|b| b.radix_sort_key()));
    }

    /// Merges all blocks that can be merged.
    ///
    /// Returns whether any blocks were merged.
    fn merge_blocks(&mut self) -> bool {
        let mut any_merged = false;

        // It's important to iterate from largest to smallest rank, so that we
        // prioritize blocks where attitudes must match exactly (like
        // corner+edge) instead of blocks where many attitudes are
        // indistinguishable (like center+core).
        for body_rank in (0..4).rev() {
            let head_rank = body_rank + 1;
            let body_candidates = self.inner_ranks[body_rank as usize].clone();
            for body_index in body_candidates {
                let body = self[body_index];

                let head_candidates = self.inner_ranks[head_rank as usize].clone();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_block() {
        let init_block = Block::from_layer_bits(0x0FF);
        let state = BlockList::default().add_block_with_setup_moves(&[], init_block);
        assert_eq!(state.len(), 1);
        assert_eq!(state[0_u32], init_block);

        let ru = Twist::new(grips::R, elements::WZ);
        let ir = Twist::new(grips::I, elements::ZY);
        let state = BlockList::default().add_block_with_setup_moves(&[ru, ir], init_block);
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
