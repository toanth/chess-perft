use crate::core::Color::{Black, White};
use crate::core::PieceType::*;
use crate::core::{Bitboard, Color, File, Rank, Square, NUM_PLAYERS, NUM_SQUARES};
use crate::moves::MoveFlags::{PromoBishop, PromoKnight, PromoQueen, PromoRook};
use crate::moves::{Move, MoveFlags, MoveList};
use crate::pos::Position;
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
fn calc_slider_table(slider_gen: SliderGen, relevant: &[Bitboard; NUM_SQUARES]) -> Box<[[Bitboard; RELEVANT_COMBINATIONS]; NUM_SQUARES]> {
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
    unsafe {
        Bitboard(std::arch::x86_64::_pext_u64(bb.0, mask.0))
    }
}

#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
static ROOK_LOOKUP: LazyLock<Box<[[Bitboard; RELEVANT_COMBINATIONS]; NUM_SQUARES]>> = LazyLock::new(|| calc_rooks());

#[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
static BISHOP_LOOKUP: LazyLock<Box<[[Bitboard; RELEVANT_COMBINATIONS]; NUM_SQUARES]>> = LazyLock::new(|| calc_bishops());

pub fn force_init() {
    #[cfg(all(feature = "pext-lookup", target_feature = "bmi2"))]
    {
        LazyLock::force(&ROOK_LOOKUP);
        LazyLock::force(&BISHOP_LOOKUP);
    }
}

pub fn rook_attacks_hq(square: Square, blockers: Bitboard) -> Bitboard {
    hq(square, blockers, Bitboard::file(square.file()) ^ square.bb()) | horizontal_attacks(square, blockers)
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
    assert!(cfg!(target_feature = "bmi2"), "no bmi2");
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

// TODO: Optimize (similar to motors?), also test pext magic bitboards
fn hq(sq: Square, blockers: Bitboard, ray: Bitboard) -> Bitboard {
    let blockers = blockers & ray;
    let forward = blockers.0.wrapping_sub(Bitboard::single(sq).0);
    let backwards = blockers.swap_bytes().0.wrapping_sub(Bitboard::single(sq).swap_bytes().0);
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
    if idx < 64 { bb & (u64::MAX >> (63 - idx)) } else { bb }
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
        #[allow(clippy::assign_op_pattern)]
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

// TODO: Manual AVX2 impl?
pub fn kogge_stone<const DIR: usize>(
    sliders: Bitboard,
    empty: Bitboard,
    fwd_filter: Bitboard,
    bwd_filter: Bitboard,
) -> Bitboard {
    let mut forward = sliders;
    let mut fwd_allowed = empty & fwd_filter;
    let mut backward = sliders;
    let mut bwd_allowed = empty & bwd_filter;
    forward |= fwd_allowed & (forward << DIR);
    backward |= bwd_allowed & (backward >> DIR);
    fwd_allowed &= fwd_allowed << DIR;
    bwd_allowed &= bwd_allowed >> DIR;
    forward |= fwd_allowed & (forward << 2 * DIR);
    backward |= bwd_allowed & (backward >> 2 * DIR);
    fwd_allowed &= fwd_allowed << 2 * DIR;
    bwd_allowed &= bwd_allowed >> 2 * DIR;
    forward |= fwd_allowed & (forward << 4 * DIR);
    backward |= bwd_allowed & (backward >> 4 * DIR);
    ((forward << DIR) & fwd_filter) | ((backward >> DIR) & bwd_filter)
}

pub fn all_knight_attacks(knights: Bitboard) -> Bitboard {
    let masks = [
        !Bitboard::A_FILE,
        !(Bitboard::A_FILE | Bitboard::file(File::B)),
        !Bitboard::file(File::H),
        !(Bitboard::file(File::G) | Bitboard::file(File::H)),
    ];
    let up = ((knights & masks[0]) << 2 * 8 - 1) | ((knights & masks[1]) << 8 - 2) | ((knights & masks[2]) << 2 * 8 + 1) | ((knights & masks[3]) << 8 + 2);
    let down = ((knights & masks[0]) >> 2 * 8 + 1) | ((knights & masks[1]) >> 8 + 2) | ((knights & masks[2]) >> 2 * 8 - 1) | ((knights & masks[3]) >> 8 - 2);
    up | down
}

pub fn all_rook_attacks(sliders: Bitboard, blockers: Bitboard) -> Bitboard {
    let all = Bitboard(!0);
    let vertical = kogge_stone::<8>(sliders, !blockers, all, all);
    let horizontal = kogge_stone::<1>(sliders, !blockers, !Bitboard::A_FILE, !Bitboard::file(File::H));
    horizontal | vertical
}

pub fn all_bishop_attacks(sliders: Bitboard, blockers: Bitboard) -> Bitboard {
    let not_a_file = !Bitboard::A_FILE;
    let not_h_file = !Bitboard::file(File::H);
    let diagonal = kogge_stone::<9>(sliders, !blockers, not_a_file, not_h_file);
    let anti_diagonal = kogge_stone::<7>(sliders, !blockers, not_h_file, not_a_file);
    diagonal | anti_diagonal
}


pub trait MoveGenCallback {
    fn gen_attacks(&mut self, bb: Bitboard, src: Square, flag: MoveFlags);
    fn gen_single(&mut self, src: Square, dest: Square, flag: MoveFlags);
    unsafe fn gen_pawns(&mut self, dests: Bitboard, offset: i8, flag: MoveFlags);
    unsafe fn gen_promos(&mut self, dests: Bitboard, offset: i8) {
        for flag in [PromoKnight, PromoBishop, PromoRook, PromoQueen] {
            unsafe {
                self.gen_pawns(dests, offset, flag);
            }
        }
    }
}

pub struct MoveListCallback<F: FnMut(Move)> {
    pub f: F,
}

impl<F: FnMut(Move)> MoveGenCallback for MoveListCallback<F> {
    fn gen_attacks(&mut self, bb: Bitboard, src: Square, flag: MoveFlags) {
        for dest in bb {
            (self.f)(Move::new(src, dest, flag))
        }
    }

    fn gen_single(&mut self, src: Square, dest: Square, flag: MoveFlags) {
        (self.f)(Move::new(src, dest, flag))
    }

    unsafe fn gen_pawns(&mut self, dests: Bitboard, offset: i8, flag: MoveFlags) {
        for dest in dests {
            let src = unsafe { dest.add(offset) };
            (self.f)(Move::new(src, dest, flag));
        }
    }
}

struct CountMovesCallback<'a> {
    ctr: &'a mut usize,
}

impl MoveGenCallback for CountMovesCallback<'_> {
    fn gen_attacks(&mut self, bb: Bitboard, _src: Square, _flag: MoveFlags) {
        *self.ctr += bb.count_ones();
    }

    fn gen_single(&mut self, _src: Square, _dest: Square, _flag: MoveFlags) {
        *self.ctr += 1;
    }

    unsafe fn gen_pawns(&mut self, dests: Bitboard, _offset: i8, _flag: MoveFlags) {
        *self.ctr += dests.count_ones();
    }
}

impl Position {
    pub fn legal_moves(&self) -> MoveList {
        let mut res = MoveList::new();
        let f = |m: Move| res.push(m);
        let callback = MoveListCallback { f };
        self.legal_moves_impl(callback);
        res
    }

    pub fn num_moves(&self) -> usize {
        let mut res = 0;
        let callback = CountMovesCallback { ctr: &mut res };
        self.legal_moves_impl(callback);
        res
    }

    pub fn legal_moves_impl<C: MoveGenCallback>(&self, mut callback: C)
    {
        let us = self.stm;
        let king = self.king_sq(us);
        let our_pieces = self.pieces.colors[us];
        let (checkers, pinned) = self.calc_checkers_and_pinned(king, us);
        let relevant_attacks = self.calc_relevant_attacks_for(!us);
        callback.gen_attacks(KINGS[king] & !our_pieces & !relevant_attacks, king, MoveFlags::Normal);
        let check_ray = match checkers.count_ones() {
            0 => Bitboard(!0),
            1 => RAYS_INCLUSIVE[king][checkers.sq()],
            _ => return,
        };
        let filter = check_ray & !our_pieces;
        for knight in self.col_piece_bb(Knight, us) & !pinned {
            callback.gen_attacks(KNIGHTS[knight] & filter, knight, MoveFlags::Normal);
        }
        let blockers = self.occupied_bb();
        let bishops = self.col_piece_bb(Bishop, us);
        Self::gen_sliders(bishops, pinned, blockers, filter, king, bishop_attacks, &mut callback);
        let rooks = self.col_piece_bb(Rook, us);
        Self::gen_sliders(rooks, pinned, blockers, filter, king, rook_attacks, &mut callback);
        let queens = self.col_piece_bb(Queen, us);
        Self::gen_sliders(queens, pinned, blockers, filter, king, queen_attacks, &mut callback);
        if us == White {
            self.gen_pawns::<true, C>(pinned, check_ray, &mut callback);
        } else {
            self.gen_pawns::<false, C>(pinned, check_ray, &mut callback);
        }
        for (i, &side) in self.castling[us].iter().enumerate() {
            if let Some(rook) = side {
                let blockers = blockers & !(self.col_piece_bb(King, us) | rook.bb());
                let king_dest = Self::CASTLE_KING_DESTS[us][i];
                let rook_dest = Self::CASTLE_ROOK_DESTS[us][i];
                if !RAYS_INCLUSIVE[king][king_dest].intersects(blockers | relevant_attacks)
                    && !RAYS_INCLUSIVE[rook][rook_dest].intersects(blockers) && !pinned.has(rook)
                {
                    let flag = if i == 0 { MoveFlags::CastleQueenside } else { MoveFlags::CastleKingside };
                    callback.gen_single(king, rook, flag);
                }
            }
        }
    }


    fn gen_sliders<F>(sliders: Bitboard, pinned: Bitboard, blockers: Bitboard, filter: Bitboard, king: Square, f: F, callback: &mut impl MoveGenCallback)
    where
        F: Fn(Square, Bitboard) -> Bitboard,
    {
        for src in sliders & !pinned {
            callback.gen_attacks(f(src, blockers) & filter, src, MoveFlags::Normal);
        }
        for src in sliders & pinned {
            let attacks = f(src, blockers) & filter & RAYS_INFINITE[king][src];
            callback.gen_attacks(attacks, src, MoveFlags::Normal);
        }
    }

    fn gen_pawns<const WHITE: bool, Callback>(&self, pinned: Bitboard, check_ray: Bitboard, callback: &mut impl MoveGenCallback)
    {
        debug_assert_eq!(WHITE, self.stm == White);
        let us = if WHITE { White } else { Black };
        let king = self.king_sq(us);
        let blockers = self.occupied_bb();
        let their_pieces = self.pieces.colors[!us];
        let quiets = !blockers & check_ray;
        let captures = their_pieces & check_ray;
        let pawns = self.col_piece_bb(Pawn, us);
        let pushable = pawns & (!pinned | Bitboard::file(king.file()));
        let push = |bb: Bitboard| { if WHITE { bb.north() } else { bb.south() } };
        let (w, e) = if WHITE { (ANTI_DIAGONALS[king], DIAGONALS[king]) } else { (DIAGONALS[king], ANTI_DIAGONALS[king]) };
        let captures_west = pawns & (!pinned | w);
        let captures_east = pawns & (!pinned | e);
        let captures_west = push(captures_west).west() & captures;
        let captures_east = push(captures_east).east() & captures;
        let double_push_rank = if WHITE { Rank::R4 } else { Rank::R5 };
        let promo_rank = if WHITE { Rank::R8 } else { Rank::R1 };
        let promo_rank = Bitboard::rank(promo_rank);
        let offsets = if WHITE { [-8, -16, -7, -9] } else { [8, 16, 9, 7] };
        unsafe {
            let pushed = push(pushable);
            callback.gen_pawns(pushed & quiets & !promo_rank, offsets[0], MoveFlags::Normal);
            let double = Bitboard::rank(double_push_rank) & push(pushed & !blockers) & quiets;
            callback.gen_pawns(double, offsets[1], MoveFlags::DoublePawnPush);
            callback.gen_pawns(captures_west & !promo_rank, offsets[2], MoveFlags::Normal);
            callback.gen_pawns(captures_east & !promo_rank, offsets[3], MoveFlags::Normal);

            callback.gen_promos(pushed & quiets & promo_rank, offsets[0]);
            callback.gen_promos(captures_west & promo_rank, offsets[2]);
            callback.gen_promos(captures_east & promo_rank, offsets[3]);
        }
        // if we're in check while the ep square is set, it must be from the capturable pawn
        if let Some(ep) = self.ep {
            let captured = unsafe { ep.add(if WHITE { -8 } else { 8 }) };
            if check_ray.intersects(captured.bb()) {
                let can_capture = (captured.bb().west() | captured.bb().east()) & pawns;
                if king.rank() == captured.rank() && can_capture.0 != 0 {
                    let sq_bb = Bitboard(can_capture.0.isolate_lowest_one());
                    let occ_bb = (self.occupied_bb() ^ sq_bb) & !captured.bb();
                    let possibly_pinning = (self.piece_bb(Rook) | self.piece_bb(Queen)) & self.col_bb(!us);
                    // horizontally "pinned" e.p. pawn: The capturing pawn and victim are blocking a check together.
                    if horizontal_attacks(king, occ_bb).intersects(possibly_pinning) {
                        // Only the moved pawn and the capturing pawn are between the king and a horizontal slider,
                        // so the pawn is effectively pinned for ep.
                        return;
                    }
                }
                for src in can_capture {
                    if !pinned.intersects(src.bb()) || RAYS_INFINITE[self.king_sq(us)][src].intersects(ep.bb()) {
                        callback.gen_single(src, ep, MoveFlags::Normal)
                    }
                }
            }
        }
    }

    pub fn attacks_of(&self, sq: Square) -> Bitboard {
        let piece = self.piece_type_on(sq).unwrap();
        let us = self.stm;
        let blockers = self.occupied_bb();
        match piece {
            Pawn => {
                let single_push = sq.bb().pawn_push(us) & !self.occupied_bb();
                let double_pushable = single_push & (Bitboard::rank(Rank::R3) | Bitboard::rank(Rank::R6));
                let double_push = double_pushable.pawn_push(us) & !self.occupied_bb();
                Self::pawn_attacks(sq.bb(), us) | single_push | double_push
            }
            Knight => {
                KNIGHTS[sq]
            }
            Bishop => {
                bishop_attacks(sq, blockers)
            }
            Rook => {
                rook_attacks(sq, blockers)
            }
            Queen => {
                queen_attacks(sq, blockers)
            }
            King => {
                KINGS[sq]
            }
        }
    }

    pub fn pawn_attacks(pawns: Bitboard, color: Color) -> Bitboard {
        let b = match color {
            White => pawns.north(),
            Black => pawns.south(),
        };
        b.west() | b.east()
    }

    // This only needs to include attacks that can prevent a king move or pin a piece
    pub fn calc_relevant_attacks_for(&self, us: Color) -> Bitboard {
        let mut res = KINGS[self.king_sq(us)];
        res |= Self::pawn_attacks(self.col_piece_bb(Pawn, us), us);
        res |= all_knight_attacks(self.col_piece_bb(Knight, us));
        let blockers = self.occupied_bb() ^ self.col_piece_bb(King, !us);
        let bishop_sliders = (self.piece_bb(Bishop) | self.piece_bb(Queen)) & self.col_bb(us);
        res |= all_bishop_attacks(bishop_sliders, blockers);
        let rook_sliders = (self.piece_bb(Rook) | self.piece_bb(Queen)) & self.col_bb(us);
        res |= all_rook_attacks(rook_sliders, blockers);
        res
    }

    pub fn calc_checkers_and_pinned(&self, king: Square, us: Color) -> (Bitboard, Bitboard) {
        let blockers = self.occupied_bb();
        let mut checkers =
            ((PAWN_CAPTS[us][king] & self.piece_bb(Pawn))
                | (KNIGHTS[king] & self.piece_bb(Knight))) & self.col_bb(!us);
        let rook_sliders = self.piece_bb(Rook) | self.piece_bb(Queen);
        let bishop_sliders = self.piece_bb(Bishop) | self.piece_bb(Queen);
        let mut pinned = Bitboard(0);
        let sliders = (bishop_sliders & BISHOPS[king]) | (rook_sliders & ROOKS[king]);
        let sliders = sliders & self.col_bb(!us);
        for slider in sliders {
            let ray = RAYS_EXCLUSIVE[king][slider];
            match (ray & blockers).count_ones() {
                0 => checkers |= slider.bb(),
                1 => pinned |= ray & blockers,
                _ => {}
            }
        }
        (
            checkers,
            pinned,
        )
    }
}
