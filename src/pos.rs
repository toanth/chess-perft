use crate::core::CastlingRight::*;
use crate::core::Color::*;
use crate::core::PieceType::*;
use crate::core::{Bitboard, ColPiece, Color, File, PieceType, Square, NUM_PIECES, NUM_PLAYERS};
use crate::core::{Rank, Res};
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct BoardPieces {
    pub colors: [Bitboard; NUM_PLAYERS],
    pub pieces: [Bitboard; NUM_PIECES],
    pub mailbox: [Option<ColPiece>; 64],
}

// can't be derived because 64 is too large
impl Default for BoardPieces {
    fn default() -> Self {
        Self {
            colors: [Bitboard(0); NUM_PLAYERS],
            pieces: [Bitboard(0); NUM_PIECES],
            mailbox: [None; 64],
        }
    }
}

impl BoardPieces {
    pub fn place_piece(&mut self, sq: Square, piece: PieceType, color: Color) {
        self.mailbox[sq] = Some(ColPiece::new(color, piece));
        let bb = Bitboard::single(sq);
        self.colors[color] |= bb;
        self.pieces[piece] |= bb;
    }

    pub fn remove_piece(&mut self, sq: Square) {
        let piece = self.mailbox[sq].unwrap();
        self.mailbox[sq] = None;
        let bb = !Bitboard::single(sq);
        self.colors[piece.color()] &= bb;
        self.pieces[piece.piece_type()] &= bb;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Position {
    pub pieces: BoardPieces,
    pub castling: [[Option<Square>; 2]; NUM_PLAYERS],
    // Can be incorrectly set in case the ep pawn is pinned; movegen checks for this
    pub ep: Option<Square>,
    pub stm: Color,
}

impl Position {
    pub const STARTPOS: &'static str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    pub fn new() -> Position {
        Self::from_fen(Self::STARTPOS).unwrap()
    }

    pub fn from_fen(fen: &str) -> Res<Position> {
        if !fen.is_ascii() {
            return Err(format!("FEN '{fen}' contains a non-ASCII character"));
        }
        let mut fen = fen.split_ascii_whitespace();
        let board = fen.next().ok_or("Empty FEN")?;
        if board == "startpos" {
            return Ok(Self::new());
        }
        let mut pieces = BoardPieces::default();
        let mut rank = 7;
        let mut file = 0;
        for c in board.chars() {
            if c == '/' {
                if file != 8 { return Err(format!("Invalid row length '{}' in first FEN part", file + 1)); }
                if rank == 0 { return Err("Too many rows in first FEN part".into()); }
                rank -= 1;
                file = 0;
            } else if c.is_digit(9) && c != '0' {
                file += c as u8 - '0' as u8;
            } else {
                let sq = Square::try_from(rank * 8 + file)?;
                let color = if c.is_ascii_uppercase() { White } else { Black };
                match c.to_ascii_lowercase() {
                    'p' => pieces.place_piece(sq, Pawn, color),
                    'n' => pieces.place_piece(sq, Knight, color),
                    'b' => pieces.place_piece(sq, Bishop, color),
                    'r' => pieces.place_piece(sq, Rook, color),
                    'q' => pieces.place_piece(sq, Queen, color),
                    'k' => pieces.place_piece(sq, King, color),
                    _ => return Err(format!("Invalid character '{c}' in first FEN part")),
                }
                file += 1;
            }
        }
        let stm = match fen.next() {
            Some("w") => White,
            Some("b") => Black,
            Some(x) => return Err(format!("Invalid stm '{x}' in FEN")),
            None => return Err("FEN ends after first part, missing side to move".into()),
        };

        let castling_str = fen.next().ok_or("Missing castling part in FEN")?;
        let mut castling = [[None; 2]; NUM_PLAYERS];
        if castling_str != "-" {
            for c in castling_str.chars() {
                let color = if c.is_ascii_uppercase() { White } else { Black };
                let rank = if color == White { Rank::R1 } else { Rank::R8 };
                let king = (pieces.pieces[King] & pieces.colors[color]).sq();
                let king_file = king.file() as u8;
                let rook = ColPiece::new(color, Rook);
                match c.to_ascii_lowercase() {
                    'q' => {
                        let Some(rook) = (0..king_file).find(|i| pieces.mailbox[(i + 8 * rank as u8) as usize] == Some(rook)) else {
                            return Err(format!("Missing {color} rook to castle with queenside"));
                        };
                        castling[color][Queenside as usize] = Some(Square::try_from(rook + 8 * rank as u8)?);
                    }
                    'k' => {
                        let Some(rook) = (king_file + 1..8).rev().find(|i| pieces.mailbox[(i + 8 * rank as u8) as usize] == Some(rook)) else {
                            return Err(format!("Missing {color} rook to castle with kingside"));
                        };
                        castling[color][Kingside as usize] = Some(Square::try_from(rook + 8 * rank as u8)?);
                    }
                    f @ 'a'..='h' => {
                        let file = File::try_from(f)?;
                        let side = (file > king.file()) as usize;
                        let sq = Square::from_rank_file(rank, file);
                        if pieces.mailbox[sq] != Some(rook) {
                            return Err(format!("There is no {color} rook on {sq} to castle with"));
                        }
                        castling[color][side] = Some(sq);
                    }
                    _ => return Err(format!("Invalid castling right '{c}' in FEN")),
                }
            }
        }
        let ep_str = fen.next().ok_or("Missing ep part in FEN")?;
        let ep = if ep_str == "-" { None } else { Some(ep_str.parse()?) };
        Ok(Self { pieces, castling, ep, stm })
    }

    pub fn piece_on(&self, sq: Square) -> Option<ColPiece> {
        self.pieces.mailbox[sq]
    }

    pub fn king_sq(&self, color: Color) -> Square {
        self.col_piece_bb(King, color).sq()
    }

    pub fn col_piece_bb(&self, piece: PieceType, color: Color) -> Bitboard {
        self.pieces.pieces[piece] & self.pieces.colors[color]
    }

    pub fn piece_bb(&self, piece: PieceType) -> Bitboard {
        self.pieces.pieces[piece]
    }

    pub fn col_bb(&self, color: Color) -> Bitboard {
        self.pieces.colors[color]
    }

    pub fn occupied_bb(&self) -> Bitboard {
        self.pieces.colors[White] | self.pieces.colors[Black]
    }

    pub const CASTLE_ROOK_DESTS: [[Square; 2]; NUM_PLAYERS] = [[Square::D1, Square::F1], [Square::D8, Square::F8]];

    pub const CASTLE_KING_DESTS: [[Square; 2]; NUM_PLAYERS] = [[Square::C1, Square::G1], [Square::C8, Square::G8]];
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for rank in (0..8).rev() {
            let mut skip = 0;
            for file in 0..8 {
                let sq = Square::try_from(rank * 8 + file).unwrap();
                let Some(p) = self.piece_on(sq) else {
                    skip += 1;
                    continue;
                };
                let mut c = p.piece_type().letter();
                if p.color() == White {
                    c = c.to_ascii_uppercase();
                }
                if skip > 0 {
                    write!(f, "{skip}")?;
                }
                write!(f, "{c}")?;
                skip = 0;
            }
            if skip > 0 {
                write!(f, "{skip}")?;
            }
            if rank != 0 {
                write!(f, "/")?;
            }
        }
        match self.stm {
            White => write!(f, " w ")?,
            Black => write!(f, " b ")?,
        }
        if self.castling.iter().flatten().all(|e| e.is_none()) {
            write!(f, "-")?;
        } else {
            for (i, col) in self.castling.iter().enumerate() {
                for side in col.iter() {
                    if let Some(sq) = side {
                        let mut s = sq.file().to_string();
                        if i == White as usize {
                            s = s.to_ascii_uppercase();
                        }
                        write!(f, "{s}")?;
                    }
                }
            }
        }
        if let Some(ep) = self.ep {
            write!(f, " {ep}")?
        } else {
            write!(f, " -")?
        }
        // we don't keep track of a half-move counter or a fullmove number, as they are unnecessary for perft
        write!(f, " 0 1")
    }
}