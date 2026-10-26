use crate::core::CastlingRight::*;
use crate::core::Color::*;
use crate::core::PieceType::*;
use crate::core::{Bitboard, ColPiece, Color, File, NUM_PIECES, NUM_PLAYERS, PieceType, Square};
use crate::core::{Rank, Res};
use std::cmp::Ordering;
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
    pub fn place(&mut self, sq: Square, piece_type: PieceType, color: Color) {
        self.place_piece(sq, ColPiece::new(color, piece_type));
    }

    pub fn place_piece(&mut self, sq: Square, piece: ColPiece) {
        self.mailbox[sq] = Some(piece);
        let bb = Bitboard::single(sq);
        self.colors[piece.color()] |= bb;
        self.pieces[piece.piece_type()] |= bb;
    }

    pub fn remove_piece(&mut self, sq: Square) {
        let piece = self.mailbox[sq].unwrap();
        self.mailbox[sq] = None;
        let bb = !Bitboard::single(sq);
        self.colors[piece.color()] &= bb;
        self.pieces[piece.piece_type()] &= bb;
    }

    pub fn move_piece(&mut self, src: Square, dest: Square) {
        let piece = self.mailbox[src];
        self.mailbox[src] = None;
        self.mailbox[dest] = piece;
        let piece = piece.unwrap();
        let bb = Bitboard::single(src) ^ Bitboard::single(dest);
        self.colors[piece.color()] ^= bb;
        self.pieces[piece.piece_type()] ^= bb;
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
            return Self::from_fen(Self::STARTPOS);
        } else if board == "kiwipete" {
            return Self::from_fen(
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            );
        }
        let mut pieces = BoardPieces::default();
        let mut rank = 7;
        let mut file = 0;
        for c in board.chars() {
            if c == '/' {
                if file != 8 {
                    return Err(format!(
                        "Invalid row length '{}' in first FEN part",
                        file + 1
                    ));
                }
                if rank == 0 {
                    return Err("Too many rows in first FEN part".into());
                }
                rank -= 1;
                file = 0;
            } else if c.is_digit(9) && c != '0' {
                file += c as u8 - '0' as u8;
            } else {
                let sq = Square::try_from(rank * 8 + file)?;
                let color = if c.is_ascii_uppercase() { White } else { Black };
                match c.to_ascii_lowercase() {
                    'p' => pieces.place(sq, Pawn, color),
                    'n' => pieces.place(sq, Knight, color),
                    'b' => pieces.place(sq, Bishop, color),
                    'r' => pieces.place(sq, Rook, color),
                    'q' => pieces.place(sq, Queen, color),
                    'k' => pieces.place(sq, King, color),
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
                        let Some(rook) = (0..king_file)
                            .find(|i| pieces.mailbox[(i + 8 * rank as u8) as usize] == Some(rook))
                        else {
                            return Err(format!("Missing {color} rook to castle with queenside"));
                        };
                        castling[color][Queenside as usize] =
                            Some(Square::try_from(rook + 8 * rank as u8)?);
                    }
                    'k' => {
                        let Some(rook) = (king_file + 1..8)
                            .rev()
                            .find(|i| pieces.mailbox[(i + 8 * rank as u8) as usize] == Some(rook))
                        else {
                            return Err(format!("Missing {color} rook to castle with kingside"));
                        };
                        castling[color][Kingside as usize] =
                            Some(Square::try_from(rook + 8 * rank as u8)?);
                    }
                    f @ 'a'..='h' => {
                        let file = File::try_from(f)?;
                        let side = (file > king.file()) as usize;
                        let sq = Square::from_rank_file(rank, file);
                        castling[color][side] = Some(sq);
                    }
                    _ => return Err(format!("Invalid castling right '{c}' in FEN")),
                }
            }
        }
        let ep_str = fen.next().ok_or("Missing ep part in FEN")?;
        let ep = if ep_str == "-" {
            None
        } else {
            Some(ep_str.parse()?)
        };
        let res = Self {
            pieces,
            castling,
            ep,
            stm,
        };
        res.check_valid()
    }

    pub fn check_valid(self) -> Res<Self> {
        if self.pieces.colors[0] & self.pieces.colors[1] != Bitboard(0) {
            return Err("Color bitboards overlap".into());
        }
        let mut acc = Bitboard(0);
        for bb in self.pieces.pieces {
            if bb.intersects(acc) {
                return Err("Piece bitboards overlap".into());
            }
            acc |= bb;
        }
        if acc != self.occupied_bb() {
            return Err("Piece bitboards and color bitboards differ".into());
        }
        let mut num_pieces = 0;
        for (i, piece) in self.pieces.mailbox.iter().enumerate() {
            let sq = Square::new(i);
            let Some(piece) = piece else {
                if self.occupied_bb().has(sq) {
                    return Err("Bitboards and mailbox differ".into());
                }
                continue;
            };
            num_pieces += 1;
            if !self.pieces.colors[piece.color()].has(sq)
                || !self.pieces.pieces[piece.piece_type()].has(sq)
            {
                return Err("Bitboards and mailbox differ".into());
            }
        }
        if num_pieces != self.occupied_bb().0.count_ones() {
            return Err("Bitboards and mailbox differ".into());
        }
        if self.pieces.pieces[Pawn].intersects(Bitboard::BACKRANKS) {
            return Err("There is a pawn on a backrank".into());
        }
        for c in Color::iter() {
            for (i, &side) in self.castling[c].iter().enumerate() {
                if let Some(sq) = side {
                    if self.piece_on(sq) != Some(ColPiece::new(c, Rook))
                        || sq.rank() != Rank::try_from(7 * c as usize).unwrap()
                        || sq.file().cmp(&self.king_sq(c).file())
                            != if i == 0 {
                                Ordering::Less
                            } else {
                                Ordering::Greater
                            }
                    {
                        return Err("Incorrect castling flag".into());
                    }
                }
            }
        }
        if self
            .calc_checkers_and_pinned(self.king_sq(!self.stm), !self.stm)
            .0
            != Bitboard(0)
        {
            return Err("The inactive player is in check".into());
        }
        Ok(self)
    }

    pub fn piece_on(&self, sq: Square) -> Option<ColPiece> {
        self.pieces.mailbox[sq]
    }

    pub fn piece_type_on(&self, sq: Square) -> Option<PieceType> {
        self.pieces.mailbox[sq].map(|p| p.piece_type())
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

    pub const CASTLE_ROOK_DESTS: [[Square; 2]; NUM_PLAYERS] =
        [[Square::D1, Square::F1], [Square::D8, Square::F8]];

    pub const CASTLE_KING_DESTS: [[Square; 2]; NUM_PLAYERS] =
        [[Square::C1, Square::G1], [Square::C8, Square::G8]];
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for rank in (0..8).rev() {
            let mut skip = 0;
            for file in 0..8 {
                let sq = Square::new(rank * 8 + file);
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
