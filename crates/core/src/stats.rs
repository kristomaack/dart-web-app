use serde::{Deserialize, Serialize};

/// Per-player totals across a whole match, derived from the throw log.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerStats {
    pub darts: u32,
    pub turns: u32,
    /// Points scored (X01) or marks hit (Round Robinhood).
    pub progress: u32,
    pub best_turn: u16,
    pub busts: u32,
    /// Highest finishing turn (X01 only).
    pub best_checkout: Option<u16>,
}

impl PlayerStats {
    /// 3-dart average in X01, marks per round in Round Robinhood.
    pub fn per_round(&self) -> f32 {
        if self.darts == 0 { 0.0 } else { self.progress as f32 * 3.0 / self.darts as f32 }
    }

    pub(crate) fn record_turn(&mut self, score: u16, bust: bool) {
        self.turns += 1;
        self.progress += score as u32;
        self.best_turn = self.best_turn.max(score);
        if bust {
            self.busts += 1;
        }
    }
}
