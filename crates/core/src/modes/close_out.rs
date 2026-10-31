use super::DartOutcome;
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
pub struct CloseOutConfig {
    /// Hits needed to close each target.
    pub hits: u8,
}

impl Default for CloseOutConfig {
    fn default() -> Self {
        Self { hits: 3 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseOutBoard {
    pub hits: u8,
    /// Marks per target, indexed like [`TARGETS`], each 0..=hits.
    pub marks: [u8; 7],
}

impl CloseOutBoard {
    pub fn new(c: &CloseOutConfig) -> Self {
        Self { hits: c.hits.max(1), marks: [0; 7] }
    }

    pub fn total_marks(&self) -> u16 {
        self.marks.iter().map(|&m| m as u16).sum()
    }

    pub fn is_closed(&self, i: usize) -> bool {
        self.marks[i] >= self.hits
    }

    pub fn is_complete(&self) -> bool {
        (0..TARGETS.len()).all(|i| self.is_closed(i))
    }
}

pub fn target_index(segment: Segment) -> Option<usize> {
    TARGETS.iter().position(|&t| t == segment)
}

pub fn accepts(dart: &Dart) -> bool {
    dart.segment == Segment::Miss || target_index(dart.segment).is_some()
}

pub fn apply(b: &mut CloseOutBoard, dart: Dart) -> DartOutcome {
    let Some(i) = target_index(dart.segment) else {
        return DartOutcome::NoScore;
    };
    if b.is_closed(i) {
        return DartOutcome::NoScore;
    }
    b.marks[i] = (b.marks[i] + dart.mult.factor() as u8).min(b.hits);
    if b.is_complete() { DartOutcome::Finished } else { DartOutcome::Scored }
}
