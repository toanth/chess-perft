use crate::core::Res;
use crate::moves::Move;
use crate::perft::{perft_impl, splitperft, PerftRes};
use crate::pos::Position;
use crate::test_positions::{ExpectedPerftRes, STANDARD_FENS};
use std::cmp::min;
use std::env::args;
use std::io;
use std::time::Instant;

mod pos;
mod movegen;
mod core;
mod moves;
mod perft;
#[cfg(test)]
mod perft_tests;
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

fn read_depth_and_fen(mut pos: Position) -> Res<Option<(usize, Position)>> {
    let mut input = String::new();
    let mut line;
    loop {
        input.clear();
        io::stdin().read_line(&mut input).unwrap();
        line = input.trim_ascii();
        if line == "x" || line == "quit" {
            return Ok(None);
        } else if line == "uci" {
            println!("id name chess-perft");
            println!("id author ToTheAnd");
            println!("uciok");
        } else if line.starts_with("position ") {
            pos = parse_pos(line)?;
        } else if line == "show" || line == "debug" || line == "d" {
            println!("{pos}");
        } else {
            break;
        }
    }
    line = line.strip_prefix("go ").unwrap_or(line);
    line = line.strip_prefix("perft ").unwrap_or(line);
    line = line.strip_prefix("depth ").unwrap_or(line);

    let Some((depth, fen)) = line.split_once(" ") else {
        let depth = line.parse::<usize>().map_err(|_| format!("Failed to parse depth '{line}'"))?;
        return Ok(Some((depth, pos)));
    };
    let depth = depth.parse::<usize>().map_err(|_| format!("Failed to parse depth '{depth}'"))?;
    pos = parse_pos(fen)?;
    Ok(Some((depth, pos)))
}

fn main() -> Res<()> {
    let args = args().collect::<Vec<String>>();
    if args.iter().find(|&x| x == "--benchmark" || x == "-b").is_some() {
        benchmark();
        return Ok(());
    }
    println!("Enter a depth and FEN, or 'x' to quit");
    loop {
        let read = read_depth_and_fen(Position::new());
        match read {
            Err(err) => {
                println!("{err}");
                continue;
            }
            Ok(None) => return Ok(()),
            Ok(Some((depth, pos))) => {
                let perft_res = splitperft(pos, depth);
                println!("{perft_res}");
            }
        };
    }
}

fn benchmark() {
    const MAX_NODES: u64 = 10_000_000;
    let expected = STANDARD_FENS.iter().map(|&f| ExpectedPerftRes::new(f)).collect::<Vec<_>>();
    let start = Instant::now();
    let mut nodes = 0;
    let mut min_depth = usize::MAX;
    for (i, r) in expected.iter().enumerate() {
        let depth = r.res.iter().rposition(|&n| n <= MAX_NODES).unwrap();
        let perft_res = perft_impl(Position::from_fen(r.fen).unwrap(), depth);
        nodes += perft_res;
        min_depth = min(min_depth, depth);
        assert_eq!(r.res[depth], perft_res, "position {i}, depth {depth}, FEN '{}'", r.fen);
    }
    let time = start.elapsed();
    let res = PerftRes {
        time,
        nodes,
        depth: min_depth,
    };
    println!("{res}");
}
