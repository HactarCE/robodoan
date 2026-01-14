# robodoan

Experimental blockbuilding-based search program for a [4D Rubik's cube](https://hypercubing.xyz/puzzles/3x3x3x3/)

Currently, this solver is only able to complete F2L. I plan on adding OLC and PLC solvers in the future.

## Performance

On an M2 Max Macbook Pro, here is how the F2L solver performed on 10 random scrambles of a 4-dimensional 3×3×3×3 Rubik's cube.

These are solutions to F2L, not the whole puzzle.

Move count is approximately STM except for very rare cases where two sequential moves use the same grip, in which case STM would be slightly lower than ETM.

There are two profiles: "fast" and "short."

### Fast profile

```
69 ETM in 8.97840925s
70 ETM in 7.331832625s
67 ETM in 7.840537417s
68 ETM in 7.571879958s
71 ETM in 9.166809125s
68 ETM in 7.999356833s
68 ETM in 7.219549042s
70 ETM in 8.134437291s
69 ETM in 8.066849666s
68 ETM in 8.181445167s

mean movecount: 68.8 ETM
mean time: 8.049 seconds
```

### Short profile

```
66 ETM in 17.397143792s
65 ETM in 17.069599208s
67 ETM in 16.124495667s
64 ETM in 16.10172325s
66 ETM in 19.68221025s
65 ETM in 18.024998208s
67 ETM in 17.205043041s
64 ETM in 15.573559166s
66 ETM in 18.377754833s
64 ETM in 13.521910042s

mean movecount: 65.4 ETM
mean time: 16.908 seconds
```

## Solving strategy

### F2L

#### Stages

F2L is solved in 6 stages of blockbuilding:

| Superstage         | Stage                               | Options | Additional pieces | Initial blocks | Target blocks |
| ------------------ | ----------------------------------- | :-----: | :---------------: | :------------: | :-----------: |
| Mid + left         | 2×2×2×2 (initial)                   |   16    |   2×2×2×2 = 16    |       16       |       1       |
|                    | 2×2×3×2 (extend Y)                  |    4    |    2×2×1×2 = 8    |       9        |       1       |
|                    | 2×3×3×2 (extend Z)                  |    3    |   2×1×3×2 = 12    |       13       |       1       |
| Right (mid + back) | 1×2×2×2 (intial)                    |    8    |    2×2×2×1 = 8    |       9        |       2       |
|                    | 1×2×3×2 (extend Y)                  |    2    |    2×2×1×1 = 4    |       6        |       2       |
| End                | 3×3×3×2 (F2L)                       |    1    |    1×3×1×2 = 2    |       5        |       1       |

- Note 1: The XYZW axes in this table roughly correspond to the XYZW axes of the [3-Block](https://hypercubing.xyz/methods/3x3x3x3/3block/) solving method.
- Note 2: Each stage has multiple options due to the symmetry of the puzzle.

These stages are selected to avoid dead ends while blockbuilding, where the blocks that have already been built get in the way of forming new ones.

#### Search algorithm

To find a solution to a stage, we use [iterative deepening depth-first search](https://en.wikipedia.org/wiki/Iterative_deepening_depth-first_search) up to a maximum depth (currently 4) to get to the target block count.

If we cannot reach the target block count, we increase the target block count by 1 and run the search again. If we are unable to find a solution that increases the block count at all, we increase the maximum depth and restart with the original target block count.

After we find a partial solution that solves some number of blocks, we re-run the search starting from the partial solution with the original target block count and maximum depth.

#### Pruning heuristics

The depth-first search rejects branches where the probability of forming enough blocks to meet the target is zero (using `Heuristic::Correct`) or very low (using `Heuristic::Fast`).

In practice, the faster heuristics lead to shorter solutions because even though some possibilities are discounted, we are able to search to a greater depth.

##### Correct heuristics

- **Combinatoric limit:** At most `2^remaining_moves * target_blocks` block pairings can be completed in the remaining moves.
- **Grip-theoretic limit:**
  - For each unordered pair of blocks `(b1, b2)`:
    - Let `m` be the grip along which they will differ when solved.
    - Let `m1 = b1.attitude * m` and `m2 = b2.attitude * m`.
    - Iff `m1 = m2`, then the blocks can be paired in 1 move. Assume that each subsequent move also creates one pairing; add `remaining_moves` to `max_blocks_solvable`.
    - Let the "free grips" be the intersection of the inactive grip sets of `b1` and `b2`.
    - Iff there is a twist of one of the free grips that takes `m1` to `m2` (equivalently: whose inverse takes `m2` to `m1`) then the blocks can be paired in 2 moves. Assume that each subsequent move also creates one pairing; add `remaining_moves - 1` to `max_blocks_solvable`.
    - Otherwise, the block can be paired in 3 moves. Assume that each subsequent move also creates one pairing; add `remaining_moves - 2` to `max_blocks_solvable`.
  - At most `max_blocks_solvable` block pairings can be completed in the remaining moves.

##### Fast heuristics

- **Combinatoric limit:** At most `2^n` block pairings can be completed in `n` moves.
- **Grip-theoretic limit:** Same as correct heuristic.

#### Representation

The puzzle state is represented using a stack-allocated list of blocks with a maximum length determined by a compile-time constant.

Each block is represented using 3 bytes:

- 2 bytes for a layer mask (3 bits × 4 axes)
- 1 byte for the attitude (ID from 0 to 191 inclusive)

Since each block is represented using 3 bytes, we're able to fit a puzzle state containing **21** blocks (63 bytes) + length (1 byte) in exactly 64 bytes.

### OLC + 2cPLC

I haven't started work on this yet.

### RKT PLC

I haven't started work on this yet.

## Representation

## History

This program is named after Charles Doan, the 3^4 FMC (fewest-moves challenge) world record holder at the time this program was developed. As of September 2024, Charles Doan holds both the computer-assisted and non-computer-assisted FMC records for the 3×3×3×3 puzzle, and in fact his submission for non-computer-assisted is even shorter than the computer-assisted solution.

I'm writing this program to give the computer-assisted category some much-needed love.

## Optimizations (to-do)

- [x] search multiple routes at once / meta search over block extensions
- [x] don't move the same grip twice within one search
- [x] indistinguishable attitudes
- [x] prune based on optimistic block formation heuristics
- [x] dynamically adjust goal based on depth
- [ ] NISS
- [ ] don't search duplicates (only consider pieces for current stage)
