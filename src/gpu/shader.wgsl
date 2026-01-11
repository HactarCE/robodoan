override MUL_ELEM_ELEM: u32;
override INVERSE_ELEM: u32;
override MUL_ELEM_GRIP: u32;
override MUL_ELEM_AXIS: u32;

// @group(0) @binding(0) var lut: texture_2d<u32>;
@group(0) @binding(0) var<storage, read> lut: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@group(0) @binding(2) var<storage, read> block_lists: array<BlockList>;
@group(0) @binding(3) var<storage, read> twists: array<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;

    let block_list_count = arrayLength(&block_lists);
    let twist_count = arrayLength(&twists) * 2;
    if index >= block_list_count * twist_count {
        return;
    }

    let block_list_index = index / twist_count;
    let twist_index = index % twist_count;

    var block_list = block_lists[block_list_index];
    var twist = Twist((twists[twist_index/2] >> ((twist_index%2) * 16)) & 0xFFFF);

    block_list_twist(&block_list, twist);

    output[index] = block_list_len(&block_list);
}



/**************************
 ****    BLOCK LIST    ****
 **************************/

const MAX_BLOCK_COUNT: u32 = 26;

struct BlockList {
    blocks: array<Block, MAX_BLOCK_COUNT>,
    inner_ranks: array<u32, 5>,
    metadata: BlockListMeta,
}

const EMPTY_BLOCK_LIST: BlockList = BlockList(
    array<Block, MAX_BLOCK_COUNT>(),
    array<u32, 5>(),
    BlockListMeta(0),
);

fn block_list_len(list: ptr<function, BlockList>) -> u32 {
    return meta_block_count(list.metadata);
}

fn block_list_is_empty(list: ptr<function, BlockList>) -> bool {
    return block_list_len(list) == 0;
}

fn block_list_twist_count(list: ptr<function, BlockList>) -> u32 {
    return meta_twist_count(list.metadata);
}

/// Returns `true` in case of overflow
fn block_list_push(list: ptr<function, BlockList>, b: Block) -> bool {
    let i = block_list_len(list);

    // Update len
    meta_increment_block_count(&list.metadata);
    if block_list_len(list) > MAX_BLOCK_COUNT {
        return true;
    }

    // Update ranks
    list.inner_ranks[block_inner_rank(b)] |= 1u << i;

    // Update blocks
    list.blocks[i] = b;

    return false;
}

fn block_list_set_block_with_same_rank(list: ptr<function, BlockList>, index: u32, new_block: Block) {
    list.blocks[index] = new_block;
}

fn block_list_remove_block(list: ptr<function, BlockList>, index: u32) {
    let old_block = list.blocks[index];

    // Update ranks
    list.inner_ranks[block_inner_rank(old_block)] &= ~(1u << index);

    // Update blocks
    list.blocks[index] = EMPTY_BLOCK;
}

fn block_list_twist(list: ptr<function, BlockList>, twist: Twist) {
    meta_count_twist_on_grip(&list.metadata, get_twist_grip(twist));

    let len = block_list_len(list);
    for (var i = 0u; i < len; i++) {
        let b = list.blocks[i];
        let block_array = split_block(b, get_twist_grip(twist));
        var active_block = block_array[0];
        let inactive_block = block_array[1];
        if block_is_empty(active_block) {
            continue; // no change
        } else {
            active_block = mul_elem_block(get_twist_transform(twist), active_block);
            if block_is_empty(inactive_block) {
                block_list_set_block_with_same_rank(list, i, active_block); // all active
            } else {
                block_list_set_block_with_same_rank(list, i, inactive_block); // inactive has same rank
                if block_list_push(list, active_block) {
                    *list = EMPTY_BLOCK_LIST; // indicate error
                }
            }
        }
    }

    block_list_cleanup(list);
}

fn block_list_cleanup(list: ptr<function, BlockList>) {
    // Merge blocks until we reach a fixed point
    while block_list_merge_blocks(list) {}

    // Filter out empty blocks and update length
    var len = block_list_len(list);
    for (var i = 0u; i < len; i++) {
        while block_is_empty(list.blocks[i]) && i < len {
            len -= 1;
            list.blocks[i] = list.blocks[len];
            list.blocks[len] = EMPTY_BLOCK;
        }
    }
    meta_set_block_count(&list.metadata, len);

    // Sort blocks by bit pattern using insertion sort
    for (var i = 1u; i < len; i++) {
        let b = list.blocks[i];
        var j = i;
        while j > 0 && list.blocks[j-1].bits > b.bits {
            list.blocks[j] = list.blocks[j-1];
            j -= 1;
        }
        list.blocks[j] = b;
    }

    // Update inner ranks
    list.inner_ranks = array<u32, 5>();
    for (var i = 0u; i < len; i++) {
        list.inner_ranks[block_inner_rank(list.blocks[i])] |= 1u << i;
    }
}

/// Returns `true` if any blocks were merged
fn block_list_merge_blocks(list: ptr<function, BlockList>) -> bool {
    var any_merged = false;

    // body_rank MUST be signed, because of the `>= 0` condition
    for (var body_rank = 3; body_rank >= 0; body_rank--) {
        let head_rank = body_rank + 1;
        var body_candidates = list.inner_ranks[body_rank];
        loop {
            let body_index = pop_bit(&body_candidates);
            if body_index >= 32 { break; }

            let body = list.blocks[body_index];

            var head_candidates = list.inner_ranks[head_rank];
            loop {
                let head_index = pop_bit(&head_candidates);
                if head_index >= 32 { break; }

                let head = list.blocks[head_index];

                let merged = merge_blocks(body, head);

                if !block_is_empty(merged) {
                    any_merged = true;
                    // Remove head
                    block_list_remove_block(list, head_index);
                    // Replace body (inner rank stays the same)
                    block_list_set_block_with_same_rank(list, body_index, merged);

                    break; // break out of inner loop
                }
            }
        }
    }

    return any_merged;
}



/***********************************
 ****    BLOCK LIST METADATA    ****
 ***********************************/

struct BlockListMeta { bits: u32 }

const OFS_BLOCK_COUNT: u32 = 0;
const OFS_TWISTED_GRIPS: u32 = 8;
const OFS_TWIST_COUNT: u32 = 16;

const MASK_BLOCK_COUNT: u32 = 0xFF << OFS_BLOCK_COUNT;
const MASK_TWISTED_GRIPS: u32 = 0xFF << OFS_TWISTED_GRIPS;
const MASK_TWIST_COUNT: u32 = 0xFFFF << OFS_TWIST_COUNT;

fn meta_block_count(m: BlockListMeta) -> u32 {
    return (m.bits >> OFS_BLOCK_COUNT) & 0xFF;
}

fn meta_twisted_grips(m: BlockListMeta) -> GripSet {
    return GripSet((m.bits >> OFS_TWISTED_GRIPS) & 0xFF);
}

fn meta_twist_count(m: BlockListMeta) -> u32 {
    return (m.bits >> OFS_TWIST_COUNT) & 0xFFFF;
}

fn meta_set_block_count(m: ptr<function, BlockListMeta>, new_block_count: u32) {
    m.bits &= ~MASK_BLOCK_COUNT;
    m.bits |= new_block_count << OFS_BLOCK_COUNT;
}

fn meta_increment_block_count(m: ptr<function, BlockListMeta>) {
    m.bits += 1 << OFS_BLOCK_COUNT;
}

fn meta_count_twist_on_grip(m: ptr<function, BlockListMeta>, grip: Grip) {
    let count_twist = !grip_set_contains(meta_twisted_grips(*m), grip);
    m.bits += u32(count_twist) << OFS_TWIST_COUNT;
    m.bits &= ~MASK_TWISTED_GRIPS | (grip_set_from_axis(grip_axis(grip)).bits << OFS_TWISTED_GRIPS);
    m.bits |= grip_set_from_grip(grip).bits << OFS_TWISTED_GRIPS;
}



/**********************
 ****    BLOCKS    ****
 **********************/

struct Block { bits: u32 }

const EMPTY_BLOCK: Block = Block(0);

const OFS_LAYERS: u32 = 0;
const OFS_ATT: u32 = 16;

const MASK_LAYERS: u32 = 0xFFF << OFS_LAYERS;
const MASK_ATT: u32 = 0xFF << OFS_ATT;

fn block_is_empty(b: Block) -> bool {
    return b.bits == 0;
}

fn block_from_layer_bits(layer_bits: u32) -> Block {
    let is_empty = (layer_bits | (layer_bits >> 4) | (layer_bits >> 8)) == 0;
    return Block(select(layer_bits, 0, is_empty));
}

fn block_from_layer_bits_nonempty(layer_bits: u32) -> Block {
    return Block(layer_bits);
}

fn block_layer_bits(b: Block) -> u32 {
    return (b.bits >> OFS_LAYERS) & 0xFFF;
}
fn block_layer_bits_x2(blocks: vec2<u32>) -> vec2<u32> {
    return (blocks >> vec2(OFS_LAYERS)) & vec2(0xFFF);
}

fn block_inner_rank(b: Block) -> u32 {
    let layer_bits = block_layer_bits(b);
    return 4 - countOneBits(layer_bits & 0x0F0);
}

fn block_outer_rank(b: Block) -> u32 {
    let layer_bits = block_layer_bits(b);
    return countOneBits((layer_bits | (layer_bits >> 8)) & 0xF);
}

fn block_attitude(b: Block) -> Elem {
    return Elem((b.bits >> OFS_ATT) & 0xFF);
}

fn block_inv_attitude(b: Block) -> Elem {
    return inv_elem(block_attitude(b));
}

fn block_with_attitude(b: Block, attitude: Elem) -> Block {
    var out = b;
    out.bits &= ~MASK_ATT;
    out.bits |= attitude.id << OFS_ATT;
    return out;
}

fn block_copy_attitude(tgt: Block, src: Block) -> Block {
    return Block((tgt.bits & ~MASK_ATT) | (src.bits & MASK_ATT));
}

fn block_at_solved(b: Block) -> Block {
    return block_with_attitude(b, IDENT);
}

fn split_block(b: Block, grip: Grip) -> array<Block, 2> {
    let g = mul_elem_grip(block_inv_attitude(b), grip);
    let layer_bits = block_layer_bits(b);
    let axis_mask = layer_bits_for_axis(grip_axis(g));
    let grip_mask = layer_bit_for_grip(g);

    var active_block: Block;
    if (layer_bits & grip_mask) == 0 {
        active_block = EMPTY_BLOCK;
    } else {
        active_block = block_copy_attitude(
            block_from_layer_bits_nonempty(layer_bits & (grip_mask | ~axis_mask)),
            b,
        );
    }

    var inactive_block: Block;
    if (layer_bits & axis_mask & ~grip_mask) == 0 {
        inactive_block = EMPTY_BLOCK;
    } else {
        inactive_block = block_copy_attitude(
            block_from_layer_bits_nonempty(layer_bits & ~grip_mask),
            b,
        );
    };

    return array(active_block, inactive_block);
}

fn block_active_or_blocked_axes(b: Block) -> AxisSet {
    let layer_bits = block_layer_bits(b);
    return AxisSet((layer_bits | (layer_bits >> 8)) & 0xF);
}

fn merge_blocks(body: Block, head: Block) -> Block {
    let body_layer_bits = block_layer_bits(body);
    let head_layer_bits = block_layer_bits(head);

    // Check layers
    let diff = body_layer_bits ^ head_layer_bits;
    let merge_axis = countTrailingZeros(diff) & 0x3;
    let differ_along_multiple_axes = (diff & ~(0x111u << merge_axis)) != 0;
    let disconnected = (diff & (0x010u << merge_axis)) == 0;
    if differ_along_multiple_axes | disconnected {
        return EMPTY_BLOCK;
    }

    // 4x Elem packed into a u32
    const XY_STABILIZER: u32 = 0x00438085;
    const YZ_STABILIZER: u32 = 0x00030c24;
    const ZW_STABILIZER: u32 = 0x0001040d;
    const XZ_STABILIZER: u32 = 0x002c566e;
    const YW_STABILIZER: u32 = 0x00020819;
    const XW_STABILIZER: u32 = 0x00285461;

    const PLANE_STABILIZERS: array<u32, 16> = array(
        0, 0, 0,       // 0, 1, 2
        XY_STABILIZER, // 3 = 0b0011
        0,             // 4
        XZ_STABILIZER, // 5 = 0b0101
        YZ_STABILIZER, // 6 = 0b0110
        0, 0,          // 7, 8
        XW_STABILIZER, // 9 = 0b1001
        YW_STABILIZER, // 10 = 0b1010
        0,             // 11
        ZW_STABILIZER, // 12 = 0b1100
        0, 0, 0,       // 13, 14, 15
    );


    // Check attitudes
    let body_attitude = block_attitude(body);
    let head_attitude = block_attitude(head);
    let body_active_or_blocked_axes = block_active_or_blocked_axes(body);

    let center_matches = true;

    let ridge_matches = elem_fixes_grip(head_attitude, positive_grip_on_axis(axis_set_unwrap_one(body_active_or_blocked_axes)));

    let attitude_delta = mul_elem_elem(block_inv_attitude(body), head_attitude);
    let ridge_indistinguishable_subgroup = unpack4xU8(PLANE_STABILIZERS[body_active_or_blocked_axes.bits]);
    let edge_matches = any(ridge_indistinguishable_subgroup == vec4(attitude_delta.id));

    let corner_matches = body_attitude.id == head_attitude.id;

    let attitude_matches = vec4(center_matches, ridge_matches, edge_matches, corner_matches)[min(3, block_outer_rank(body))];

    if !attitude_matches {
        return EMPTY_BLOCK;
    }

    return block_copy_attitude(
        block_from_layer_bits_nonempty(body_layer_bits | head_layer_bits),
        head,
    );
}

fn mul_elem_block(elem: Elem, b: Block) -> Block {
    return block_with_attitude(b, mul_elem_elem(elem, block_attitude(b)));
}

fn layer_bits_for_axis(axis: Axis) -> u32 {
    return 0x111u << axis.id;
}

fn layer_bit_for_grip(grip: Grip) -> u32 {
    return 1u << (grip_axis(grip).id + (grip_sign(grip) << 3));
}



/********************************
 ****    SEMANTIC BITSETS    ****
 ********************************/

struct GripSet { bits: u32 }

struct AxisSet { bits: u32 }

fn grip_set_contains(grip_set: GripSet, g: Grip) -> bool {
    return (grip_set.bits & (1u << g.id)) != 0;
}

fn axis_set_contains(axis_set: AxisSet, ax: Axis) -> bool {
    return (axis_set.bits & (1u << ax.id)) != 0;
}

fn grip_set_from_grip(grip: Grip) -> GripSet {
    return GripSet(1u << grip.id);
}

fn grip_set_from_axis(axis: Axis) -> GripSet {
    return GripSet(0x3u << (axis.id << 1));
}

fn axis_set_unwrap_one(axis_set: AxisSet) -> Axis {
    return Axis(countTrailingZeros(axis_set.bits));
}



/************************
 ****    BITSET96    ****
 ************************/

struct BitSet96 { contents: vec3<u32> }

const EMPTY_BITSET96: BitSet96 = BitSet96(vec3());

fn bitset96_set(bitset: ptr<function, BitSet96>, index: u32) {
    bitset.contents[index / 32] |= 1u << (index % 32);
}

fn bitset96_count_ones(bitset: BitSet96) -> u32 {
    let ones = countOneBits(bitset.contents);
    return ones.x + ones.y + ones.z;
}

fn bitset96_bits_before(bitset: BitSet96, index: u32) -> u32 {
    let i = max(vec3(index), vec3(0, 32, 64)) - vec3(0, 32, 64);
    let mask_lowest_n_bits = select(vec3(0u), vec3(1u) << i, i < vec3(32)) - 1;
    let ones = countOneBits(bitset.contents & mask_lowest_n_bits);
    return ones.x + ones.y + ones.z;
}



/*********************
 ****    TWIST    ****
 *********************/

struct Twist { bits: u32 }

fn construct_twist(grip: Grip, transform: Elem) -> Twist {
    return Twist(grip.id | (transform.id << 8));
}

fn get_twist_grip(twist: Twist) -> Grip {
    return Grip(twist.bits & 0xFF);
}

fn get_twist_transform(twist: Twist) -> Elem {
    return Elem(twist.bits >> 8);
}



/**************************
 ****    PRIMITIVES    ****
 **************************/

struct Elem { id: u32 }

const IDENT: Elem = Elem(0);

struct Grip { id: u32 }

struct Axis { id: u32 }

fn get_lut(e: Elem, row: u32) -> u32 {
    let index = e.id + row * 192;
    return unpack4xU8(lut[index / 4])[index % 4];
}

fn mul_elem_elem(lhs: Elem, rhs: Elem) -> Elem {
    return Elem(get_lut(lhs, MUL_ELEM_ELEM + rhs.id));
}

fn inv_elem(e: Elem) -> Elem {
    return Elem(get_lut(e, INVERSE_ELEM));
}

fn mul_elem_grip(e: Elem, g: Grip) -> Grip {
    return Grip(get_lut(e, MUL_ELEM_GRIP + g.id));
}

fn mul_elem_axis(e: Elem, ax: Axis) -> Axis {
    return Axis(get_lut(e, MUL_ELEM_AXIS + ax.id));
}

fn grip_axis(g: Grip) -> Axis {
    return Axis(g.id >> 1);
}

fn grip_sign(g: Grip) -> u32 {
    return g.id & 1;
}

fn elem_fixes_grip(e: Elem, g: Grip) -> bool {
    return mul_elem_grip(e, g).id == g.id;
}

fn positive_grip_on_axis(ax: Axis) -> Grip {
    return Grip(ax.id << 1);
}

/// Returns 32 if there are no more bits
fn pop_bit(bitset: ptr<function, u32>) -> u32 {
    let index = countTrailingZeros(*bitset);
    // if `*bitset == 0` then this assignment is a no-op
    *bitset &= ~(1u << index);
    return index;
}
