use super::{Board, DartOutcome};
use crate::dart::{Dart, Segment};
use serde::{Deserialize, Serialize};

/// The seven targets, in display order.
pub const TARGETS: [Segment; 7] = [
    Segment::Num(15),
    Segment::Num(16),
    Segment::Num(17),
    Segment::Num(18),
    Segment::Num(19),
    Segment::Num(20),
    Segment::Bull,
];

/// Hits-to-close choices offered in setup.
pub const HIT_OPTIONS: [u8; 4] = [1, 3, 5, 7];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CricketConfig {
    /// Hits needed to close each target.
    pub hits: u8,
    /// Extra hits on a closed target score its value while an opponent still
    /// has it open, and you must lead on points to win. Missing in games saved
    /// before points existed, which played without them.
    #[serde(default)]
    pub points: bool,
}

impl Default for CricketConfig {
    fn default() -> Self {
        Self { hits: 3, points: true }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CricketBoard {
    pub hits: u8,
    pub points_on: bool,
    /// Marks per target, indexed like [`TARGETS`], each 0..=hits.
    pub marks: [u8; 7],
    pub points: u32,
    /// Marks beyond closing that scored points; they count toward MPR.
    pub scoring_marks: u16,
}

impl CricketBoard {
    pub fn new(c: &CricketConfig) -> Self {
        Self { hits: c.hits.max(1), points_on: c.points, marks: [0; 7], points: 0, scoring_marks: 0 }
    }

    /// Closing marks plus scoring marks: what MPR and turn scores count.
    pub fn total_marks(&self) -> u16 {
        self.marks.iter().map(|&m| m as u16).sum::<u16>() + self.scoring_marks
    }

    pub fn is_closed(&self, i: usize) -> bool {
        self.marks[i] >= self.hits
    }

    pub fn is_complete(&self) -> bool {
        (0..TARGETS.len()).all(|i| self.is_closed(i))
    }
}

/// Points per mark on a target: its number, or 25 for the bull.
pub fn target_value(i: usize) -> u16 {
    match TARGETS[i] {
        Segment::Num(n) => n as u16,
        _ => 25,
    }
}

pub fn target_index(segment: Segment) -> Option<usize> {
    TARGETS.iter().position(|&t| t == segment)
}

pub fn accepts(dart: &Dart) -> bool {
    dart.segment == Segment::Miss || target_index(dart.segment).is_some()
}

fn cricket_boards<'a>(others: impl Iterator<Item = &'a Board>) -> impl Iterator<Item = &'a CricketBoard> {
    others.map(|b| match b {
        Board::Cricket(b) => b,
        _ => unreachable!("opponent board does not belong to Cricket"),
    })
}

/// Everything closed and, with points on, level with or ahead of every
/// opponent still in the leg.
pub fn is_won<'a>(b: &CricketBoard, others: impl Iterator<Item = &'a Board>) -> bool {
    b.is_complete() && (!b.points_on || cricket_boards(others).all(|o| b.points >= o.points))
}

/// `others` are the boards of the opponents still playing this leg.
pub fn apply<'a>(b: &mut CricketBoard, dart: Dart, others: impl Iterator<Item = &'a Board> + Clone) -> DartOutcome {
    let scored = mark(b, dart, others.clone());
    // Checked on every dart, not only scoring ones: a player who closed out
    // behind can become the leader without hitting anything (an opponent
    // placed, or every number is dead), and must not be stuck.
    if is_won(b, others) {
        DartOutcome::Finished
    } else if scored {
        DartOutcome::Scored
    } else {
        DartOutcome::NoScore
    }
}

/// Whether more hits on target `i` score points: points are on and some
/// opponent still has it open.
pub fn can_score<'a>(b: &CricketBoard, i: usize, others: impl Iterator<Item = &'a Board>) -> bool {
    b.points_on && cricket_boards(others).any(|o| !o.is_closed(i))
}

/// Whether a hit on target `i` would change anything for this player.
pub fn is_live<'a>(b: &CricketBoard, i: usize, others: impl Iterator<Item = &'a Board>) -> bool {
    !b.is_closed(i) || can_score(b, i, others)
}

/// Applies the dart's marks and points; whether it changed the board.
fn mark<'a>(b: &mut CricketBoard, dart: Dart, others: impl Iterator<Item = &'a Board>) -> bool {
    let Some(i) = target_index(dart.segment) else {
        return false;
    };
    let can_score = can_score(b, i, others);
    if b.is_closed(i) && !can_score {
        return false;
    }
    let total = b.marks[i] + dart.mult.factor() as u8;
    let overflow = total.saturating_sub(b.hits);
    b.marks[i] = total.min(b.hits);
    if can_score {
        b.points = b.points.saturating_add(overflow as u32 * target_value(i) as u32);
        b.scoring_marks = b.scoring_marks.saturating_add(overflow as u16);
    }
    true
}
