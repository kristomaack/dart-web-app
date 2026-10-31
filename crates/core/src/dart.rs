use serde::{Deserialize, Serialize};
use std::fmt;

/// Where a dart landed on the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Segment {
    /// Numbers 1..=20.
    Num(u8),
    /// Outer bull (single, 25) or bullseye (double, 50).
    Bull,
    Miss,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mult {
    #[default]
    Single,
    Double,
    Triple,
}

impl Mult {
    pub fn factor(self) -> u16 {
        match self {
            Mult::Single => 1,
            Mult::Double => 2,
            Mult::Triple => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dart {
    pub segment: Segment,
    pub mult: Mult,
}

impl Dart {
    /// Builds a dart, rejecting combinations that don't exist on a board
    /// (numbers outside 1..=20, triple bull). A miss is always single.
    pub fn new(segment: Segment, mult: Mult) -> Option<Self> {
        match (segment, mult) {
            (Segment::Num(n), _) if !(1..=20).contains(&n) => None,
            (Segment::Bull, Mult::Triple) => None,
            (Segment::Miss, _) => Some(Self::MISS),
            _ => Some(Self { segment, mult }),
        }
    }

    pub const MISS: Dart = Dart { segment: Segment::Miss, mult: Mult::Single };

    pub fn single(n: u8) -> Self {
        Self { segment: Segment::Num(n), mult: Mult::Single }
    }
    pub fn double(n: u8) -> Self {
        Self { segment: Segment::Num(n), mult: Mult::Double }
    }
    pub fn triple(n: u8) -> Self {
        Self { segment: Segment::Num(n), mult: Mult::Triple }
    }
    pub fn outer_bull() -> Self {
        Self { segment: Segment::Bull, mult: Mult::Single }
    }
    pub fn bullseye() -> Self {
        Self { segment: Segment::Bull, mult: Mult::Double }
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.segment, self.mult) == Some(*self)
    }

    pub fn points(&self) -> u16 {
        match self.segment {
            Segment::Num(n) => n as u16 * self.mult.factor(),
            Segment::Bull => 25 * self.mult.factor(),
            Segment::Miss => 0,
        }
    }

    /// Doubles include the bullseye.
    pub fn is_double(&self) -> bool {
        self.mult == Mult::Double && self.segment != Segment::Miss
    }
}

impl fmt::Display for Dart {
    /// Board notation: `T20`, `D16`, `7`, `25`, `Bull`, `Miss`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.segment, self.mult) {
            (Segment::Miss, _) => f.write_str("Miss"),
            (Segment::Bull, Mult::Double) => f.write_str("Bull"),
            (Segment::Bull, _) => f.write_str("25"),
            (Segment::Num(n), Mult::Single) => write!(f, "{n}"),
            (Segment::Num(n), Mult::Double) => write!(f, "D{n}"),
            (Segment::Num(n), Mult::Triple) => write!(f, "T{n}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_and_validity() {
        assert_eq!(Dart::triple(20).points(), 60);
        assert_eq!(Dart::bullseye().points(), 50);
        assert_eq!(Dart::outer_bull().points(), 25);
        assert!(Dart::bullseye().is_double());
        assert_eq!(Dart::new(Segment::Bull, Mult::Triple), None);
        assert_eq!(Dart::new(Segment::Num(21), Mult::Single), None);
        assert_eq!(Dart::new(Segment::Miss, Mult::Triple), Some(Dart::MISS));
        assert_eq!(Dart::triple(19).to_string(), "T19");
    }
}
