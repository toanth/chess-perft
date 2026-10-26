use crate::movegen::MoveListCallback;
use crate::moves::Move;
use crate::perft::BulkCounting::{Bulk, NoBulk};
use crate::pos::Position;
use std::fmt;
use std::fmt::Display;
use std::marker::ConstParamTy;
use std::time::{Duration, Instant};

#[derive(Debug, Copy, Clone, Eq, PartialEq, ConstParamTy)]
pub enum BulkCounting {
    NoBulk,
    Bulk,
}

pub fn perft_impl<const B: BulkCounting>(pos: Position, depth: usize) -> u64 {
    if B == NoBulk && depth == 0 {
        return 1;
    } else if B == Bulk && depth == 1 {
        return pos.num_moves() as u64;
    }
    let mut res = 0;
    let f = |m: Move| {
        let mut new_pos = pos;
        new_pos.make_move(m);
        res += perft_impl::<B>(new_pos, depth - 1);
    };
    pos.legal_moves_impl(MoveListCallback { f });
    res
}

#[allow(dead_code)]
pub fn perft(pos: Position, depth: usize, bulk: BulkCounting) -> PerftRes {
    let start = Instant::now();
    let nodes = if bulk == NoBulk {
        perft_impl::<{ NoBulk }>(pos, depth)
    } else if depth == 0 {
        1
    } else {
        perft_impl::<{ Bulk }>(pos, depth)
    };
    PerftRes {
        time: start.elapsed(),
        nodes,
        depth,
    }
}

pub fn splitperft(pos: Position, depth: usize, bulk: BulkCounting) -> SplitPerftRes {
    assert!(depth > 0);
    let mut children = vec![];
    let mut nodes = 0;
    let start = Instant::now();
    for m in pos.legal_moves() {
        let child_nodes = if depth == 1 {
            1
        } else {
            let mut child = pos;
            child.make_move(m);
            if bulk == NoBulk {
                perft_impl::<{ NoBulk }>(child, depth - 1)
            } else {
                perft_impl::<{ Bulk }>(child, depth - 1)
            }
        };
        nodes += child_nodes;
        children.push((m, child_nodes));
    }
    let time = start.elapsed();
    let perft_res = PerftRes { time, nodes, depth };
    SplitPerftRes {
        perft_res,
        children,
    }
}

fn format_time(time: Duration) -> String {
    let time = time.as_millis();
    if time < 10_000 {
        format!("{}ms", time)
    } else if time < 60_000 {
        format!("{:.1}s", time as f64 / 1000.)
    } else {
        format!("{:.1}min", time as f64 / 60_000.)
    }
}

fn format_nps(nodes: u64, time: Duration) -> String {
    let mut nps = nodes as f64 / time.as_secs_f64();
    let suffix = ["", "K", "M", "B", "T", "Q"];
    for suffix in suffix {
        if nps < 10_000. {
            return format!("{nps:.1}{suffix}");
        }
        nps /= 1_000.;
    }
    format!("{:.0}Q", nps * 1_000.)
}

#[derive(Copy, Clone, Debug)]
pub struct PerftRes {
    pub time: Duration,
    pub nodes: u64,
    pub depth: usize,
}

impl Display for PerftRes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time = format_time(self.time);
        let nps = format_nps(self.nodes, self.time);
        write!(
            f,
            "Finished perft depth {depth} in {time} ({nps}nps)\nNodes: {nodes}",
            depth = self.depth,
            nodes = self.nodes,
        )
    }
}

#[derive(Debug)]
pub struct SplitPerftRes {
    pub perft_res: PerftRes,
    pub children: Vec<(Move, u64)>,
}

impl Display for SplitPerftRes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for child in &self.children {
            writeln!(f, "{0}\t{1}", child.0, child.1)?;
        }
        let nodes = self.perft_res.nodes;
        let time = self.perft_res.time;
        let nps = format_nps(nodes, time);
        let time = format_time(time);
        write!(
            f,
            "Finished splitperft depth {depth} with {children} children in {time} ({nps}nps)\nNodes: {nodes}",
            depth = self.perft_res.depth,
            children = self.children.len(),
        )?;
        Ok(())
    }
}
