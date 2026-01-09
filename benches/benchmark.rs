use criterion::{Criterion, criterion_group, criterion_main};
use rand_pcg::Pcg64Mcg;
use robodoan::new::{Block, BlockLayerMask, BlockList};
use robodoan::*;

const NDIM: usize = 4;

fn exec_moves_on_blocks(init_state: BlockList, twists: &[Twist]) -> BlockList {
    twists.iter().fold(init_state, |state, &twist| {
        state.twist(twist).if_nonempty().unwrap()
    })
}

fn exec_moves_on_state(mut state: PuzzleState, twists: &[Twist]) -> PuzzleState {
    for &twist in twists {
        state.do_twist(twist);
    }
    state
}

fn do_and_undo(twists: Vec<Twist>) -> Vec<Twist> {
    twists.iter().chain(twists.iter().rev()).copied().collect()
}

fn criterion_benchmark(c: &mut Criterion) {
    let puzzle = &*RUBIKS_4D;
    let mut rng = Pcg64Mcg::new(0x_5c8d9681_fb970317_36ec6b11_2fa3c0bd);

    let move_count = 32;
    let gen_random_moves = move || puzzle.random_moves(&mut rng, move_count);

    let puzzle_with_2x2x2x2_block = BlockList::default()
        .add_block_with_setup_moves(&[], Block::from_layer_bits(0xFF0))
        .unwrap();

    for (init_state, name) in [(puzzle_with_2x2x2x2_block, "2x2x2x2 block")] {
        c.bench_function(&format!("do moves on {name}"), |b| {
            let gen_random_moves = gen_random_moves.clone();
            b.iter_batched(
                gen_random_moves.clone(),
                |input| exec_moves_on_blocks(init_state.clone(), &input),
                criterion::BatchSize::SmallInput,
            );
        });

        c.bench_function(&format!("do & undo moves on {name}"), |b| {
            let mut gen_random_moves = gen_random_moves.clone();
            b.iter_batched(
                || do_and_undo(gen_random_moves()),
                |input| exec_moves_on_blocks(init_state.clone(), &input),
                criterion::BatchSize::SmallInput,
            );
        });
    }

    for (init_state, name) in [(PuzzleState::default(), "full puzzle")] {
        c.bench_function(&format!("do moves on {name}"), |b| {
            let gen_random_moves = gen_random_moves.clone();
            b.iter_batched(
                gen_random_moves.clone(),
                |input| exec_moves_on_state(init_state.clone(), &input),
                criterion::BatchSize::SmallInput,
            );
        });

        c.bench_function(&format!("do & undo moves on {name}"), |b| {
            let mut gen_random_moves = gen_random_moves.clone();
            b.iter_batched(
                || do_and_undo(gen_random_moves()),
                |input| exec_moves_on_state(init_state.clone(), &input),
                criterion::BatchSize::SmallInput,
            );
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
