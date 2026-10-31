//! Game-mode registry. Adding a mode means: a new `ModeKind` + `ModeConfig`
//! variant, a `Board` variant, and a module implementing its rules. The match
//! arms below make the compiler point out every place that needs to know.

pub mod robinhood;
pub mod x01;

use crate::dart::Dart;
use serde::{Deserialize, Serialize};

pub use robinhood::RobinhoodBoard;
pub use x01::{X01Board, X01Config};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeKind {
    X01,
    RoundRobinhood,
}

impl ModeKind {
    pub const ALL: [ModeKind; 2] = [ModeKind::X01, ModeKind::RoundRobinhood];

    pub fn slug(self) -> &'static str {
        match self {
            ModeKind::X01 => "x01",
            ModeKind::RoundRobinhood => "robinhood",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.slug() == s)
    }

    pub fn title(self) -> &'static str {
        match self {
            ModeKind::X01 => "301 / 501",
            ModeKind::RoundRobinhood => "Round Robinhood",
        }
    }

    pub fn tagline(self) -> &'static str {
        match self {
            ModeKind::X01 => "Count down to zero, first one out wins.",
            ModeKind::RoundRobinhood => "Hit 15–20 and Bull three times each.",
        }
    }

    pub fn default_config(self) -> ModeConfig {
        match self {
            ModeKind::X01 => ModeConfig::X01(X01Config::default()),
            ModeKind::RoundRobinhood => ModeConfig::RoundRobinhood,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModeConfig {
    X01(X01Config),
    RoundRobinhood,
}

/// A single player's position within the current leg.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Board {
    X01(X01Board),
    Robinhood(RobinhoodBoard),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DartOutcome {
    /// The dart changed the board.
    Scored,
    /// Legal dart that changed nothing (miss, closed target, not doubled in).
    NoScore,
    /// The turn is void; the board reverts to how it was at turn start.
    Bust,
    /// The board is complete: checked out / everything closed.
    Finished,
}

impl ModeConfig {
    pub fn kind(&self) -> ModeKind {
        match self {
            ModeConfig::X01(_) => ModeKind::X01,
            ModeConfig::RoundRobinhood => ModeKind::RoundRobinhood,
        }
    }

    /// Short human description, e.g. "501 · Double out".
    pub fn label(&self) -> String {
        match self {
            ModeConfig::X01(c) => c.label(),
            ModeConfig::RoundRobinhood => "Round Robinhood".into(),
        }
    }

    pub fn initial_board(&self) -> Board {
        match self {
            ModeConfig::X01(c) => Board::X01(X01Board::new(c)),
            ModeConfig::RoundRobinhood => Board::Robinhood(RobinhoodBoard::default()),
        }
    }

    /// Whether this dart can be entered in this mode at all.
    pub fn accepts(&self, dart: &Dart) -> bool {
        dart.is_valid()
            && match self {
                ModeConfig::X01(_) => true,
                ModeConfig::RoundRobinhood => robinhood::accepts(dart),
            }
    }

    pub fn apply(&self, board: &mut Board, dart: Dart) -> DartOutcome {
        match (self, board) {
            (ModeConfig::X01(c), Board::X01(b)) => x01::apply(c, b, dart),
            (ModeConfig::RoundRobinhood, Board::Robinhood(b)) => robinhood::apply(b, dart),
            _ => unreachable!("board does not belong to this mode"),
        }
    }
}

impl Board {
    /// Monotonic "how far along" measure used for stats and ranking:
    /// points scored in X01, marks in Round Robinhood.
    pub fn progress(&self) -> u16 {
        match self {
            Board::X01(b) => b.start - b.remaining,
            Board::Robinhood(b) => b.total_marks(),
        }
    }
}
