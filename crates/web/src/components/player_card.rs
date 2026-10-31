use crate::format;
use dart_core::{
    Board, DartOutcome, MatchState, ModeConfig, Player, checkout::checkout, modes::close_out,
};
use leptos::prelude::*;

fn per_round_label(mode: &ModeConfig) -> &'static str {
    match mode {
        ModeConfig::X01(_) => "Avg",
        ModeConfig::CloseOut(_) => "MPR",
    }
}

fn legs_dots(won: u8, to_win: u8) -> impl IntoView {
    (to_win > 1).then(|| {
        view! {
            <span class="legs" aria-label=format!("{won} of {to_win} legs")>
                {(0..to_win)
                    .map(|i| view! { <span class="leg-dot" class:won={i < won}></span> })
                    .collect_view()}
            </span>
        }
    })
}

/// The player at the board: big score, this turn's darts, finish hint.
#[component]
pub fn CurrentCard(
    state: Memo<Option<MatchState>>,
    players: Vec<Player>,
    mode: ModeConfig,
    legs_to_win: u8,
) -> impl IntoView {
    let name = Memo::new(move |_| state.with(|s| s.as_ref().map(|s| players[s.current].name.clone()).unwrap_or_default()));
    let stat_label = per_round_label(&mode);
    let with = move |f: &dyn Fn(&MatchState) -> AnyView| state.with(|s| s.as_ref().map(f));

    let mode_main = mode.clone();
    let main = move || {
        let mode = mode_main.clone();
        with(&move |s: &MatchState| match (&s.players[s.current].board, &mode) {
            (Board::X01(b), ModeConfig::X01(c)) => {
                let hint = if !b.opened {
                    "Double in to start scoring".to_string()
                } else {
                    checkout(b.remaining, s.darts_left(), c.double_out)
                        .map(|r| format::route(&r))
                        .unwrap_or_default()
                };
                view! {
                    <div class="score-big" aria-live="polite">{b.remaining}</div>
                    <div class="hint">{hint}</div>
                }
                .into_any()
            }
            (Board::CloseOut(b), _) => view! {
                <div class="targets">
                    {close_out::TARGETS
                        .iter()
                        .enumerate()
                        .map(|(i, t)| {
                            let label = match t {
                                dart_core::Segment::Num(n) => n.to_string(),
                                _ => "Bull".into(),
                            };
                            view! {
                                <div class="target" class:closed={b.is_closed(i)}>
                                    <span class="target-label">{label}</span>
                                    <span class="target-marks">{format::marks(b.marks[i], b.hits)}</span>
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
            }
            .into_any(),
            _ => ().into_any(),
        })
    };

    let slots = move || {
        with(&|s: &MatchState| {
            let total = s.turn_score();
            view! {
                <div class="slots">
                    {(0..dart_core::DARTS_PER_TURN)
                        .map(|i| match s.turn.get(i) {
                            Some((d, o)) => view! {
                                <span class="slot filled" class:dim=*o == DartOutcome::NoScore>
                                    {d.to_string()}
                                </span>
                            }
                            .into_any(),
                            None => view! { <span class="slot"></span> }.into_any(),
                        })
                        .collect_view()}
                    <span class="slot-total">{(!s.turn.is_empty()).then(|| format!("{total}"))}</span>
                </div>
            }
            .into_any()
        })
    };

    let meta = move || {
        with(&move |s: &MatchState| {
            let p = &s.players[s.current];
            view! {
                {legs_dots(p.legs_won, legs_to_win)}
                <span class="stat">{stat_label}" "{format!("{:.1}", p.stats.per_round())}</span>
            }
            .into_any()
        })
    };

    view! {
        <section class="current" aria-label=move || format!("{} to throw", name.get())>
            <div class="current-head">
                <span class="current-name">{name}</span>
                <span class="current-meta">{meta}</span>
            </div>
            {main}
            {slots}
        </section>
    }
}

/// Compact row for a player waiting their turn.
#[component]
pub fn PlayerRow(
    idx: usize,
    state: Memo<Option<MatchState>>,
    player: Player,
    legs_to_win: u8,
) -> impl IntoView {
    let body = move || {
        state.with(|s| {
            let s = s.as_ref()?;
            let p = &s.players[idx];
            let score = match &p.board {
                Board::X01(b) => view! { <span class="row-score">{b.remaining}</span> }.into_any(),
                Board::CloseOut(b) => view! {
                    <span class="mini-marks">
                        {b.marks
                            .iter()
                            .zip(close_out::TARGETS)
                            .map(|(&m, t)| {
                                let label = match t {
                                    dart_core::Segment::Num(n) => n.to_string(),
                                    _ => "B".into(),
                                };
                                view! {
                                    <span class="mini-target">
                                        <span class="mini-label">{label}</span>
                                        <span
                                            class="mini"
                                            class:closed={m >= b.hits}
                                            style:--fill-pct=format!("{}%", m as u32 * 100 / b.hits as u32)
                                        ></span>
                                    </span>
                                }
                            })
                            .collect_view()}
                    </span>
                }
                .into_any(),
            };
            let last = if p.last_turn_bust {
                "Bust".to_string()
            } else {
                format::route(&p.last_turn)
            };
            Some(view! {
                <div class="row-text">
                    <span class="row-title">
                        {player.name.clone()}
                        {p.place.map(|pl| view! { <span class="badge">{format::ordinal(pl)}</span> })}
                    </span>
                    <span class="row-sub" class:bust=p.last_turn_bust>{last}</span>
                </div>
                {legs_dots(p.legs_won, legs_to_win)}
                {score}
            })
        })
    };
    view! { <li class="row player-row">{body}</li> }
}
