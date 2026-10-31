use super::DartOutcome;
use crate::dart::Dart;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct X01Config {
    pub start: u16,
    pub double_in: bool,
    pub double_out: bool,
}

impl Default for X01Config {
    fn default() -> Self {
        Self { start: 501, double_in: false, double_out: true }
    }
}

impl X01Config {
    pub fn label(&self) -> String {
        let mut s = self.start.to_string();
        match (self.double_in, self.double_out) {
            (true, true) => s.push_str(" · Double in/out"),
            (true, false) => s.push_str(" · Double in"),
            (false, true) => s.push_str(" · Double out"),
            (false, false) => {}
        }
        s
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct X01Board {
    pub start: u16,
    pub remaining: u16,
    /// False until the player has doubled in (always true without double-in).
    pub opened: bool,
}

impl X01Board {
    pub fn new(c: &X01Config) -> Self {
        Self { start: c.start, remaining: c.start, opened: !c.double_in }
    }
}

pub fn apply(c: &X01Config, b: &mut X01Board, dart: Dart) -> DartOutcome {
    if !b.opened {
        if !dart.is_double() {
            return DartOutcome::NoScore;
        }
        b.opened = true;
    }
    let points = dart.points();
    if points == 0 {
        return DartOutcome::NoScore;
    }
    let left = b.remaining as i32 - points as i32;
    let bust = left < 0 || (c.double_out && (left == 1 || (left == 0 && !dart.is_double())));
    if bust {
        return DartOutcome::Bust;
    }
    b.remaining = left as u16;
    if left == 0 { DartOutcome::Finished } else { DartOutcome::Scored }
}
