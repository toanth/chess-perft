use crate::core::Color::{Black, White};
use crate::core::PieceType::*;
use crate::core::{Bitboard, Color, Rank, Square, NUM_PLAYERS, NUM_SQUARES};
use crate::moves::MoveFlags::{CaptPromoBishop, CaptPromoKnight, CaptPromoQueen, CaptPromoRook, EnPassant, PromoBishop, PromoKnight, PromoQueen, PromoRook};
use crate::moves::{Move, MoveFlags, MoveList};
use crate::pos::Position;

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

pub const ROOKS: [Bitboard; NUM_SQUARES] = {
    let mut rooks = [Bitboard(0); NUM_SQUARES];
    let mut i = 0;
    while i < NUM_SQUARES {
        rooks[i] = Bitboard((Bitboard::RANK_1.0 << (i / 8 * 8)) ^ (Bitboard::A_FILE.0 << (i % 8)));
        i += 1;
    }
    rooks
};

pub const BISHOPS: [Bitboard; NUM_SQUARES] = {
    let mut bishops = [Bitboard(0); NUM_SQUARES];
    let mut i = 0;
    while i < NUM_SQUARES {
        bishops[i] = Bitboard(DIAGONALS[i].0 ^ ANTI_DIAGONALS[i].0);
        i += 1;
    }
    bishops
};


pub fn rook_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    hq(square, blockers, Bitboard::file(square.file()) ^ square.bb()) | horizontal_attacks(square, blockers)
}

pub fn bishop_attacks(square: Square, blockers: Bitboard) -> Bitboard {
    hq(square, blockers, DIAGONALS[square]) | hq(square, blockers, ANTI_DIAGONALS[square])
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

impl Position {
    pub fn legal_moves(&self) -> MoveList {
        let mut res = MoveList::new();
        let us = self.stm;
        let king = self.king_sq(us);
        let our_pieces = self.pieces.colors[us];
        let (checkers, pinned) = self.calc_checkers_and_pinned(king, us);
        let relevant_attacks = self.calc_relevant_attacks_for(!us);
        for dest in KINGS[king] & !our_pieces & !relevant_attacks {
            res.push(Move::new(king, dest, MoveFlags::NormalQuiet));
        }
        let check_ray = match checkers.0.count_ones() {
            0 => Bitboard(!0),
            1 => RAYS_INCLUSIVE[king][checkers.sq()],
            _ => return res,
        };
        let filter = check_ray & !our_pieces;
        for knight in self.col_piece_bb(Knight, us) & !pinned {
            for dest in KNIGHTS[knight] & filter {
                res.push(Move::new(knight, dest, MoveFlags::NormalQuiet));
            }
        }
        let blockers = self.occupied_bb();
        let bishops = self.col_piece_bb(Bishop, us);
        Self::gen_sliders(bishops, pinned, blockers, filter, king, bishop_attacks, &mut res);
        let rooks = self.col_piece_bb(Rook, us);
        Self::gen_sliders(rooks, pinned, blockers, filter, king, rook_attacks, &mut res);
        let queens = self.col_piece_bb(Queen, us);
        Self::gen_sliders(queens, pinned, blockers, filter, king, queen_attacks, &mut res);
        if us == White {
            self.gen_pawns::<true>(pinned, check_ray, &mut res);
        } else {
            self.gen_pawns::<false>(pinned, check_ray, &mut res);
        }
        for (i, &side) in self.castling[us].iter().enumerate() {
            if let Some(rook) = side {
                let blockers = blockers & !(self.col_piece_bb(King, us) | rook.bb());
                let king_dest = Self::CASTLE_KING_DESTS[us][i];
                let rook_dest = Self::CASTLE_ROOK_DESTS[us][i];
                if !RAYS_INCLUSIVE[king][king_dest].intersects(blockers | relevant_attacks)
                    && !RAYS_INCLUSIVE[rook][rook_dest].intersects(blockers) && !pinned.has(rook)
                {
                    let flags = if i == 0 { MoveFlags::CastleQueenside } else { MoveFlags::CastleKingside };
                    res.push(Move::new(king, rook, flags));
                }
            }
        }
        res
    }


    fn gen_sliders<F>(sliders: Bitboard, pinned: Bitboard, blockers: Bitboard, filter: Bitboard, king: Square, f: F, res: &mut MoveList)
    where
        F: Fn(Square, Bitboard) -> Bitboard,
    {
        for src in sliders & !pinned {
            let attacks = f(src, blockers) & filter;
            for dest in attacks & !blockers {
                res.push(Move::new(src, dest, MoveFlags::NormalQuiet));
            }
            for dest in attacks & blockers {
                res.push(Move::new(src, dest, MoveFlags::NormalCapture));
            }
        }
        for src in sliders & pinned {
            let attacks = f(src, blockers) & filter & RAYS_INFINITE[king][src];
            for dest in attacks & !blockers {
                res.push(Move::new(src, dest, MoveFlags::NormalQuiet));
            }
            for dest in attacks & blockers {
                res.push(Move::new(src, dest, MoveFlags::NormalCapture));
            }
        }
    }

    fn gen_pawns<const WHITE: bool>(&self, pinned: Bitboard, check_ray: Bitboard, res: &mut MoveList) {
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
            for dest in pushed & quiets & !promo_rank {
                let src = dest.add(offsets[0]);
                res.push(Move::new(src, dest, MoveFlags::NormalQuiet));
            }
            for dest in Bitboard::rank(double_push_rank) & push(pushed & !blockers) & quiets {
                let src = dest.add(offsets[1]);
                res.push(Move::new(src, dest, MoveFlags::DoublePawnPush));
            }
            for dest in captures_west & !promo_rank {
                let src = dest.add(offsets[2]);
                res.push(Move::new(src, dest, MoveFlags::NormalCapture));
            }
            for dest in captures_east & !promo_rank {
                let src = dest.add(offsets[3]);
                res.push(Move::new(src, dest, MoveFlags::NormalCapture));
            }

            for dest in pushed & quiets & promo_rank {
                let src = dest.add(offsets[0]);
                for p in [PromoKnight, PromoBishop, PromoRook, PromoQueen] {
                    res.push(Move::new(src, dest, p));
                }
            }
            for dest in captures_west & promo_rank {
                let src = dest.add(offsets[2]);
                for p in [CaptPromoKnight, CaptPromoBishop, CaptPromoRook, CaptPromoQueen] {
                    res.push(Move::new(src, dest, p));
                }
            }
            for dest in captures_east & promo_rank {
                let src = dest.add(offsets[3]);
                for p in [CaptPromoKnight, CaptPromoBishop, CaptPromoRook, CaptPromoQueen] {
                    res.push(Move::new(src, dest, p));
                }
            }

            // if we're in check while the ep square is set, it must be from the capturable pawn
            if let Some(ep) = self.ep {
                let captured = ep.add(if WHITE { -8 } else { 8 });
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
                            res.push(Move::new(src, ep, EnPassant));
                        }
                    }
                }
            }
        }
    }

    pub fn attacks_of(&self, sq: Square) -> Bitboard {
        let piece = self.piece_on(sq).unwrap();
        let us = self.stm;
        let blockers = self.occupied_bb();
        match piece.piece_type() {
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
        for knight in self.col_piece_bb(Knight, us) {
            res |= KNIGHTS[knight];
        }
        // TODO: Setwise slider attacks
        // TODO: Mask pieces that can't influence move legality
        let blockers = self.occupied_bb() ^ self.col_piece_bb(King, !us);
        for bishop in self.col_piece_bb(Bishop, us) {
            res |= bishop_attacks(bishop, blockers);
        }
        for rook in self.col_piece_bb(Rook, us) {
            res |= rook_attacks(rook, blockers);
        }
        for queen in self.col_piece_bb(Queen, us) {
            res |= queen_attacks(queen, blockers);
        }
        res
    }

    fn calc_checkers_and_pinned(&self, king: Square, us: Color) -> (Bitboard, Bitboard) {
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
            match (ray & blockers).0.count_ones() {
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
