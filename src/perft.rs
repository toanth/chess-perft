use crate::moves::Move;
use crate::pos::Position;
use std::fmt;
use std::fmt::Display;
use std::time::{Duration, Instant};

pub fn perft_impl(pos: Position, depth: usize) -> u64 {
    if depth == 0 {
        return 1;
    }
    let mut res = 0;
    for m in pos.legal_moves() {
        let child = pos.make_move(m);
        res += perft_impl(child, depth - 1);
    }
    res
}

#[allow(dead_code)]
pub fn perft(pos: Position, depth: usize) -> PerftRes {
    let start = Instant::now();
    let nodes = perft_impl(pos, depth);
    PerftRes {
        time: start.elapsed(),
        nodes,
        depth,
    }
}

pub fn splitperft(pos: Position, depth: usize) -> SplitPerftRes {
    assert!(depth > 0);
    let mut children = vec![];
    let mut nodes = 0;
    let start = Instant::now();
    for m in pos.legal_moves() {
        let child = pos.make_move(m);
        let child_nodes = perft_impl(child, depth - 1);
        nodes += child_nodes;
        children.push((m, child_nodes));
    }
    let time = start.elapsed();
    let perft_res = PerftRes {
        time,
        nodes,
        depth,
    };
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
            "Finished splitperft depth {depth} in {time} ({nps}nps) with {children} children\nNodes: {nodes}",
            depth = self.perft_res.depth,
            children = self.children.len(),
        )?;
        Ok(())
    }
}