//! Dart scoring rules, independent of any UI or storage.

pub mod checkout;
pub mod dart;
pub mod game;
pub mod modes;
pub mod player;
pub mod stats;

pub use dart::{Dart, Mult, Segment};
pub use game::{Event, FinishMode, Match, MatchConfig, MatchState, PlayerState, DARTS_PER_TURN, MAX_PLAYERS};
pub use modes::{Board, DartOutcome, ModeConfig, ModeKind};
pub use player::Player;
pub use stats::PlayerStats;
