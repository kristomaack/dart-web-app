//! The match engine. A [`Match`] is just its config plus an append-only log of
//! darts; everything else ([`MatchState`]) is derived by replaying that log.
//! Undo is popping the log. A backend can store and validate the same log.

use crate::dart::Dart;
use crate::modes::{Board, DartOutcome, ModeConfig};
use crate::player::Player;
use crate::stats::PlayerStats;
use serde::{Deserialize, Serialize};

pub const MAX_PLAYERS: usize = 8;
pub const DARTS_PER_TURN: usize = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishMode {
    /// The match ends as soon as someone wins it.
    #[default]
    FirstOut,
    /// Winners drop out and the rest keep playing for the remaining places.
    PlayForPlaces,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchConfig {
    pub mode: ModeConfig,
    /// Throwing order.
    pub players: Vec<Player>,
    /// Best of N legs; 1 is a single game.
    pub best_of: u8,
    pub finish: FinishMode,
}

impl MatchConfig {
    pub fn legs_to_win(&self) -> u8 {
        self.best_of.max(1) / 2 + 1
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Match {
    pub id: String,
    /// Milliseconds since the Unix epoch.
    pub created_at: f64,
    pub config: MatchConfig,
    pub throws: Vec<Dart>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThrowError {
    Finished,
    NotAllowed,
}

impl Match {
    pub fn new(id: String, created_at: f64, config: MatchConfig) -> Self {
        Self { id, created_at, config, throws: Vec::new() }
    }

    pub fn state(&self) -> MatchState {
        MatchState::replay(&self.config, &self.throws)
    }

    pub fn throw(&mut self, dart: Dart) -> Result<MatchState, ThrowError> {
        if !self.config.mode.accepts(&dart) {
            return Err(ThrowError::NotAllowed);
        }
        if self.state().finished {
            return Err(ThrowError::Finished);
        }
        self.throws.push(dart);
        Ok(self.state())
    }

    pub fn undo(&mut self) -> bool {
        self.throws.pop().is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlayerState {
    pub board: Board,
    pub legs_won: u8,
    pub stats: PlayerStats,
    /// Final place in the match (1 = winner), once decided.
    pub place: Option<u8>,
    pub last_turn: Vec<Dart>,
    pub last_turn_bust: bool,
}

/// What the most recent dart caused, for toasts and haptics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Event {
    TurnEnded { player: usize, score: u16 },
    Bust { player: usize },
    LegWon { player: usize },
    Placed { player: usize, place: u8 },
    MatchOver { winner: usize },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MatchState {
    pub players: Vec<PlayerState>,
    /// 1-based leg number.
    pub leg: u8,
    pub leg_starter: usize,
    pub current: usize,
    /// Darts thrown so far in the current turn.
    pub turn: Vec<(Dart, DartOutcome)>,
    pub finished: bool,
    pub last_event: Option<Event>,
    turn_start: Board,
}

impl MatchState {
    pub fn replay(cfg: &MatchConfig, throws: &[Dart]) -> Self {
        let board = cfg.mode.initial_board();
        let mut s = Self {
            players: cfg
                .players
                .iter()
                .map(|_| PlayerState {
                    board: board.clone(),
                    legs_won: 0,
                    stats: PlayerStats::default(),
                    place: None,
                    last_turn: Vec::new(),
                    last_turn_bust: false,
                })
                .collect(),
            leg: 1,
            leg_starter: 0,
            current: 0,
            turn: Vec::new(),
            finished: cfg.players.is_empty(),
            last_event: None,
            turn_start: board,
        };
        for &dart in throws {
            if s.finished {
                break;
            }
            s.step(cfg, dart);
        }
        s
    }

    pub fn darts_left(&self) -> usize {
        DARTS_PER_TURN - self.turn.len()
    }

    /// Turn score so far (0 while busted is impossible: busts end the turn).
    pub fn turn_score(&self) -> u16 {
        self.players[self.current].board.progress() - self.turn_start.progress()
    }

    /// Everyone ordered best-first: final places, then legs, then progress.
    pub fn ranking(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.players.len()).collect();
        order.sort_by_key(|&i| {
            let p = &self.players[i];
            (
                p.place.unwrap_or(u8::MAX),
                std::cmp::Reverse(p.legs_won),
                std::cmp::Reverse(p.board.progress()),
            )
        });
        order
    }

    fn step(&mut self, cfg: &MatchConfig, dart: Dart) {
        self.last_event = None;
        let p = self.current;
        let outcome = cfg.mode.apply(&mut self.players[p].board, dart);
        self.turn.push((dart, outcome));
        self.players[p].stats.darts += 1;

        match outcome {
            DartOutcome::Bust => {
                self.players[p].board = self.turn_start.clone();
                self.end_turn(true);
                self.last_event = Some(Event::Bust { player: p });
                self.advance();
            }
            DartOutcome::Finished => {
                let score = self.end_turn(false);
                if matches!(cfg.mode, ModeConfig::X01(_)) {
                    let best = &mut self.players[p].stats.best_checkout;
                    *best = Some(best.unwrap_or(0).max(score));
                }
                self.player_finished(cfg, p);
            }
            _ if self.turn.len() == DARTS_PER_TURN => {
                let score = self.end_turn(false);
                self.last_event = Some(Event::TurnEnded { player: p, score });
                self.advance();
            }
            _ => {}
        }
    }

    fn end_turn(&mut self, bust: bool) -> u16 {
        let p = &mut self.players[self.current];
        let score = if bust { 0 } else { p.board.progress() - self.turn_start.progress() };
        p.stats.record_turn(score, bust);
        p.last_turn = self.turn.drain(..).map(|(d, _)| d).collect();
        p.last_turn_bust = bust;
        score
    }

    fn next_active(&self, from: usize) -> usize {
        let n = self.players.len();
        (1..=n).map(|i| (from + i) % n).find(|&i| self.players[i].place.is_none()).unwrap_or(from)
    }

    fn advance(&mut self) {
        self.current = self.next_active(self.current);
        self.turn_start = self.players[self.current].board.clone();
    }

    fn placed_count(&self) -> u8 {
        self.players.iter().filter(|p| p.place.is_some()).count() as u8
    }

    fn player_finished(&mut self, cfg: &MatchConfig, p: usize) {
        let multi_leg = cfg.legs_to_win() > 1;
        if multi_leg || self.placed_count() == 0 {
            self.players[p].legs_won += 1;
        }
        let won_match = !multi_leg || self.players[p].legs_won >= cfg.legs_to_win();
        if won_match {
            let place = self.placed_count() + 1;
            self.players[p].place = Some(place);
            self.last_event = Some(Event::Placed { player: p, place });
        } else {
            self.last_event = Some(Event::LegWon { player: p });
        }

        let active = self.players.iter().filter(|p| p.place.is_none()).count();
        if active <= 1 || (cfg.finish == FinishMode::FirstOut && won_match) {
            self.finish_match();
        } else if multi_leg {
            self.new_leg(cfg);
        } else {
            self.advance();
        }
    }

    fn new_leg(&mut self, cfg: &MatchConfig) {
        self.leg += 1;
        for p in self.players.iter_mut().filter(|p| p.place.is_none()) {
            p.board = cfg.mode.initial_board();
        }
        self.leg_starter = self.next_active(self.leg_starter);
        self.current = self.leg_starter;
        self.turn_start = self.players[self.current].board.clone();
    }

    fn finish_match(&mut self) {
        let mut next = self.placed_count() + 1;
        for i in self.ranking() {
            if self.players[i].place.is_none() {
                self.players[i].place = Some(next);
                next += 1;
            }
        }
        self.finished = true;
        let winner = self.ranking()[0];
        self.last_event = Some(Event::MatchOver { winner });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modes::{RobinhoodBoard, X01Board, X01Config};

    fn players(n: usize) -> Vec<Player> {
        (0..n).map(|i| Player { id: i.to_string(), name: format!("P{i}") }).collect()
    }

    fn x01(start: u16, double_in: bool, double_out: bool, n: usize) -> Match {
        Match::new(
            "m".into(),
            0.0,
            MatchConfig {
                mode: ModeConfig::X01(X01Config { start, double_in, double_out }),
                players: players(n),
                best_of: 1,
                finish: FinishMode::FirstOut,
            },
        )
    }

    fn rem(s: &MatchState, p: usize) -> u16 {
        match &s.players[p].board {
            Board::X01(X01Board { remaining, .. }) => *remaining,
            _ => unreachable!(),
        }
    }

    fn marks(s: &MatchState, p: usize) -> [u8; 7] {
        match &s.players[p].board {
            Board::Robinhood(RobinhoodBoard { marks }) => *marks,
            _ => unreachable!(),
        }
    }

    fn throw_all(m: &mut Match, darts: &[Dart]) -> MatchState {
        for &d in darts {
            m.throw(d).unwrap();
        }
        m.state()
    }

    #[test]
    fn turns_rotate_after_three_darts() {
        let mut m = x01(501, false, true, 2);
        let s = throw_all(&mut m, &[Dart::triple(20); 3]);
        assert_eq!(rem(&s, 0), 321);
        assert_eq!(s.current, 1);
        assert_eq!(s.last_event, Some(Event::TurnEnded { player: 0, score: 180 }));
    }

    #[test]
    fn bust_below_zero_reverts_turn() {
        // 301 - 180 = 121 left; next turn T20 T20 leaves 1.
        let mut m = x01(301, false, false, 2);
        let mut darts = vec![Dart::triple(20); 3];
        darts.extend([Dart::MISS; 3]);
        darts.extend([Dart::triple(20), Dart::triple(20)]);
        let mut finish = m.clone();
        let s = throw_all(&mut finish, &[darts.clone(), vec![Dart::single(1)]].concat());
        assert!(s.finished, "without double-out a single finishes");

        let s = throw_all(&mut m, &[darts, vec![Dart::single(2)]].concat());
        assert_eq!(rem(&s, 0), 121);
        assert_eq!(s.current, 1);
        assert_eq!(s.last_event, Some(Event::Bust { player: 0 }));
        assert_eq!(s.players[0].stats.busts, 1);
        assert_eq!(s.players[0].stats.progress, 180);
    }

    #[test]
    fn double_out_busts_on_one_and_on_single_finish() {
        let c = X01Config { start: 40, double_in: false, double_out: true };
        let mut b = X01Board::new(&c);
        assert_eq!(crate::modes::x01::apply(&c, &mut b, Dart::single(39)), DartOutcome::Bust);
        assert_eq!(crate::modes::x01::apply(&c, &mut b, Dart::single(20)), DartOutcome::Scored);
        assert_eq!(b.remaining, 20);
        assert_eq!(crate::modes::x01::apply(&c, &mut b, Dart::single(20)), DartOutcome::Bust);
        assert_eq!(crate::modes::x01::apply(&c, &mut b, Dart::double(10)), DartOutcome::Finished);

        let c = X01Config { start: 50, double_in: false, double_out: true };
        let mut b = X01Board::new(&c);
        assert_eq!(crate::modes::x01::apply(&c, &mut b, Dart::bullseye()), DartOutcome::Finished);
    }

    #[test]
    fn double_in_gates_scoring() {
        let mut m = x01(301, true, true, 1);
        let s = throw_all(&mut m, &[Dart::triple(20), Dart::single(20), Dart::double(10)]);
        assert_eq!(rem(&s, 0), 281);
        let s = throw_all(&mut m, &[Dart::single(20)]);
        assert_eq!(rem(&s, 0), 261);
    }

    #[test]
    fn bust_also_reverts_double_in() {
        let mut m = x01(30, true, true, 1);
        let s = throw_all(&mut m, &[Dart::double(14), Dart::triple(20)]);
        assert_eq!(s.players[0].board, Board::X01(X01Board { start: 30, remaining: 30, opened: false }));
        assert_eq!(s.turn.len(), 0);
    }

    #[test]
    fn undo_crosses_turn_boundary() {
        let mut m = x01(501, false, true, 2);
        throw_all(&mut m, &[Dart::triple(20); 3]);
        assert_eq!(m.state().current, 1);
        m.undo();
        let s = m.state();
        assert_eq!(s.current, 0);
        assert_eq!(s.turn.len(), 2);
        assert_eq!(rem(&s, 0), 381);
    }

    #[test]
    fn robinhood_marks_cap_and_mid_turn_win() {
        let mut m = Match::new(
            "r".into(),
            0.0,
            MatchConfig {
                mode: ModeConfig::RoundRobinhood,
                players: players(2),
                best_of: 1,
                finish: FinishMode::FirstOut,
            },
        );
        assert_eq!(m.throw(Dart::single(14)), Err(ThrowError::NotAllowed));
        let s = throw_all(&mut m, &[Dart::double(15), Dart::triple(15), Dart::single(15)]);
        assert_eq!(marks(&s, 0)[0], 3);
        assert_eq!(s.last_event, Some(Event::TurnEnded { player: 0, score: 3 }));

        let mut m2 = m.clone();
        throw_all(&mut m2, &[Dart::MISS; 3]); // P1
        throw_all(&mut m2, &[Dart::triple(16), Dart::triple(17), Dart::triple(18)]);
        throw_all(&mut m2, &[Dart::MISS; 3]); // P1
        let s = throw_all(&mut m2, &[Dart::triple(19), Dart::triple(20), Dart::outer_bull()]);
        assert_eq!(marks(&s, 0)[6], 1);
        throw_all(&mut m2, &[Dart::MISS; 3]); // P1
        let s = throw_all(&mut m2, &[Dart::bullseye()]);
        assert!(s.finished, "closing the last target wins mid-turn");
        assert_eq!(s.players[0].place, Some(1));
        assert_eq!(s.players[1].place, Some(2));
        assert_eq!(m2.throw(Dart::MISS), Err(ThrowError::Finished));
    }

    #[test]
    fn play_for_places_continues_without_winner() {
        let mut m = x01(40, false, true, 3);
        m.config.mode = ModeConfig::X01(X01Config { start: 40, double_in: false, double_out: true });
        m.config.finish = FinishMode::PlayForPlaces;
        let s = throw_all(&mut m, &[Dart::double(20)]);
        assert!(!s.finished);
        assert_eq!(s.players[0].place, Some(1));
        assert_eq!(s.current, 1);
        throw_all(&mut m, &[Dart::MISS; 3]); // P1
        let s = throw_all(&mut m, &[Dart::double(20)]); // P2 out
        assert!(s.finished, "one player left");
        assert_eq!(s.players[2].place, Some(2));
        assert_eq!(s.players[1].place, Some(3));
    }

    #[test]
    fn legs_rotate_starter_and_count() {
        let mut m = x01(40, false, true, 2);
        m.config.best_of = 3;
        let s = throw_all(&mut m, &[Dart::double(20)]);
        assert_eq!(s.leg, 2);
        assert_eq!(s.players[0].legs_won, 1);
        assert_eq!(s.current, 1, "player 2 starts leg 2");
        assert_eq!(rem(&s, 0), 40);
        assert_eq!(s.last_event, Some(Event::LegWon { player: 0 }));
        let s = throw_all(&mut m, &[Dart::double(20)]);
        assert_eq!(s.leg, 3);
        assert_eq!(s.current, 0);
        let s = throw_all(&mut m, &[Dart::double(20)]);
        assert!(s.finished);
        assert_eq!(s.ranking(), vec![0, 1]);
        assert_eq!(s.players[0].stats.best_checkout, Some(40));
    }

    #[test]
    fn match_roundtrips_through_json() {
        let mut m = x01(501, true, true, 2);
        throw_all(&mut m, &[Dart::double(20), Dart::bullseye(), Dart::MISS]);
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(serde_json::from_str::<Match>(&json).unwrap(), m);
    }
}
