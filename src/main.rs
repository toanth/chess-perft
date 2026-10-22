use crate::core::Res;
use crate::perft::{perft_impl, splitperft};
use crate::pos::Position;
use crate::test_positions::{ExpectedPerftRes, STANDARD_FENS};
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

fn read_depth_and_fen() -> Res<Option<(usize, Position)>> {
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    let mut line = line.trim_ascii();
    if line == "x" || line == "quit" {
        return Ok(None);
    }
    line = line.strip_prefix("go ").unwrap_or(line);
    line = line.strip_prefix("perft ").unwrap_or(line);
    line = line.strip_prefix("depth ").unwrap_or(line);

    let Some((depth, mut fen)) = line.split_once(" ") else {
        return Err(format!("Invalid input: '{}'. Expected depth and FEN", line));
    };
    let depth = depth.parse::<usize>().map_err(|_| format!("Failed to parse depth '{depth}'"))?;
    fen = fen.strip_prefix("position ").unwrap_or(fen);
    fen = fen.strip_prefix("fen ").unwrap_or(fen);
    let pos = Position::from_fen(&fen)?;
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
        let read = read_depth_and_fen();
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
    for (i, r) in expected.iter().enumerate() {
        let depth = r.res.iter().rposition(|&n| n <= MAX_NODES).unwrap();
        let perft_res = perft_impl(Position::from_fen(r.fen).unwrap(), depth);
        assert_eq!(r.res[depth], perft_res, "position {i}, depth {depth}, FEN '{}'", r.fen);
    }
    let end = start.elapsed();
    println!("Finished in {}ms", end.as_millis());
}
