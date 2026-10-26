// gives a warning for being unused when pext is not enabled
// (e.g. with `RUSTFLAGS="-C target-cpu=..."`)
#![feature(alloc_slice_into_array)]
#![feature(portable_simd)]

use crate::core::Res;
use crate::moves::Move;
use crate::perft::{PerftRes, perft_impl, splitperft};
use crate::pos::Position;
use crate::test_positions::{ExpectedPerftRes, STANDARD_FENS};
use std::cmp::min;
use std::env::args;
use std::io;
use std::ops::ControlFlow;
use std::ops::ControlFlow::{Break, Continue};
use std::time::Instant;

mod attacks;
mod core;
mod movegen;
mod moves;
mod perft;
#[cfg(test)]
mod perft_tests;
mod pos;
mod test_positions;

fn parse_pos(mut input: &str) -> Res<Position> {
    input = input.strip_prefix("position ").unwrap_or(input);
    input = input.strip_prefix("fen ").unwrap_or(input);
    let Some((fen, moves)) = input.split_once(" moves ") else {
        return Position::from_fen(&input);
    };
    let mut pos = Position::from_fen(&fen)?;
    for m in moves.split_ascii_whitespace() {
        let m = Move::parse(m, &pos)?;
        if !pos.is_probably_legal(m) {
            return Err(format!("Move '{m}' is not legal in {pos}"));
        }
        pos = pos.make_move(m);
    }
    Ok(pos)
}

fn handle_input(mut line: &str, pos: &mut Position) -> Res<ControlFlow<()>> {
    line = line.trim();
    if line == "x" || line == "quit" {
        return Ok(Break(()));
    } else if line == "uci" {
        println!("id name chess-perft");
        println!("id author ToTheAnd");
        println!("uciok");
        return Ok(Continue(()));
    } else if line.starts_with("position ") {
        *pos = parse_pos(line)?;
        return Ok(Continue(()));
    } else if line == "show" || line == "debug" || line == "d" {
        println!("{pos}");
        return Ok(Continue(()));
    }

    line = line.strip_prefix("go ").unwrap_or(line);
    line = line.strip_prefix("perft ").unwrap_or(line);
    line = line.strip_prefix("depth ").unwrap_or(line);

    let Some((depth, fen)) = line.split_once(" ") else {
        let depth = line
            .parse::<usize>()
            .map_err(|_| format!("Failed to parse depth '{line}'"))?;
        println!("{}", splitperft(*pos, depth));
        return Ok(Continue(()));
    };
    let depth = depth
        .parse::<usize>()
        .map_err(|_| format!("Failed to parse depth '{depth}'"))?;
    *pos = parse_pos(fen)?;
    println!("{}", splitperft(*pos, depth));
    Ok(Continue(()))
}

fn run(pos: &mut Position, lines: impl Iterator<Item = String>) -> Option<()> {
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if line == "--benchmark" || line == "-b" {
            benchmark();
            return None;
        }
        match handle_input(&line, pos) {
            Err(err) => {
                println!("{err}");
                continue;
            }
            Ok(Break(())) => return None,
            Ok(Continue(())) => {}
        };
    }
    Some(())
}

fn main() {
    attacks::force_init();
    let mut pos = Position::new();
    let Some(()) = run(&mut pos, args().skip(1)) else {
        return;
    };
    println!("Enter a depth and FEN, or 'x' to quit");
    _ = run(&mut pos, io::stdin().lines().map(|l| l.unwrap()));
}

fn benchmark() {
    const MAX_NODES: u64 = 50_000_000;
    let expected = STANDARD_FENS
        .iter()
        .map(|&f| ExpectedPerftRes::new(f))
        .collect::<Vec<_>>();
    let mut nps = Vec::with_capacity(expected.len());
    let start = Instant::now();
    let mut nodes = 0;
    let mut min_depth = usize::MAX;
    for (i, r) in expected.iter().enumerate() {
        let depth = r.res.iter().rposition(|&n| n <= MAX_NODES).unwrap();
        let before = Instant::now();
        let perft_res = perft_impl(Position::from_fen(r.fen).unwrap(), depth);
        nodes += perft_res;
        let elapsed = before.elapsed().as_secs_f64();
        nps.push((perft_res as f64 / elapsed / 1_000_000., r.fen));
        min_depth = min(min_depth, depth);
        assert_eq!(
            r.res[depth], perft_res,
            "position {i}, depth {depth}, FEN '{}'",
            r.fen
        );
    }
    let time = start.elapsed();
    let res = PerftRes {
        time,
        nodes,
        depth: min_depth,
    };
    nps.sort_by(|(a_nps, _), (b_nps, _)| f64::total_cmp(a_nps, b_nps));
    let _fens = nps.iter().map(|(_, fen)| *fen).collect::<Vec<_>>();
    let n = nps.len();
    let nps = nps.iter().map(|(nps, _)| *nps).collect::<Vec<_>>();
    let avg = nps.iter().sum::<f64>() / n as f64;
    let median = nps[n / 2];
    let a = nps[n / 20];
    let b = nps[n * 19 / 20];
    let l = nps[1];
    let h = nps[n - 1];
    println!("{res}");
    println!(
        "Mnps: avg {avg:.1}, median {median:.1}, 5th/95th percentile [{a:.1}, {b:.1}], span[{l:.1}, {h:.1}]"
    );
}
