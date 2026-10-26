use crate::attacks::{
    ANTI_DIAGONALS, BISHOPS, DIAGONALS, KINGS, KNIGHTS, PAWN_CAPTS, RAYS_EXCLUSIVE, RAYS_INCLUSIVE,
    RAYS_INFINITE, ROOKS, all_knight_attacks, all_slider_attacks, bishop_attacks,
    horizontal_attacks, queen_attacks, rook_attacks,
};
use crate::core::Color::{Black, White};
use crate::core::PieceType::*;
use crate::core::{Bitboard, Color, Rank, Square};
use crate::moves::MoveFlags::{PromoBishop, PromoKnight, PromoQueen, PromoRook};
use crate::moves::{Move, MoveFlags, MoveList};
use crate::pos::Position;

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

    pub fn legal_moves_impl<C: MoveGenCallback>(&self, mut callback: C) {
        let us = self.stm;
        let king = self.king_sq(us);
        let our_pieces = self.pieces.colors[us];
        let (checkers, pinned) = self.calc_checkers_and_pinned(king, us);
        let relevant_attacks = self.calc_relevant_attacks_for(!us);
        callback.gen_attacks(
            KINGS[king] & !our_pieces & !relevant_attacks,
            king,
            MoveFlags::Normal,
        );
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
        Self::gen_sliders(
            bishops,
            pinned,
            blockers,
            filter,
            king,
            bishop_attacks,
            &mut callback,
        );
        let rooks = self.col_piece_bb(Rook, us);
        Self::gen_sliders(
            rooks,
            pinned,
            blockers,
            filter,
            king,
            rook_attacks,
            &mut callback,
        );
        let queens = self.col_piece_bb(Queen, us);
        Self::gen_sliders(
            queens,
            pinned,
            blockers,
            filter,
            king,
            queen_attacks,
            &mut callback,
        );
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
                    && !RAYS_INCLUSIVE[rook][rook_dest].intersects(blockers)
                    && !pinned.has(rook)
                {
                    let flag = if i == 0 {
                        MoveFlags::CastleQueenside
                    } else {
                        MoveFlags::CastleKingside
                    };
                    callback.gen_single(king, rook, flag);
                }
            }
        }
    }

    fn gen_sliders<F>(
        sliders: Bitboard,
        pinned: Bitboard,
        blockers: Bitboard,
        filter: Bitboard,
        king: Square,
        f: F,
        callback: &mut impl MoveGenCallback,
    ) where
        F: Fn(Square, Bitboard) -> Bitboard,
    {
        for src in sliders & !pinned {
            let attacks = f(src, blockers) & filter;
            callback.gen_attacks(attacks, src, MoveFlags::Normal);
        }
        for src in sliders & pinned {
            let attacks = f(src, blockers) & filter & RAYS_INFINITE[king][src];
            callback.gen_attacks(attacks, src, MoveFlags::Normal);
        }
    }

    fn gen_pawns<const WHITE: bool, Callback>(
        &self,
        pinned: Bitboard,
        check_ray: Bitboard,
        callback: &mut impl MoveGenCallback,
    ) {
        debug_assert_eq!(WHITE, self.stm == White);
        let us = if WHITE { White } else { Black };
        let king = self.king_sq(us);
        let blockers = self.occupied_bb();
        let their_pieces = self.pieces.colors[!us];
        let quiets = !blockers & check_ray;
        let captures = their_pieces & check_ray;
        let pawns = self.col_piece_bb(Pawn, us);
        let pushable = pawns & (!pinned | Bitboard::file(king.file()));
        let push = |bb: Bitboard| {
            if WHITE { bb.north() } else { bb.south() }
        };
        let (w, e) = if WHITE {
            (ANTI_DIAGONALS[king], DIAGONALS[king])
        } else {
            (DIAGONALS[king], ANTI_DIAGONALS[king])
        };
        let captures_west = pawns & (!pinned | w);
        let captures_east = pawns & (!pinned | e);
        let captures_west = push(captures_west).west() & captures;
        let captures_east = push(captures_east).east() & captures;
        let double_push_rank = if WHITE { Rank::R4 } else { Rank::R5 };
        let promo_rank = if WHITE { Rank::R8 } else { Rank::R1 };
        let promo_rank = Bitboard::rank(promo_rank);
        let offsets = if WHITE {
            [-8, -16, -7, -9]
        } else {
            [8, 16, 9, 7]
        };
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
                    let possibly_pinning =
                        (self.piece_bb(Rook) | self.piece_bb(Queen)) & self.col_bb(!us);
                    // horizontally "pinned" e.p. pawn: The capturing pawn and victim are blocking a check together.
                    if horizontal_attacks(king, occ_bb).intersects(possibly_pinning) {
                        // Only the moved pawn and the capturing pawn are between the king and a horizontal slider,
                        // so the pawn is effectively pinned for ep.
                        return;
                    }
                }
                for src in can_capture {
                    if !pinned.intersects(src.bb())
                        || RAYS_INFINITE[self.king_sq(us)][src].intersects(ep.bb())
                    {
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
                let double_pushable =
                    single_push & (Bitboard::rank(Rank::R3) | Bitboard::rank(Rank::R6));
                let double_push = double_pushable.pawn_push(us) & !self.occupied_bb();
                Self::pawn_attacks(sq.bb(), us) | single_push | double_push
            }
            Knight => KNIGHTS[sq],
            Bishop => bishop_attacks(sq, blockers),
            Rook => rook_attacks(sq, blockers),
            Queen => queen_attacks(sq, blockers),
            King => KINGS[sq],
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
        let rook_sliders = (self.piece_bb(Rook) | self.piece_bb(Queen)) & self.col_bb(us);
        res |= all_slider_attacks(bishop_sliders, rook_sliders, blockers);
        res
    }

    pub fn calc_checkers_and_pinned(&self, king: Square, us: Color) -> (Bitboard, Bitboard) {
        let blockers = self.occupied_bb();
        let mut checkers = ((PAWN_CAPTS[us][king] & self.piece_bb(Pawn))
            | (KNIGHTS[king] & self.piece_bb(Knight)))
            & self.col_bb(!us);
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
        (checkers, pinned)
    }
}
