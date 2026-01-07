use std::error::Error;

use itertools::Itertools;
use rand::SeedableRng;
use robodoan::*;

fn main() -> Result<(), Box<dyn Error>> {
    let profile = Profile::Short;

    // rayon::ThreadPoolBuilder::new()
    //     .num_threads(1)
    //     .build_global()
    //     .unwrap();

    if let Some(filename) = std::env::args().nth(1) {
        let log_file_text = std::fs::read_to_string(&filename)?;
        let scramble: mc4d::Mc4dScramble = log_file_text.parse()?;
        println!("Loaded log file from {filename}");
        println!();
        // let (solve_twists, _elapsed_time) = search_4d(scramble.scramble());
        let solve_twists = robodoan::Solver::new(profile, scramble.scramble()).solve();
        println!();
        std::fs::write("out.log", scramble.to_string(false, solve_twists))?;
        return Ok(());
    }

    let mut results = vec![];
    let mut rng = rand::rngs::SmallRng::seed_from_u64(123);
    for i in 0..10 {
        let scramble = RUBIKS_4D.random_moves(&mut rng, 100);
        println!("\n\n---- STARTING SEARCH #{} ----\n", i + 1);
        println!("Scramble: {}", scramble.iter().join(" "));
        let t = std::time::Instant::now();
        let solution = robodoan::Solver::new(profile, scramble).solve();
        results.push((solution.len(), t.elapsed()));
    }
    println!("\n\n---- RESULTS ----\n");
    for (move_count, time) in results {
        println!("{move_count} ETM in {time:?}");
    }

    // // let scramble = RUBIKS_4D.random_moves(&mut rand::rng(), 100);
    // let scramble = parse_twists(
    //     "LF IB2 IDFR LF RBI ID ODFL BUO IBL BR2 OUF BLO IDFL OB FI LD RU2 DFLI
    // FUO IU2 OUBR BD IDFL OUB LDFO BDO FUL IR IUL OL2 LDBI FL BL IU LI2 ODFR OB2
    // OUF DFLI RI LO RF RB LD IDBL UBRO LDFI FULI FI2 OUF ODFL UFRI LU DL FU LDBO
    // DFRI OB LD UI FLI FO IF IFL DFLI LD FD DBO RUBI DO FD IDFL UBI LUBI BURO BDRI
    // BU BD2 RDBI UBL DB2 LO LDBO OL2 RF BDO ULI UFLO BR2 LB IL DFRI DFR FUI ULI FL
    // IU UBRI LO BURI", );
    // // let scramble = parse_twists(
    // //     "IDFL LF UBI IUB IL2 RUBO IUBR DO DLO IUBL UI2 ID FULI LUBO OUF DFRI
    // DL BULO FLI LUBI FR BLO UBO UBRO OUFL IU RFI BR2 RI ID IB2 OUBL RUBO DFLO
    // DFRO BO BDL UFRO OUFL ID BURO IBL OUL BD IBL DBLI FDLO BUO DBLO FLO LDF BRO
    // ID FLI IUF OL OL OBL ULO BUO FURI UL2 BR FDLO IUBL UBLO IBL BR2 IUBL BURO IU2
    // DF ODBL LDBO BR RUB LUBI IU2 FURI IUL OUL ODBR OL2 IB DL UB BO OUB FL2 DFRO
    // ODBR UFRI RI RUFI RDBO IUR UBO BUL RDFI LUFO", // );
    // println!("Scramble: {}", scramble.iter().join(" "));
    // println!();

    // robodoan::Solver::new(scramble).solve();

    Ok(())
}
