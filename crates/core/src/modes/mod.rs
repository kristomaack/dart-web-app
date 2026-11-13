//! Game-mode registry. Adding a mode means: a new `ModeKind` + `ModeConfig`
//! variant, a `Board` variant, and a module implementing its rules. The match
//! arms below make the compiler point out every place that needs to know.

pub mod cricket;
pub mod x01;

use crate::dart::Dart;
use serde::{Deserialize, Serialize};

pub use cricket::{CricketBoard, CricketConfig};
pub use x01::{X01Board, X01Config};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeKind {
    X01,
    /// Was "Close Out" before it gained points.
    #[serde(alias = "close_out")]
    Cricket,
}

impl ModeKind {
    pub const ALL: [ModeKind; 2] = [ModeKind::X01, ModeKind::Cricket];

    pub fn slug(self) -> &'static str {
        match self {
            ModeKind::X01 => "x01",
            ModeKind::Cricket => "cricket",
        }
    }

    /// Slugs this mode used to have, so old links still resolve. Keep in step
    /// with the `#[serde(alias)]`es on the variant, which do the same for saves.
    pub fn legacy_slugs(self) -> &'static [&'static str] {
        match self {
            ModeKind::X01 => &[],
            ModeKind::Cricket => &["close-out"],
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.slug() == s || k.legacy_slugs().contains(&s))
    }

    pub fn title(self) -> &'static str {
        match self {
            ModeKind::X01 => "301 / 501",
            ModeKind::Cricket => "Cricket",
        }
    }

    pub fn tagline(self) -> &'static str {
        match self {
            ModeKind::X01 => "First to exactly zero wins.",
            ModeKind::Cricket => "Close 15–20 and the Bull.",
        }
    }

    pub fn default_config(self) -> ModeConfig {
        match self {
            ModeKind::X01 => ModeConfig::X01(X01Config::default()),
            ModeKind::Cricket => ModeConfig::Cricket(CricketConfig::default()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModeConfig {
    X01(X01Config),
    #[serde(alias = "close_out")]
    Cricket(CricketConfig),
}

/// A single player's position within the current leg.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Board {
    X01(X01Board),
    Cricket(CricketBoard),
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
            ModeConfig::Cricket(_) => ModeKind::Cricket,
        }
    }

    /// Short human description, e.g. "501 · Double out".
    pub fn label(&self) -> String {
        match self {
            ModeConfig::X01(c) => c.label(),
            ModeConfig::Cricket(c) => {
                format!("Cricket · {} hits{}", c.hits, if c.points { " · points" } else { "" })
            }
        }
    }

    pub fn initial_board(&self) -> Board {
        match self {
            ModeConfig::X01(c) => Board::X01(X01Board::new(c)),
            ModeConfig::Cricket(c) => Board::Cricket(CricketBoard::new(c)),
        }
    }

    /// Whether this dart can be entered in this mode at all.
    pub fn accepts(&self, dart: &Dart) -> bool {
        dart.is_valid()
            && match self {
                ModeConfig::X01(_) => true,
                ModeConfig::Cricket(_) => cricket::accepts(dart),
            }
    }

    /// `others` are the boards of the opponents still in the leg.
    pub fn apply<'a>(
        &self,
        board: &mut Board,
        dart: Dart,
        others: impl Iterator<Item = &'a Board> + Clone,
    ) -> DartOutcome {
        match (self, board) {
            (ModeConfig::X01(c), Board::X01(b)) => x01::apply(c, b, dart),
            (ModeConfig::Cricket(_), Board::Cricket(b)) => cricket::apply(b, dart, others),
            _ => unreachable!("board does not belong to this mode"),
        }
    }

    /// Whether this board has won against `others` without throwing again.
    /// Only Cricket with points can be won by an opponent dropping out.
    pub fn is_won<'a>(&self, board: &Board, others: impl Iterator<Item = &'a Board>) -> bool {
        match (self, board) {
            (ModeConfig::X01(_), Board::X01(_)) => false,
            (ModeConfig::Cricket(_), Board::Cricket(b)) => cricket::is_won(b, others),
            _ => unreachable!("board does not belong to this mode"),
        }
    }
}

impl Board {
    /// Monotonic "how far along" measure used for stats and ranking:
    /// points scored in X01, marks in Cricket.
    pub fn progress(&self) -> u16 {
        match self {
            Board::X01(b) => b.start - b.remaining,
            Board::Cricket(b) => b.total_marks(),
        }
    }

    /// Cricket points (0 when the mode has none). Ranks players ahead of progress.
    pub fn points(&self) -> u32 {
        match self {
            Board::X01(_) => 0,
            Board::Cricket(b) => b.points,
        }
    }
}
