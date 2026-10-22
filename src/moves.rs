use crate::core::{ColPiece, Color, PieceType, Res, Square};
use crate::movegen::RAYS_EXCLUSIVE;
use crate::pos::Position;
use arrayvec::ArrayVec;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum MoveFlags {
    #[default]
    NormalQuiet,
    DoublePawnPush,
    CastleQueenside,
    CastleKingside,
    PromoKnight,
    PromoBishop,
    PromoRook,
    PromoQueen,
    NormalCapture,
    EnPassant,
    CaptPromoKnight = 12,
    CaptPromoBishop,
    CaptPromoRook,
    CaptPromoQueen,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Move {
    src: Square,
    dest: Square,
    flags: MoveFlags,
}

impl Move {
    pub fn new(src: Square, dest: Square, flags: MoveFlags) -> Self {
        Self {
            src,
            dest,
            flags,
        }
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

    fn is_capture(self) -> bool {
        self.flags as u8 & 0x8 != 0
    }

    pub(crate) fn is_ep(self) -> bool {
        self.flags() == MoveFlags::EnPassant
    }

    fn is_promotion(self) -> bool {
        self.flags as u8 & 0x4 != 0
    }

    fn promo_piece(self) -> PieceType {
        debug_assert!(self.is_promotion(), "{self:?}");

        (((self.flags as u8) & 0x3) + PieceType::Knight as u8).try_into().unwrap()
    }

    fn is_castle(self) -> bool {
        self.flags as u8 & 0xE == 0x2
    }

    fn flags(self) -> MoveFlags {
        self.flags
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}{}", self.src_sq(), self.dest_sq())?;
        if self.is_promotion() {
            write!(f, "{}", self.promo_piece().letter())?;
        }
        Ok(())
    }
}

impl FromStr for Move {
    type Err = String;

    fn from_str(s: &str) -> Res<Self> {
        if !s.is_ascii() {
            return Err(format!("Move '{s:?}' is not ASCII"));
        }
        let len = s.len();
        if ![4, 5].contains(&len) {
            return Err(format!("Move '{s}' has incorrect length, must be 4 or 5 characters"));
        }
        let src = Square::from_str(&s[..2])?;
        let dest = Square::from_str(&s[2..4])?;
        let flags = if len == 5 {
            match s.chars().last().unwrap() {
                'n' => MoveFlags::PromoKnight,
                'b' => MoveFlags::PromoBishop,
                'r' => MoveFlags::PromoRook,
                'q' => MoveFlags::PromoQueen,
                x => return Err(format!("Invalid piece type '{x}'")),
            }
        } else {
            MoveFlags::NormalQuiet
        };
        Ok(Move::new(src, dest, flags))
    }
}


// 271 is the maximum number of legal moves in a (possibly not reachable) position, see
// <https://lichess.org/@/Tobs40/blog/why-a-reachable-position-can-have-at-most-218-playable-moves/a5xdxeqs>
pub const MAX_MOVES: usize = 271;

pub type MoveList = ArrayVec<Move, MAX_MOVES>;

impl Position {
    pub fn is_pseudolegal(&self, m: Move) -> bool {
        let src = m.src;
        let Some(piece) = self.piece_on(src) else { return false };
        if piece.color() != self.stm {
            return false;
        }
        if m.is_promotion() && (piece.piece_type() != PieceType::Pawn || ![0, 7].contains(&(m.dest.rank() as u8))) {
            return false;
        } else if m.is_ep() && self.piece_on(m.dest).is_some() {
            return false;
        } else if m.is_castle() {
            let rook = if m.flags == MoveFlags::CastleQueenside {
                self.castling[self.stm][0]
            } else {
                self.castling[self.stm][1]
            };
            let Some(rook) = rook else { return false };
            if piece.piece_type() != PieceType::King || self.piece_on(rook) != Some(ColPiece::new(self.stm, PieceType::Rook)) {
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
        debug_assert!(self.is_pseudolegal(m), "{m} {self}");
        let src = m.src;
        let dest = m.dest;
        let us = self.stm;
        let piece = self.piece_on(src).unwrap();
        let ep = self.ep;
        self.stm = !self.stm;
        self.ep = None;
        self.pieces.remove_piece(src);
        if self.piece_on(dest).is_some() {
            self.pieces.remove_piece(dest);
        }
        self.pieces.place_piece(dest, piece.piece_type(), piece.color());
        match piece.piece_type() {
            PieceType::Pawn => {
                if m.is_double_pawn_push() {
                    let bb = dest.bb();
                    // only done pseudo-legally here
                    if (bb.east() | bb.west()).intersects(self.col_piece_bb(PieceType::Pawn, !us)) {
                        self.ep = Some(((dest as u8 + src as u8) / 2).try_into().unwrap());
                    }
                } else if m.is_promotion() {
                    let piece_type = m.promo_piece();
                    self.pieces.pieces[piece.piece_type()] ^= dest.bb();
                    self.pieces.place_piece(dest, piece_type, piece.color());
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
                    self.pieces.place_piece(rook_dest, PieceType::Rook, us);
                    self.pieces.place_piece(king_dest, PieceType::King, us);
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