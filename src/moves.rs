use crate::attacks::RAYS_EXCLUSIVE;
use crate::core::{ColPiece, Color, PieceType, Res, Square};
use crate::pos::Position;
use arrayvec::ArrayVec;
use std::fmt::Display;
use std::mem;
use std::str::FromStr;

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum MoveFlags {
    #[default]
    PromoKnight = 1,
    PromoBishop,
    PromoRook,
    PromoQueen,
    Normal = 8,
    DoublePawnPush,
    CastleQueenside,
    CastleKingside,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Move {
    src: Square,
    dest: Square,
    flags: MoveFlags,
}

impl Move {
    pub fn new(src: Square, dest: Square, flags: MoveFlags) -> Self {
        Self { src, dest, flags }
    }

    fn src_sq(self) -> Square {
        self.src
    }

    fn dest_sq(self) -> Square {
        self.dest
    }

    fn is_double_pawn_push(self) -> bool {
        self.flags() == MoveFlags::DoublePawnPush
    }

    fn promo_piece(self) -> Option<PieceType> {
        if (self.flags() as u8) >= MoveFlags::Normal as u8 {
            None
        } else {
            unsafe { Some(mem::transmute(self.flags)) }
        }
    }

    fn is_castle(self) -> bool {
        self.flags as u8 >= MoveFlags::CastleQueenside as u8
    }

    fn flags(self) -> MoveFlags {
        self.flags
    }

    pub fn parse(s: &str, pos: &Position) -> Res<Self> {
        if !s.is_ascii() {
            return Err(format!("Move '{s:?}' is not ASCII"));
        }
        let len = s.len();
        if ![4, 5].contains(&len) {
            return Err(format!(
                "Move '{s}' has incorrect length, must be 4 or 5 characters"
            ));
        }
        let src = Square::from_str(&s[..2])?;
        let dest = Square::from_str(&s[2..4])?;
        let Some(piece) = pos.piece_on(src) else {
            return Err(format!(
                "No piece on start square {src} of move '{s}' in {pos}"
            ));
        };
        let flags = if len == 5 {
            match s.chars().last().unwrap() {
                'n' => MoveFlags::PromoKnight,
                'b' => MoveFlags::PromoBishop,
                'r' => MoveFlags::PromoRook,
                'q' => MoveFlags::PromoQueen,
                x => return Err(format!("Invalid promotion piece type '{x}'")),
            }
        } else if piece.piece_type() == PieceType::King {
            if pos
                .piece_on(dest)
                .is_some_and(|p| p.color() == piece.color())
                || (dest.file() as usize).abs_diff(src.file() as usize) > 1
            {
                if dest.file() < src.file() {
                    MoveFlags::CastleQueenside
                } else {
                    MoveFlags::CastleKingside
                }
            } else {
                MoveFlags::Normal
            }
        } else if piece.piece_type() == PieceType::Pawn
            && (dest.rank() as usize).abs_diff(src.rank() as usize) > 1
        {
            MoveFlags::DoublePawnPush
        } else {
            MoveFlags::Normal
        };
        Ok(Move::new(src, dest, flags))
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}{}", self.src_sq(), self.dest_sq())?;
        if let Some(promo) = self.promo_piece() {
            write!(f, "{}", promo.letter())?;
        }
        Ok(())
    }
}

// 271 is the maximum number of legal moves in a (possibly not reachable) position, see
// <https://lichess.org/@/Tobs40/blog/why-a-reachable-position-can-have-at-most-218-playable-moves/a5xdxeqs>
pub const MAX_MOVES: usize = 271;

pub type MoveList = ArrayVec<Move, MAX_MOVES>;

impl Position {
    pub fn is_probably_legal(&self, m: Move) -> bool {
        let src = m.src;
        let Some(piece) = self.piece_on(src) else {
            return false;
        };
        if piece.color() != self.stm {
            return false;
        }
        if m.promo_piece().is_some()
            && (piece.piece_type() != PieceType::Pawn || ![0, 7].contains(&(m.dest.rank() as u8)))
        {
            return false;
        } else if m.is_castle() {
            let rook = if m.flags == MoveFlags::CastleQueenside {
                self.castling[self.stm][0]
            } else {
                self.castling[self.stm][1]
            };
            let Some(rook) = rook else { return false };
            if piece.piece_type() != PieceType::King
                || self.piece_on(rook) != Some(ColPiece::new(self.stm, PieceType::Rook))
            {
                return false;
            }
            if RAYS_EXCLUSIVE[src][m.dest].intersects(self.occupied_bb() & !rook.bb()) {
                return false;
            }
            return true;
        }
        self.attacks_of(src).has(m.dest)
    }

    pub fn make_move(mut self, m: Move) -> Self {
        debug_assert!(self.is_probably_legal(m), "{m} {self}");
        let src = m.src;
        let dest = m.dest;
        let us = self.stm;
        let piece = self.piece_on(src).unwrap();
        let ep = self.ep;
        self.stm = !self.stm;
        self.ep = None;
        if self.piece_on(dest).is_some() {
            self.pieces.remove_piece(dest);
        }
        self.pieces.move_piece(src, dest);
        match piece.piece_type() {
            PieceType::Pawn => {
                if m.is_double_pawn_push() {
                    let bb = dest.bb();
                    // only done pseudo-legally here
                    if (bb.east() | bb.west()).intersects(self.col_piece_bb(PieceType::Pawn, !us)) {
                        self.ep = Some(((dest as u8 + src as u8) / 2).try_into().unwrap());
                    }
                } else if let Some(promo) = m.promo_piece() {
                    self.pieces.pieces[piece.piece_type()] ^= dest.bb();
                    self.pieces.place(dest, promo, piece.color());
                } else if Some(dest) == ep {
                    let captured = unsafe { dest.add(if us == Color::White { -8 } else { 8 }) };
                    self.pieces.remove_piece(captured);
                }
            }
            PieceType::King => {
                if m.is_castle() {
                    let rook_dests = Self::CASTLE_ROOK_DESTS[us];
                    let king_dests = Self::CASTLE_KING_DESTS[us];
                    let (rook_dest, king_dest) = match m.flags {
                        MoveFlags::CastleQueenside => (rook_dests[0], king_dests[0]),
                        MoveFlags::CastleKingside => (rook_dests[1], king_dests[1]),
                        _ => unreachable!(),
                    };
                    self.pieces.remove_piece(dest);
                    self.pieces.place(rook_dest, PieceType::Rook, us);
                    self.pieces.place(king_dest, PieceType::King, us);
                }
                self.castling[us][0] = None;
                self.castling[us][1] = None;
            }
            PieceType::Rook => {
                if Some(src) == self.castling[us][0] {
                    self.castling[us][0] = None;
                } else if Some(src) == self.castling[us][1] {
                    self.castling[us][1] = None;
                }
            }
            _ => {}
        }
        if Some(dest) == self.castling[!us][0] {
            self.castling[!us][0] = None;
        } else if Some(dest) == self.castling[!us][1] {
            self.castling[!us][1] = None;
        }
        self
    }
}
