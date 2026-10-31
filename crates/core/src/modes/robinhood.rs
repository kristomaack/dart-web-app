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

/// Hits needed to close a target.
pub const CLOSED: u8 = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RobinhoodBoard {
    /// Marks per target, indexed like [`TARGETS`], each 0..=3.
    pub marks: [u8; 7],
}

impl RobinhoodBoard {
    pub fn total_marks(&self) -> u16 {
        self.marks.iter().map(|&m| m as u16).sum()
    }

    pub fn closed_count(&self) -> usize {
        self.marks.iter().filter(|&&m| m >= CLOSED).count()
    }

    pub fn is_complete(&self) -> bool {
        self.closed_count() == TARGETS.len()
    }
}

pub fn target_index(segment: Segment) -> Option<usize> {
    TARGETS.iter().position(|&t| t == segment)
}

pub fn accepts(dart: &Dart) -> bool {
    dart.segment == Segment::Miss || target_index(dart.segment).is_some()
}

pub fn apply(b: &mut RobinhoodBoard, dart: Dart) -> DartOutcome {
    let Some(i) = target_index(dart.segment) else {
        return DartOutcome::NoScore;
    };
    if b.marks[i] >= CLOSED {
        return DartOutcome::NoScore;
    }
    b.marks[i] = (b.marks[i] + dart.mult.factor() as u8).min(CLOSED);
    if b.is_complete() { DartOutcome::Finished } else { DartOutcome::Scored }
}
