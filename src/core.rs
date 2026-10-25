use derive_more::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr, ShrAssign};
use std::fmt::{Display, Formatter};
use std::mem;
use std::ops::{Index, IndexMut, Not};
use std::str::FromStr;

pub type Res<T> = std::result::Result<T, String>;

pub const NUM_PLAYERS: usize = 2;
pub const NUM_PIECES: usize = 6;

pub const NUM_SQUARES: usize = 64;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceType {
    pub fn letter(self) -> char {
        match self {
            PieceType::Pawn => 'p',
            PieceType::Knight => 'n',
            PieceType::Bishop => 'b',
            PieceType::Rook => 'r',
            PieceType::Queen => 'q',
            PieceType::King => 'k',
        }
    }
}

impl TryFrom<u8> for PieceType {
    type Error = String;

    fn try_from(value: u8) -> Res<Self> {
        if value >= 6 {
            Err(format!("Invalid piece type: {}", value))
        } else {
            unsafe { Ok(mem::transmute(value)) }
        }
    }
}

impl<T> Index<PieceType> for [T; NUM_PIECES] {
    type Output = T;
    fn index(&self, index: PieceType) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<PieceType> for [T; NUM_PIECES] {
    fn index_mut(&mut self, index: PieceType) -> &mut T {
        &mut self[index as usize]
    }
}


#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn iter() -> impl Iterator<Item=Self> {
        [Self::White, Self::Black].into_iter()
    }
}

impl Not for Color {
    type Output = Self;
    fn not(self) -> Self::Output {
        // TODO: Make sure this gets code genned to 1 - self as usize
        match self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }
}
impl<T> Index<Color> for [T; NUM_PLAYERS] {
    type Output = T;
    fn index(&self, index: Color) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<Color> for [T; NUM_PLAYERS] {
    fn index_mut(&mut self, index: Color) -> &mut T {
        &mut self[index as usize]
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Color::White => write!(f, "white"),
            Color::Black => write!(f, "black"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
#[allow(dead_code)]
pub enum ColPiece {
    WhitePawn,
    WhiteKnight,
    WhiteBishop,
    WhiteRook,
    WhiteQueen,
    WhiteKing,
    BlackPawn = 8,
    BlackKnight,
    BlackBishop,
    BlackRook,
    BlackQueen,
    BlackKing,
}

impl ColPiece {
    pub fn new(color: Color, piece_type: PieceType) -> Self {
        unsafe { mem::transmute(color as u8 * 8 + piece_type as u8) }
    }

    pub fn color(self) -> Color {
        unsafe { mem::transmute(self as u8 / 8) }
    }

    pub fn piece_type(self) -> PieceType {
        unsafe { mem::transmute(self as u8 % 8) }
    }
}


#[rustfmt::skip]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(usize)]
#[allow(dead_code)]
pub enum Rank {
    R1, R2, R3, R4, R5, R6, R7, R8,
}

impl Display for Rank {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Rank::R1 => write!(f, "1"),
            Rank::R2 => write!(f, "2"),
            Rank::R3 => write!(f, "3"),
            Rank::R4 => write!(f, "4"),
            Rank::R5 => write!(f, "5"),
            Rank::R6 => write!(f, "6"),
            Rank::R7 => write!(f, "7"),
            Rank::R8 => write!(f, "8"),
        }
    }
}

impl From<usize> for Rank {
    fn from(f: usize) -> Self {
        if f >= 8 {
            panic!("{f} is not a valid rank");
        }
        unsafe { mem::transmute(f) }
    }
}
impl TryFrom<char> for Rank {
    type Error = String;
    fn try_from(c: char) -> Res<Self> {
        match c {
            '1'..='8' => Ok(Self::from(c as usize - '1' as usize)),
            _ => Err(format!("Invalid rank '{c}'")),
        }
    }
}

#[rustfmt::skip]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[repr(usize)]
#[allow(dead_code)]
pub enum File {
    A, B, C, D, E, F, G, H,
}

impl Display for File {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            File::A => write!(f, "a"),
            File::B => write!(f, "b"),
            File::C => write!(f, "c"),
            File::D => write!(f, "d"),
            File::E => write!(f, "e"),
            File::F => write!(f, "f"),
            File::G => write!(f, "g"),
            File::H => write!(f, "h"),
        }
    }
}

impl From<usize> for File {
    fn from(f: usize) -> Self {
        if f >= 8 {
            panic!("{f} is not a valid file");
        }
        unsafe { mem::transmute(f) }
    }
}

impl TryFrom<char> for File {
    type Error = String;

    fn try_from(c: char) -> Res<Self> {
        let c = c.to_ascii_lowercase();
        match c {
            'a'..='h' => Ok(Self::from(c as usize - 'a' as usize)),
            _ => Err(format!("Invalid file '{c}'")),
        }
    }
}


#[rustfmt::skip]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
#[allow(dead_code)]
pub enum Square {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
}

impl TryFrom<u8> for Square {
    type Error = String;

    fn try_from(sq: u8) -> Res<Self> {
        if sq >= 64 {
            Err(format!("{sq} is not a valid Square"))
        } else {
            unsafe {
                Ok(mem::transmute(sq))
            }
        }
    }
}

impl FromStr for Square {
    type Err = String;

    fn from_str(input: &str) -> Res<Self> {
        if input.len() != 2 {
            return Err(format!("Invalid square '{input}'"));
        }
        let file = input.chars().nth(0).unwrap().try_into()?;
        let rank = input.chars().nth(1).unwrap().try_into()?;
        Ok(Self::from_rank_file(rank, file))
    }
}

impl Square {
    pub fn from_rank_file(rank: Rank, file: File) -> Self {
        Self::try_from(rank as u8 * 8 + file as u8).unwrap()
    }

    // try_from isn't const
    pub const fn new(square: usize) -> Self {
        assert!(square < NUM_SQUARES);
        unsafe { mem::transmute(square as u8) }
    }

    pub fn idx(self) -> u8 {
        self as u8
    }

    pub unsafe fn add(self, delta: i8) -> Self {
        let val = self as i8 + delta;
        debug_assert!(val >= 0 && val <= 64);
        unsafe {
            mem::transmute(val)
        }
    }

    pub fn iter() -> impl Iterator<Item=Self> {
        (0..64).map(|i| Square::new(i))
    }

    pub fn bb(self) -> Bitboard {
        Bitboard::single(self)
    }

    pub fn flip(self) -> Square {
        Self::try_from(self as u8 ^ 0b111_000).unwrap()
    }

    pub const fn file(self) -> File {
        unsafe {
            mem::transmute(self as usize & 0b111)
        }
    }

    pub const fn rank(self) -> Rank {
        unsafe {
            mem::transmute((self as usize >> 3) & 0b111)
        }
    }
}

impl Display for Square {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}{1}", self.file(), self.rank())
    }
}

impl<T> Index<Square> for [T; 64] {
    type Output = T;
    fn index(&self, index: Square) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<Square> for [T; 64] {
    fn index_mut(&mut self, index: Square) -> &mut T {
        &mut self[index as usize]
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    BitOr,
    BitOrAssign,
    BitAnd,
    BitAndAssign,
    BitXor,
    BitXorAssign,
    Shl,
    ShlAssign,
    Shr,
    ShrAssign,
    Not,
)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const A_FILE: Self = Self(0x0101_0101_0101_0101);
    pub const H_FILE: Self = Self(0x8080_8080_8080_8080);
    pub const RANK_1: Self = Self(0xFF);
    pub const RANK_8: Self = Self(0xFF00_0000_0000_0000);
    pub const BACKRANKS: Self = Self(Self::RANK_1.0 | Self::RANK_8.0);
    pub const EDGES: Self = Self(Self::BACKRANKS.0 | Self::A_FILE.0 | Self::H_FILE.0);

    pub const fn single(sq: Square) -> Self {
        Self(1 << sq as usize)
    }

    pub fn file(file: File) -> Self {
        Self::A_FILE << (file as usize)
    }

    pub fn rank(rank: Rank) -> Self {
        Self::RANK_1 << (8 * rank as usize)
    }

    pub const fn west(self) -> Self {
        Self((self.0 & !Self::A_FILE.0) >> 1)
    }
    pub const fn east(self) -> Self {
        Self((self.0 & !Self::H_FILE.0) << 1)
    }
    pub const fn north(self) -> Self {
        Self(self.0 << 8)
    }
    pub const fn south(self) -> Self {
        Self(self.0 >> 8)
    }

    pub fn pawn_push(self, color: Color) -> Self {
        match color {
            Color::White => self.north(),
            Color::Black => self.south(),
        }
    }

    pub fn count_ones(self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn swap_bytes(self) -> Self {
        Self(self.0.swap_bytes())
    }

    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub fn has(self, sq: Square) -> bool {
        self.0 >> (sq as usize) & 1 != 0
    }

    pub fn pop_lsb(&mut self) -> u8 {
        let shift = self.0.trailing_zeros();
        *self &= Self(self.0.wrapping_sub(1));
        shift as u8
    }

    pub fn sq(self) -> Square {
        debug_assert!(self.count_ones() == 1);
        (self.0.trailing_zeros() as u8).try_into().unwrap()
    }
}

impl Iterator for Bitboard {
    type Item = Square;

    fn next(&mut self) -> Option<Square> {
        if self.0 == 0 { None } else { Some(self.pop_lsb().try_into().unwrap()) }
    }
}

impl Display for Bitboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CastlingRight {
    Queenside,
    Kingside,
}
