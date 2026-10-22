use crate::moves::Move;
use crate::perft::perft;
use crate::pos::Position;
use crate::test_positions::{ExpectedPerftRes, CUSTOM_FENS, INVALID, STANDARD_FENS};
use rayon::prelude::*;
use std::io::{stdout, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[test]
fn simple_test() {
    let pos = Position::from_fen("    8/7r/8/K1k5/8/8/4p3/8 b - - 10 11").unwrap();
    let mov = Move::parse("e2e1q", &pos).unwrap();
    let pos = pos.make_move(mov);
    let res = perft(pos, 5);
    assert_eq!(res.nodes, 15010);
}

#[test]
fn kiwipete_test() {
    let pos = Position::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap();
    let res = perft(pos, 4);
    assert_eq!(res.nodes, 4_085_603);
    // kiwipete after white castles (cheaper to run than increasing the depth of kiwipete, and failed perft once in motors)
    let pos =
        Position::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R4RK1 b kq - 1 1").unwrap();
    let res = perft(pos, 4);
    assert_eq!(res.nodes, 4_119_629);
    // kiwipete after white plays a2a3
    let pos =
        Position::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/P1N2Q1p/1PPBBPPP/R3K2R b KQkq - 0 1")
            .unwrap();
    let res = perft(pos, 4);
    assert_eq!(res.nodes, 4_627_439);
}

#[test]
fn leonids_position_test() {
    let pos =
        Position::from_fen("q2k2q1/2nqn2b/1n1P1n1b/2rnr2Q/1NQ1QN1Q/3Q3B/2RQR2B/Q2K2Q1 w - - 0 1").unwrap();
    let res = perft(pos, 1);
    assert_eq!(res.nodes, 99);
    assert!(res.time.as_millis() <= 20);
    let res = perft(pos, 2);
    assert_eq!(res.nodes, 6271);
    let res = perft(pos, 3);
    assert_eq!(res.nodes, 568_299);
    if cfg!(not(debug_assertions)) {
        let res = perft(pos, 4);
        assert_eq!(res.nodes, 34_807_627);
    }
}

#[test]
fn castling_perft_test() {
    // this is not actually a reachable position, but we accept it, so we should handle it
    let fen = "r3k2r/ppp1pp1p/2nqb1Nn/3p4/4P3/2PP4/1PPPNBPP/2NRQK1R w KQkq -";
    let pos = Position::from_fen(fen).unwrap();
    let expected: &[u64] = &[
        33,
        1328,
        42079,
        1700714,
        #[cfg(not(debug_assertions))]
        53117779,
    ];
    for (depth, perft_num) in expected.iter().enumerate() {
        assert_eq!(perft(pos, depth + 1).nodes, *perft_num);
    }
}

#[test]
fn no_choice_test() {
    let fen = "5b1k/4p1p1/4P1P1/8/8/1p1p4/1P1P4/K1B5 w - - 0 1";
    let pos = Position::from_fen(fen).unwrap();
    let res = perft(pos, 500);
    assert_eq!(res.nodes, 1);
    let res = perft(pos, 500);
    assert_eq!(res.nodes, 1);
}


// ** The following tests are based on tests by kz04px: <https://github.com/kz04px/rawr/blob/master/tests/perft_extra.rs> **

#[test]
fn perft_enpassant() {
    let tests = [
        // EP
        ("8/8/8/8/1k1PpN1R/8/8/4K3 b - d3 0 1", vec![1, 9, 193]),
        ("8/8/8/8/1k1Ppn1R/8/8/4K3 b - d3 0 1", vec![1, 17, 220]),
        ("4k3/8/8/2PpP3/8/8/8/4K3 w - d6 0 1", vec![1, 9, 47, 376]),
        ("4k3/8/8/8/2pPp3/8/8/4K3 b - d3 0 1", vec![1, 9, 47, 376]),
        // EP - pinned diagonal
        ("8/5k2/8/2Pp4/8/1B6/8/6K1 w - d6 0 1", vec![1, 13, 101]),
        ("4k3/b7/8/2Pp4/8/4B3/8/6K1 w - d6 0 1", vec![1, 14, 123]),
        ("4k3/B7/8/2Pp4/8/8/8/6K1 w - d6 0 1", vec![1, 9, 50]),
        ("4k3/b7/8/2Pp4/8/8/8/6K1 w - d6 0 1", vec![1, 5, 45]),
        ("4k3/7b/8/4pP2/8/8/8/1K6 w - e6 0 1", vec![1, 5, 45]),
        ("6k1/8/8/8/2pP4/8/B7/3K4 b - d3 0 1", vec![1, 5, 45]),
        ("1k6/8/8/8/4Pp2/8/7B/4K3 b - e3 0 1", vec![1, 5, 45]),
        ("4k3/b7/8/1pP5/8/8/8/6K1 w - b6 0 1", vec![1, 6, 52]),
        ("4k3/7b/8/5Pp1/8/8/8/1K6 w - g6 0 1", vec![1, 6, 51]),
        ("6k1/8/8/8/1Pp5/8/B7/4K3 b - b3 0 1", vec![1, 6, 52]),
        ("1k6/8/8/8/5pP1/8/7B/4K3 b - g3 0 1", vec![1, 6, 51]),
        ("4k3/K7/8/1pP5/8/8/8/6b1 w - b6 0 1", vec![1, 6, 66]),
        ("4k3/7K/8/5Pp1/8/8/8/1b6 w - g6 0 1", vec![1, 6, 60]),
        ("6B1/8/8/8/1Pp5/8/k7/4K3 b - b3 0 1", vec![1, 6, 66]),
        ("1B6/8/8/8/5pP1/8/7k/4K3 b - g3 0 1", vec![1, 6, 60]),
        ("4k3/b7/8/2Pp4/3K4/8/8/8 w - d6 0 1", vec![1, 5, 44]),
        ("4k3/8/1b6/2Pp4/3K4/8/8/8 w - d6 0 1", vec![1, 6, 59]),
        ("4k3/8/b7/1Pp5/2K5/8/8/8 w - c6 0 1", vec![1, 6, 49]),
        ("4k3/8/7b/5pP1/5K2/8/8/8 w - f6 0 1", vec![1, 6, 49]),
        ("4k3/7b/8/4pP2/4K3/8/8/8 w - e6 0 1", vec![1, 5, 44]),
        ("4k3/8/6b1/4pP2/4K3/8/8/8 w - e6 0 1", vec![1, 6, 53]),
        ("4k3/8/3K4/1pP5/8/q7/8/8 w - b6 0 1", vec![1, 5, 114]),
        ("7k/4K3/8/1pP5/8/q7/8/8 w - b6 0 1", vec![1, 8, 171]),
        // EP - pinned horizontal
        ("4k3/8/8/K2pP2r/8/8/8/8 w - d6 0 1", vec![1, 6, 94]),
        ("4k3/8/8/K2pP2q/8/8/8/8 w - d6 0 1", vec![1, 6, 130]),
        ("4k3/8/8/r1Pp3K/8/8/8/8 w - d6 0 1", vec![1, 6, 88]),
        ("4k3/8/8/r2pP2K/8/8/8/8 w - d6 0 1", vec![1, 6, 87]),
        ("4k3/8/8/q2pP2K/8/8/8/8 w - d6 0 1", vec![1, 6, 129]),
        ("8/8/8/8/1k1Pp2R/8/8/4K3 b - d3 0 1", vec![1, 8, 125]),
        ("8/8/8/8/1R1Pp2k/8/8/4K3 b - d3 0 1", vec![1, 6, 87]),
        ("8/8/8/8/1r1Pp2k/8/8/4K3 b - d3 0 1", vec![1, 17, 88]),
        ("8/8/8/8/1R1PpP1k/8/8/4K3 b - d3 0 1", vec![1, 6, 93]),
        ("8/8/8/8/1r1PpP1k/8/8/4K3 b - d3 0 1", vec![1, 16, 97]),
        ("8/8/8/8/1RpPp2k/8/8/4K3 b - d3 0 1", vec![1, 9, 129]),
        ("8/8/8/8/1rpPp2k/8/8/4K3 b - d3 0 1", vec![1, 17, 88]),
        ("8/8/8/1k2pPR1/5K2/8/8/8 w - e6 0 1", vec![1, 7, 58]),
        // EP - pinned vertical
        ("k7/8/4r3/3pP3/8/8/8/4K3 w - d6 0 1", vec![1, 5, 70]),
        ("k3K3/8/8/3pP3/8/8/8/4r3 w - d6 0 1", vec![1, 6, 91]),
        ("br1knbqr/pp2p1pp/1n6/2p5/3pPpP1/2PQ3B/PP1P1P1P/BRNKN2R b HBhb e3 0 11", vec![1, 30, 866]),
        ("5r2/1p3k2/pBp1p1b1/3rq1b1/PPR1pPpp/4Q1P1/4P1BP/5RK1 b - f3 0 28", vec![1, 48, 2066]),
        // EP - in check
        ("4k3/8/8/4pP2/3K4/8/8/8 w - e6 0 1", vec![1, 9, 49]),
        ("8/8/8/4k3/5Pp1/8/8/3K4 b - f3 0 1", vec![1, 9, 50]),
        ("5r2/8/8/1k2pPR1/5K2/8/8/8 w - - 0 1", vec![1, 6, 112]),
    ];

    for (fen, results) in tests {
        let pos = Position::from_fen(fen).unwrap();
        for (idx, &expected) in results.iter().enumerate() {
            let result = perft(pos, idx);
            assert_eq!(result.nodes, expected, "depth {idx}: {fen}");
        }
    }
}

#[test]
fn perft_double_checked() {
    let tests = [
        ("4k3/8/4r3/8/8/8/3p4/4K3 w - - 0 1", [1, 4, 80, 320]),
        ("4k3/8/4q3/8/8/8/3b4/4K3 w - - 0 1", [1, 4, 143, 496]),
    ];

    for (fen, results) in tests {
        let pos = Position::from_fen(fen).unwrap();
        for (idx, &expected) in results.iter().enumerate() {
            let result = perft(pos, idx);
            assert_eq!(result.nodes, expected, "depth {idx}: {fen}");
        }
    }
}

#[test]
fn perft_pins() {
    let tests = [
        ("4k3/8/8/8/1b5b/8/3Q4/4K3 w - - 0 1", [1, 3, 54, 1256, 20328]),
        ("4k3/8/8/8/1b5b/8/3R4/4K3 w - - 0 1", [1, 3, 54, 836, 14835]),
        ("4k3/8/8/8/1b5b/2Q5/5P2/4K3 w - - 0 1", [1, 6, 98, 2274, 34581]),
        ("4k3/8/8/8/1b5b/2R5/5P2/4K3 w - - 0 1", [1, 4, 72, 1300, 23118]),
        ("4k3/8/8/8/1b2r3/8/3Q4/4K3 w - - 0 1", [1, 3, 66, 1390, 29093]),
        ("4k3/8/8/8/1b2r3/8/3QP3/4K3 w - - 0 1", [1, 6, 119, 2074, 40736]),
        // Additional position with some rather tricky cases
        ("1k6/8/1q3b2/2R1P3/r1RK1B1r/8/8/8 w - - 0 1", [1, 8, 276, 4995, 180603]),
        // castling out of a pin
        ("3k4/4r3/8/8/8/4B3/8/R3K3 w Q - 0 1", [1, 16, 214, 4814, 72521]),
        // even more weird pins
        ("3q3r/7R/3r3Q/B2R2rk/1b3B1p/2B1Pp2/1qNKBR2/4B3 b - - 21 42", [1, 2, 84, 3342, 130469]),
    ];

    for (fen, results) in tests {
        let pos = Position::from_fen(fen).unwrap();
        for (idx, &expected) in results.iter().enumerate() {
            let result = perft(pos, idx);
            assert_eq!(result.nodes, expected, "depth {idx}: {fen}");
        }
    }
}

#[test]
fn perft_dfrc() {
    let tests = [
        ("2r1kr2/8/8/8/8/8/8/1R2K1R1 w GBfc - 0 1", [1, 22, 501, 11459]),
        ("rkr5/8/8/8/8/8/8/5RKR w HFca - 0 1", [1, 22, 442, 10217]),
        ("2r3kr/8/8/8/8/8/8/2KRR3 w h - 3 2", [1, 3, 72, 1371]),
        ("5rkr/8/8/8/8/8/8/RKR5 w CAhf - 0 1", [1, 22, 442, 10206]),
        ("3rkr2/8/8/8/8/8/8/R3K2R w HAfd - 0 1", [1, 20, 452, 9873]),
        ("4k3/8/8/8/8/8/8/4KR2 w F - 0 1", [1, 14, 47, 781]),
        ("4kr2/8/8/8/8/8/8/4K3 w f - 0 1", [1, 3, 42, 246]),
        ("4k3/8/8/8/8/8/8/2R1K3 w C - 0 1", [1, 16, 71, 1277]),
        ("2r1k3/8/8/8/8/8/8/4K3 w c - 0 1", [1, 5, 80, 448]),
    ];
    for (fen, results) in tests {
        let pos = Position::from_fen(fen).unwrap();
        for (idx, &expected) in results.iter().enumerate() {
            let result = perft(pos, idx);
            assert_eq!(result.nodes, expected, "depth {idx}: {fen}");
        }
    }
}

#[test]
/// Only meant to make sure DFRC works assuming Chess960 and normal chess movegen already works.
fn dfrc_perft_test() {
    #[cfg(debug_assertions)]
    const FENS: [&str; 2] = [
        "r1q1k1rn/1p1ppp1p/1npb2b1/p1N3p1/8/1BP4P/PP1PPPP1/1RQ1KRBN w BFag - 0 9 ;D1 32 ;D2 1093 ;D3 34210 ;D4 1187103", // ;D5 37188628",// ;D6 1308319545",
        "rk3r2/8/8/5r2/6R1/8/8/R3K1R1 w AGaf - 0 1 ;D1 31 ;D2 841 ;D3 23877 ;D4 711547", // ;D5 20894205",// ;D6 644033568"
    ];
    #[cfg(not(debug_assertions))]
    const FENS: [&str; 2] = [
        "r1q1k1rn/1p1ppp1p/1npb2b1/p1N3p1/8/1BP4P/PP1PPPP1/1RQ1KRBN w BFag - 0 9 ;D1 32 ;D2 1093 ;D3 34210 ;D4 1187103 ;D5 37188628", // ;D6 1308319545",
        "rk3r2/8/8/5r2/6R1/8/8/R3K1R1 w AGaf - 0 1 ;D1 31 ;D2 841 ;D3 23877 ;D4 711547 ;D5 20894205", // ;D6 644033568"
    ];

    perft_test(&FENS);
}


#[test]
#[ignore]
fn standard_perft_test() {
    perft_test(STANDARD_FENS);
}

// #[test]
// #[ignore]
// fn chess960_perft_test() {
//     perft_test(&CHESS_960_FENS);
// }

#[test]
#[ignore]
fn custom_perft_test() {
    perft_test(CUSTOM_FENS);
}

/// Parallelizes the perft testcases so that this takes less time, but the chess960 suite still takes a very long time.
fn perft_test(fens: &'static [&'static str]) {
    let start_time = Instant::now();
    let solved_tests = AtomicU64::new(0);
    let solved_tests = &solved_tests;
    let f = |i: usize, testcase: &'static str| {
        let expected = ExpectedPerftRes::new(testcase);
        let pos = Position::from_fen(expected.fen).unwrap();
        println!("Running test on fen {0}, board\n{pos}", expected.fen);
        stdout().flush().unwrap();
        let fen = pos.to_string();
        let pos2 = Position::from_fen(&fen).unwrap();
        assert_eq!(pos, pos2, "Boards differ: '{pos }' vs '{pos2}', fen was '{}'\n{pos:?}\n{pos2:?}", expected.fen);

        for (depth, expected_count) in expected.res.iter().enumerate().filter(|&(_, x)| *x != INVALID) {
            let res = perft(pos, depth);
            assert_eq!(res.depth, depth);
            assert_eq!(res.nodes, *expected_count, "{depth} {pos}");
            println!(
                "Perft depth {0} of pos {i} took {1} ms, total time so far: {2}ms",
                res.depth,
                res.time.as_millis(),
                start_time.elapsed().as_millis(),
            );
        }
        let solved = solved_tests.fetch_add(1, Ordering::Relaxed);
        println!("Finished {0} / {1} positions", solved + 1, fens.len());
    };
    fens.par_iter().enumerate().for_each(|(i, fen)| f(i, fen));
}
