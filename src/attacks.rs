use crate::core::Color::{Black, White};
use crate::core::{Bitboard, File, NUM_PLAYERS, NUM_SQUARES, Rank, Square};
use std::simd::u64x4;
#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
use std::sync::LazyLock;

pub const PAWN_CAPTS: [[Bitboard; NUM_SQUARES]; NUM_PLAYERS] = {
    let mut res = [[Bitboard(0); NUM_SQUARES]; NUM_PLAYERS];
    let mut i = 0;
    while i < NUM_SQUARES {
        let bb = Bitboard(1 << i).north();
        res[White as usize][i] = Bitboard(bb.west().0 | bb.east().0);
        let bb = Bitboard(1 << i).south();
        res[Black as usize][i] = Bitboard(bb.west().0 | bb.east().0);
        i += 1;
    }
    res
};

const fn calc_king(sq: usize) -> Bitboard {
    let sq = Bitboard(1 << sq);
    let bb = Bitboard(sq.0 | sq.west().0 | sq.east().0);
    Bitboard(sq.0 ^ bb.0 | bb.north().0 | bb.south().0)
}

pub const KINGS: [Bitboard; NUM_SQUARES] = {
    let mut kings = [Bitboard(0); NUM_SQUARES];
    let mut i = 0;
    while i < NUM_SQUARES {
        kings[i] = calc_king(i);
        i += 1;
    }
    kings
};

const fn calc_knight(sq: usize) -> Bitboard {
    let sq = Bitboard(1 << sq);
    let w = sq.west().west();
    let e = sq.east().east();
    let h = Bitboard(w.0 | e.0);
    let n = sq.north().north();
    let s = sq.south().south();
    let v = Bitboard(n.0 | s.0);
    Bitboard(h.north().0 | h.south().0 | v.east().0 | v.west().0)
}

pub const KNIGHTS: [Bitboard; NUM_SQUARES] = {
    let mut knights = [Bitboard(0); NUM_SQUARES];
    let mut i = 0;
    while i < NUM_SQUARES {
        knights[i] = calc_knight(i);
        i += 1;
    }
    knights
};

// Attacks of a rook on an empty board, without the rook's square itself
pub const ROOKS: [Bitboard; NUM_SQUARES] = {
    let mut rooks = [Bitboard(0); NUM_SQUARES];
    let mut i = 0;
    while i < NUM_SQUARES {
        rooks[i] = Bitboard((Bitboard::RANK_1.0 << (i / 8 * 8)) ^ (Bitboard::A_FILE.0 << (i % 8)));
        i += 1;
    }
    rooks
};

// Attacks of a bishop on an empty board, without the bishop's square itself
pub const BISHOPS: [Bitboard; NUM_SQUARES] = {
    let mut bishops = [Bitboard(0); NUM_SQUARES];
    let mut i = 0;
    while i < NUM_SQUARES {
        bishops[i] = Bitboard(DIAGONALS[i].0 ^ ANTI_DIAGONALS[i].0);
        i += 1;
    }
    bishops
};

#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
pub const RELEVANT_ROOKS: [Bitboard; NUM_SQUARES] = {
    let mut res = [Bitboard(0); NUM_SQUARES];
    let mut i = 0;
    while i < NUM_SQUARES {
        res[i] = ROOKS[i];
        let rank = Square::new(i).rank() as usize;
        if rank != Rank::R1 as usize {
            res[i].0 &= !Bitboard::RANK_1.0;
        }
        if rank != Rank::R8 as usize {
            res[i].0 &= !Bitboard::RANK_8.0;
        }
        let file = Square::new(i).file() as usize;
        if file != File::A as usize {
            res[i].0 &= !Bitboard::A_FILE.0;
        }
        if file != File::H as usize {
            res[i].0 &= !Bitboard::H_FILE.0;
        }
        i += 1;
    }
    res
};

#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
pub const RELEVANT_BISHOPS: [Bitboard; NUM_SQUARES] = {
    let mut res = [Bitboard(0); NUM_SQUARES];
    let mut i = 0;
    while i < NUM_SQUARES {
        res[i].0 = BISHOPS[i].0 & !Bitboard::EDGES.0;
        i += 1;
    }
    res
};

#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
const RELEVANT_COMBINATIONS: usize = 1 << 12;

#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
type SliderGen = fn(Square, Bitboard) -> Bitboard;

#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
fn calc_slider_table(
    slider_gen: SliderGen,
    relevant: &[Bitboard; NUM_SQUARES],
) -> Box<[[Bitboard; RELEVANT_COMBINATIONS]; NUM_SQUARES]> {
    let mut sliders = vec![[Bitboard(0); RELEVANT_COMBINATIONS]; NUM_SQUARES];
    for sq in Square::iter() {
        debug_assert!(!relevant[sq].has(sq));
        let relevant = relevant[sq];
        assert_ne!(relevant, Bitboard(0));
        let mut comb = Bitboard(0);
        loop {
            comb.0 = comb.0.wrapping_sub(relevant.0) & relevant.0;
            let res = slider_gen(sq, comb);
            assert_ne!(res, Bitboard(0));
            assert!(!res.has(sq));
            let index = pext(comb, relevant);
            sliders[sq as usize][index.0 as usize] = res;
            if comb == Bitboard(0) {
                break;
            }
        }
    }
    sliders.into_array().unwrap()
}

#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
fn calc_rooks() -> Box<[[Bitboard; RELEVANT_COMBINATIONS]; NUM_SQUARES]> {
    calc_slider_table(rook_attacks_hq, &RELEVANT_ROOKS)
}

#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
fn calc_bishops() -> Box<[[Bitboard; RELEVANT_COMBINATIONS]; NUM_SQUARES]> {
    calc_slider_table(bishop_attacks_hq, &RELEVANT_BISHOPS)
}

// There is the unstable .extract_bits function, but at the time of writing this *never* emits `pext`
// even on targets where pext is known to exist and be fast, which makes it essentially useless.
#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
fn pext(bb: Bitboard, mask: Bitboard) -> Bitboard {
    unsafe { Bitboard(std::arch::x86_64::_pext_u64(bb.0, mask.0)) }
}

#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
static ROOK_LOOKUP: LazyLock<Box<[[Bitboard; RELEVANT_COMBINATIONS]; NUM_SQUARES]>> =
    LazyLock::new(|| calc_rooks());

#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
static BISHOP_LOOKUP: LazyLock<Box<[[Bitboard; RELEVANT_COMBINATIONS]; NUM_SQUARES]>> =
    LazyLock::new(|| calc_bishops());

pub fn force_init() {
    #[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
    {
        LazyLock::force(&ROOK_LOOKUP);
        LazyLock::force(&BISHOP_LOOKUP);
    }
}

pub fn rook_attacks_hq(square: Square, blockers: Bitboard) -> Bitboard {
    hq(
        square,
        blockers,
        Bitboard::file(square.file()) ^ square.bb(),
    ) | horizontal_attacks(square, blockers)
}

pub fn bishop_attacks_hq(square: Square, blockers: Bitboard) -> Bitboard {
    hq(square, blockers, DIAGONALS[square]) | hq(square, blockers, ANTI_DIAGONALS[square])
}

#[cfg(not(all(feature = "pext-lookup", target_feature = "bmi2")))]
pub fn rook_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    rook_attacks_hq(square, blockers)
}

#[cfg(not(all(feature = "pext-lookup", target_feature = "bmi2")))]
pub fn bishop_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    bishop_attacks_hq(square, blockers)
}

#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
pub fn rook_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    let index = pext(blockers, RELEVANT_ROOKS[square]).0 as usize;
    let res = ROOK_LOOKUP[square][index];
    debug_assert_eq!(res, rook_attacks_hq(square, blockers));
    res
}

#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
pub fn bishop_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    let index = pext(blockers, RELEVANT_BISHOPS[square]).0 as usize;
    let res = BISHOP_LOOKUP[square][index];
    debug_assert_eq!(res, bishop_attacks_hq(square, blockers));
    res
}

pub fn queen_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    rook_attacks(square, blockers) | bishop_attacks(square, blockers)
}

fn hq(sq: Square, blockers: Bitboard, ray: Bitboard) -> Bitboard {
    let blockers = blockers & ray;
    let forward = blockers.0.wrapping_sub(Bitboard::single(sq).0);
    let backwards = blockers
        .swap_bytes()
        .0
        .wrapping_sub(Bitboard::single(sq).swap_bytes().0);
    Bitboard(forward ^ backwards.swap_bytes()) & ray
}

pub fn horizontal_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    let idx = square as usize;
    let rank_shift = idx & 0b111_000;
    let shifted = (blockers.0 >> rank_shift) & 0b0111_1110;
    let res = RANK_LOOKUP[(4 * shifted) as usize + (idx & 0b000_111)] as u64;
    Bitboard(res << rank_shift)
}

static RANK_LOOKUP: [u8; 8 * (1 << (8 - 2))] = {
    let mut res = [0; 64 * 8];
    let mut i = 0;
    while i < 64 {
        let mut sq = 0;
        while sq < 8 {
            let sq_bb = 1 << sq;
            let blockers: u8 = (i << 1) & !sq_bb;
            let attacks = blockers.wrapping_sub(sq_bb);
            let rev = blockers.reverse_bits().wrapping_sub(sq_bb.reverse_bits());
            let attacks = rev.reverse_bits() ^ attacks;
            res[i as usize * 8 + sq as usize] = attacks;
            sq += 1;
        }
        i += 1;
    }
    res
};

const fn steps(n: usize) -> Bitboard {
    let mut res = Bitboard(0);
    let mut i = 0;
    while i < 64 {
        res.0 |= 1 << i;
        i += n;
    }
    res
}

const fn remove_ones_above(bb: u64, idx: usize) -> u64 {
    if idx < 64 {
        bb & (u64::MAX >> (63 - idx))
    } else {
        bb
    }
}

const fn remove_ones_below(bb: u64, idx: usize) -> u64 {
    bb & (u64::MAX << idx)
}

pub const DIAGONALS: [Bitboard; NUM_SQUARES] = {
    let mut res = [Bitboard(0); NUM_SQUARES];
    let steps = steps(9);
    let mut i = 0;
    while i < NUM_SQUARES {
        let diag = (i / 8) as isize - (i % 8) as isize;
        if diag > 0 {
            let diag = diag as usize;
            res[i] = Bitboard(steps.0 << (diag * 8));
            res[i] = Bitboard(remove_ones_above(res[i].0, (8 + diag + 1) * 8) ^ (1 << i));
        } else {
            let diag = -diag as usize;
            res[i] = Bitboard(remove_ones_below(steps.0 << diag, diag));
            res[i] = Bitboard(remove_ones_above(res[i].0, (8 - diag) * 8) ^ (1 << i));
        }
        i += 1;
    }
    res
};

pub const ANTI_DIAGONALS: [Bitboard; NUM_SQUARES] = {
    let mut res: [Bitboard; NUM_SQUARES] = [Bitboard(0); 64];
    let steps = steps(7);
    let mut i = 0;
    while i < 64 {
        let anti_diag = i / 8 + i % 8;
        res[i] = Bitboard(remove_ones_above(steps.0 << anti_diag, anti_diag * 8) ^ (1 << i));
        if anti_diag >= 8 {
            res[i] = Bitboard(remove_ones_below(res[i].0, (anti_diag - 8 + 2) * 8 - 1));
        }
        i += 1;
    }
    res
};

const fn ray_between_exclusive(a: usize, b: usize, ray: u64) -> Bitboard {
    let a = 1 << a;
    let b = 1 << b;
    let max = if a > b { a } else { b };
    let min = if a < b { a } else { b };
    Bitboard((max - min) & ray & !min)
}

// `static` instead of `const` because it's pretty large
pub static RAYS_EXCLUSIVE: [[Bitboard; 64]; 64] = {
    let mut res = [[Bitboard(0); 64]; 64];
    let mut start = 0;
    while start < 64 {
        let file = start % 8;
        let rank = start / 8;
        let mut i = 0;
        while i < 8 {
            let sq = rank * 8 + i;
            res[start][sq] = ray_between_exclusive(start, sq, Bitboard::RANK_1.0 << 8 * rank);
            let sq = 8 * i + file;
            res[start][sq] = ray_between_exclusive(start, sq, Bitboard::A_FILE.0 << file);
            i += 1;
        }
        let mut diag = DIAGONALS[start].0;
        while diag != 0 {
            let sq = diag.trailing_zeros() as usize;
            res[start][sq] = ray_between_exclusive(start, sq, DIAGONALS[start].0);
            diag &= diag - 1;
        }
        let mut anti_diag = ANTI_DIAGONALS[start].0;
        while anti_diag != 0 {
            let sq = anti_diag.trailing_zeros() as usize;
            res[start][sq] = ray_between_exclusive(start, sq, ANTI_DIAGONALS[start].0);
            anti_diag &= anti_diag - 1;
        }
        start += 1;
    }
    res
};

/// If the squares are not on a ray, this still includes both squares
pub static RAYS_INCLUSIVE: [[Bitboard; 64]; 64] = {
    let mut res = [[Bitboard(0); 64]; 64];
    let mut a = 0;
    while a < 64 {
        let mut b = 0;
        while b < 64 {
            res[a][b] = Bitboard(RAYS_EXCLUSIVE[a][b].0 | (1 << a) | (1 << b));
            b += 1;
        }
        a += 1;
    }
    res
};

pub static RAYS_INFINITE: [[Bitboard; 64]; 64] = {
    let mut res = [[Bitboard(0); 64]; 64];
    let mut a = 0;
    while a < 64 {
        let mut b = 0;
        while b < 64 {
            let r = if a == b {
                0
            } else if a % 8 == b % 8 {
                Bitboard::A_FILE.0 << (b % 8)
            } else if a / 8 == b / 8 {
                Bitboard::RANK_1.0 << (b / 8 * 8)
            } else if DIAGONALS[a].0 & (1 << b) != 0 {
                DIAGONALS[a].0
            } else if ANTI_DIAGONALS[a].0 & (1 << b) != 0 {
                ANTI_DIAGONALS[a].0
            } else {
                0
            };

            res[a][b] = Bitboard(r);
            b += 1;
        }
        a += 1;
    }
    res
};

// a manual SIMD implementation wasn't faster than this
pub fn all_knight_attacks(knights: Bitboard) -> Bitboard {
    let masks = [
        !Bitboard::A_FILE,
        !(Bitboard::A_FILE | Bitboard::file(File::B)),
        !Bitboard::file(File::H),
        !(Bitboard::file(File::G) | Bitboard::file(File::H)),
    ];
    let up = ((knights & masks[0]) << 2 * 8 - 1)
        | ((knights & masks[1]) << 8 - 2)
        | ((knights & masks[2]) << 2 * 8 + 1)
        | ((knights & masks[3]) << 8 + 2);
    let down = ((knights & masks[0]) >> 2 * 8 + 1)
        | ((knights & masks[1]) >> 8 + 2)
        | ((knights & masks[2]) >> 2 * 8 - 1)
        | ((knights & masks[3]) >> 8 - 2);
    up | down
}

type U64x4 = std::simd::Simd<u64, 4>;

pub fn all_slider_attacks(bishops: Bitboard, rooks: Bitboard, blockers: Bitboard) -> Bitboard {
    let fwd_filters = U64x4::from_array([
        !Bitboard::A_FILE.0,
        !Bitboard::H_FILE.0,
        !0,
        !Bitboard::A_FILE.0,
    ]);
    let bwd_filters = U64x4::from_array([
        !Bitboard::H_FILE.0,
        !Bitboard::A_FILE.0,
        !0,
        !Bitboard::H_FILE.0,
    ]);
    let sliders = U64x4::from_array([bishops.0, bishops.0, rooks.0, rooks.0]);
    let empty = u64x4::splat(!blockers.0);
    let mut forward = sliders;
    let mut backward = sliders;
    let mut shift = U64x4::from_array([9, 7, 8, 1]);
    let shift_once = shift;
    let mut fwd_allowed = empty & fwd_filters;
    let mut bwd_allowed = empty & bwd_filters;
    forward |= fwd_allowed & (forward << shift);
    backward |= bwd_allowed & (backward >> shift);
    for _ in 0..2 {
        fwd_allowed &= fwd_allowed << shift;
        bwd_allowed &= bwd_allowed >> shift;
        shift <<= 1;
        forward |= fwd_allowed & (forward << shift);
        backward |= bwd_allowed & (backward >> shift);
    }
    let res = ((forward << shift_once) & fwd_filters) | ((backward >> shift_once) & bwd_filters);
    let &[a, b, c, d] = res.as_array();
    Bitboard(a | b | c | d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_slider_attacks() {
        let bishops = Bitboard(0);
        let rooks = Bitboard(0x400);
        let blockers = Bitboard(0x400);
        let attacks = all_slider_attacks(bishops, rooks, blockers);
        let expected = Bitboard(0x40404040404fb04);
        assert_eq!(attacks, expected);
        let blockers = Bitboard(0x440018184004);
        let attacks = all_slider_attacks(bishops, rooks, blockers);
        let expected = Bitboard(0x40404047b04);
        assert_eq!(attacks, expected);

        let bishops = Bitboard(0x8000000);
        let rooks = Bitboard(0x20000000);
        let blockers = Bitboard(0x200000240000) | rooks | bishops;
        let attacks = all_slider_attacks(bishops, rooks, blockers);
        let expected = Bitboard(0x12234d8342040);
        assert_eq!(attacks, expected);

        let bishops = Bitboard(0x420000004);
        let rooks = Bitboard(0x2120000000);
        let blockers = Bitboard(0x806a002520920004);
        let attacks = all_slider_attacks(bishops, rooks, blockers);
        let expected = Bitboard(0x2335abfeff71ab21);
        assert_eq!(attacks, expected);
    }
}
