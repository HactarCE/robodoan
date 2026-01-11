use std::error::Error;

use itertools::Itertools;
use rand::SeedableRng;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use robodoan::{sim::ALL_TWISTS, *};

fn main() -> Result<(), Box<dyn Error>> {
    let params = BlockBuildingSearchParams::default();

    if let Some(filename) = std::env::args().nth(1) {
        let log_file_text = std::fs::read_to_string(&filename)?;
        let scramble: mc4d::Mc4dScramble = log_file_text.parse()?;
        println!("Loaded log file from {filename}");
        println!();
        // let (solve_twists, _elapsed_time) = search_4d(scramble.scramble());
        let solve_twists = robodoan::Solver::new(params, scramble.scramble()).solve();
        println!();
        std::fs::write("out.log", scramble.to_string(false, solve_twists))?;
        return Ok(());
    }

    let mut results = vec![];
    let mut rng = rand::rngs::SmallRng::seed_from_u64(123);
    for i in 0..10 {
        let scramble = sim::random_twists(&mut rng, 100);
        println!("\n\n---- STARTING SEARCH #{} ----\n", i + 1);
        println!("Scramble: {}", scramble.iter().join(" "));
        let t = std::time::Instant::now();
        let solution = robodoan::Solver::new(params, scramble).solve();
        results.push((solution.len(), t.elapsed()));
    }
    println!("\n\n---- RESULTS ----\n");
    for (move_count, time) in results {
        println!("{move_count} ETM in {time:?}");
    }

    return Ok(());

    println!();
    println!();
    let mut examples = gpu_test::take_all_examples();
    examples.truncate(1024 * 64);

    let states = examples.iter().map(|(b, _)| b.clone()).collect_vec();
    let twists = &*ALL_TWISTS;

    println!(
        "Testing {} states * {} twists on GPU",
        states.len(),
        twists.len(),
    );

    println!("Executing on CPU ...");
    let t = std::time::Instant::now();
    let expected: Vec<_> = states
        .par_iter()
        .flat_map_iter(|block_list| twists.iter().map(|&twist| block_list.twist(twist).len()))
        .collect();
    println!("Done in {:?}!", t.elapsed());

    let mut gpu = robodoan::gpu::Gpu::new();
    println!("Executing on GPU ...");
    let t = std::time::Instant::now();
    let actual = gpu.test_do_twist(&states, &twists);

    // let mut times = vec![];
    // for _ in 0..20 {
    //     let t1 = std::time::Instant::now();
    //     gpu.test_do_twist(&states, &twists);
    //     times.push(t1.elapsed());
    // }
    // let len = times.len() as f64;
    // let ms = times.iter().map(|d| d.as_secs_f64() * 1000.0);
    // let avg = ms.clone().sum::<f64>() / len;
    // let stddev = (ms.map(|s| (s - avg) * (s - avg)).sum::<f64>() / len).sqrt();
    // println!("average: {avg} ms, stddev: {stddev} ms");

    println!("Done in {:?}! Checking results ...", t.elapsed());

    assert_eq!(expected.len(), actual.len());

    for (i, (exp, act)) in itertools::izip!(&expected, actual).enumerate() {
        if *exp != act {
            println!("failed on index {i}");
            dbg!(states[i / 184]);
            dbg!(twists[i % 184]);
            dbg!(i);
            dbg!(i / 184);
            dbg!(i % 184);
            pretty_assertions::assert_eq!(*exp, act);
        }
    }
    println!("SUCCESS! They all matched!");

    Ok(())
}

/// Sets the thread count for the global thread pool.
pub fn set_thread_count(thread_count: usize) {
    rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build_global()
        .unwrap();
}
