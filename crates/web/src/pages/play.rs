use crate::app::use_app;
use crate::components::pad::Pad;
use crate::components::player_card::{CurrentCard, PlayerRow};
use crate::format;
use crate::platform::{self, haptic};
use dart_core::{Dart, Event, MatchState, Mult, modes::cricket};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use std::time::Duration;
use wasm_bindgen::JsCast;

#[derive(Clone, PartialEq)]
struct Toast {
    id: u64,
    text: String,
    warn: bool,
    /// Playing its exit animation; removed once that ends.
    leaving: bool,
}

/// Matches the `toast-out` animation in app.css.
const TOAST_EXIT: Duration = Duration::from_millis(180);

/// A detached copy of the current card, positioned over it. The card itself
/// re-renders from live state, so this snapshot stands in for the outgoing
/// player while it fades. Taken before the state changes.
fn snapshot_card() -> Option<(web_sys::Element, web_sys::HtmlElement)> {
    let doc = web_sys::window()?.document()?;
    let old: web_sys::HtmlElement = doc.query_selector(".play-body .current").ok()??.dyn_into().ok()?;
    let body = old.closest(".play-body").ok()??;
    let ghost: web_sys::HtmlElement = old.clone_node_with_deep(true).ok()?.dyn_into().ok()?;
    ghost.set_attribute("aria-hidden", "true").ok()?;
    ghost.class_list().add_1("ghost").ok()?;
    let style = format!("top:{}px;left:{}px;width:{}px", old.offset_top(), old.offset_left(), old.offset_width());
    ghost.set_attribute("style", &style).ok()?;
    Some((body, ghost))
}

fn fade_out_card((body, ghost): (web_sys::Element, web_sys::HtmlElement), back: bool) {
    if body.append_child(&ghost).is_err() {
        return;
    }
    // Forward, the card sinks back into the queue below; undo lifts it away.
    let to = match (platform::reduced_motion(), back) {
        (true, _) => "none",
        (false, false) => "translateY(16px) scale(0.97)",
        (false, true) => "translateY(-16px) scale(0.97)",
    };
    let frames = serde_json::json!([{ "opacity": 1 }, { "opacity": 0, "transform": to }]);
    let done = ghost.clone();
    platform::animate(&ghost, &frames.to_string(), 180.0, "ease-in", Some(Box::new(move || done.remove())));
}

#[component]
pub fn Play() -> impl IntoView {
    let app = use_app();
    let navigate = use_navigate();

    let Some(config) = app.active.with_untracked(|m| m.as_ref().map(|m| m.config.clone())) else {
        return view! {
            <main class="page empty">
                <p>"No game in progress."</p>
                <a class="btn-primary" href="/">"Choose a game"</a>
            </main>
        }
        .into_any();
    };

    let state = Memo::new(move |_| app.active.with(|m| m.as_ref().map(|m| m.state())));
    let mult = RwSignal::new(Mult::Single);
    let toasts = RwSignal::new(Vec::<Toast>::new());
    let next_toast = StoredValue::new(0u64);

    platform::keep_awake(true);
    on_cleanup(|| platform::keep_awake(false));

    let dismiss = move |id: u64| {
        toasts.update(|v| v.iter_mut().filter(|t| t.id == id).for_each(|t| t.leaving = true));
        set_timeout(move || toasts.update(|v| v.retain(|t| t.id != id)), TOAST_EXIT);
    };
    let dismiss_all = move || {
        for id in toasts.with_untracked(|v| v.iter().filter(|t| !t.leaving).map(|t| t.id).collect::<Vec<_>>()) {
            dismiss(id);
        }
    };
    let show_toast = move |text: String, warn: bool| {
        dismiss_all();
        let id = next_toast.get_value();
        next_toast.set_value(id + 1);
        toasts.update(|v| v.push(Toast { id, text, warn, leaving: false }));
        set_timeout(
            move || {
                if toasts.with_untracked(|v| v.iter().any(|t| t.id == id && !t.leaving)) {
                    dismiss(id);
                }
            },
            Duration::from_millis(1600),
        );
    };

    // Keyed on the turn, so each new thrower gets a fresh card (and its entrance).
    let turn_key = Memo::new(move |_| state.with(|s| s.as_ref().map(|s| (s.current, s.leg, s.players[s.current].stats.turns))));
    // Which way the last turn change went, so undo animates backwards.
    let going_back = StoredValue::new(false);
    let change_turn = move |back: bool, f: &mut dyn FnMut()| {
        let before = turn_key.get_untracked();
        let snapshot = snapshot_card();
        going_back.set_value(back);
        f();
        if turn_key.get_untracked() != before {
            if let Some(snapshot) = snapshot {
                fade_out_card(snapshot, back);
            }
        }
    };

    let names: Vec<String> = config.players.iter().map(|p| p.name.clone()).collect();
    let progress_unit = match config.mode {
        dart_core::ModeConfig::X01(_) => "",
        dart_core::ModeConfig::Cricket(_) => " marks",
    };
    let nav = navigate.clone();
    let on_dart = Callback::new(move |d: Dart| {
        let mut event = None;
        change_turn(false, &mut || {
            app.active.update(|m| {
                if let Some(Ok(s)) = m.as_mut().map(|m| m.throw(d)) {
                    event = s.last_event;
                }
            })
        });
        match event {
            Some(Event::Bust { player }) => {
                platform::vibrate(haptic::BUST);
                show_toast(format!("Bust · {}", names[player]), true);
            }
            Some(Event::TurnEnded { player, score }) => {
                platform::vibrate(haptic::TAP);
                show_toast(format!("{} · {score}{progress_unit}", names[player]), false);
            }
            Some(Event::LegWon { player }) => {
                platform::vibrate(haptic::WIN);
                show_toast(format!("{} wins the leg", names[player]), false);
            }
            Some(Event::Placed { player, place }) => {
                platform::vibrate(haptic::WIN);
                show_toast(format!("{} finishes {}", names[player], format::ordinal(place)), false);
            }
            Some(Event::MatchOver { .. }) => {
                platform::vibrate(haptic::WIN);
                nav("/results", Default::default());
            }
            None => platform::vibrate(haptic::TAP),
        }
    });
    let on_undo = Callback::new(move |_| {
        change_turn(true, &mut || {
            app.active.update(|m| {
                if let Some(m) = m {
                    m.undo();
                }
            })
        });
        dismiss_all();
        mult.set(Mult::Single);
        platform::vibrate(haptic::TICK);
    });
    let dead = Signal::derive(move || {
        state.with(|s| {
            let Some(s) = s else { return Vec::new() };
            match &s.players[s.current].board {
                dart_core::Board::Cricket(b) => (0..cricket::TARGETS.len())
                    .filter(|&i| !cricket::is_live(b, i, s.opponents(s.current)))
                    .map(|i| cricket::TARGETS[i])
                    .collect(),
                _ => Vec::new(),
            }
        })
    });
    let can_undo = Signal::derive(move || app.active.with(|m| m.as_ref().is_some_and(|m| !m.throws.is_empty())));

    let nav = navigate.clone();
    let end_game = move |_| {
        if platform::confirm("End this game? It won't be saved to history.") {
            app.active.set(None);
            nav("/", Default::default());
        }
    };

    let legs_to_win = config.legs_to_win();
    let card_players = config.players.clone();
    let card_mode = config.mode.clone();
    let card = view! {
        <For
            each=move || [turn_key.get()]
            key=|k| *k
            children=move |_| {
                view! {
                    <div class="card-wrap" class:back=going_back.get_value()>
                        <CurrentCard
                            state=state
                            players=card_players.clone()
                            mode=card_mode.clone()
                            legs_to_win=legs_to_win
                        />
                    </div>
                }
            }
        />
    };

    // Waiting players in the order they'll throw next.
    let row_players = config.players.clone();
    let others = move || {
        let n = row_players.len();
        let order: Vec<usize> = state.with(|s: &Option<MatchState>| {
            s.as_ref().map(|s| (1..n).map(|i| (s.current + i) % n).collect()).unwrap_or_default()
        });
        order
            .into_iter()
            .map(|i| view! { <PlayerRow idx=i state=state player=row_players[i].clone() legs_to_win=legs_to_win/> })
            .collect_view()
    };

    let label = config.mode.label();
    let best_of = config.best_of;
    let subtitle = move || {
        state.with(|s| {
            s.as_ref().map(|s| if best_of > 1 { format!("Leg {} · Best of {best_of}", s.leg) } else { String::new() })
        })
    };

    view! {
        <main class="play">
            <nav class="bar">
                <a class="bar-btn" href="/">"‹ Home"</a>
                <span class="bar-title">
                    {label}
                    <small>{subtitle}</small>
                </span>
                <button type="button" class="bar-btn end" on:click=end_game>"End"</button>
            </nav>
            <div class="play-body">
                {card}
                <ul class="list others">{others}</ul>
            </div>
            <div class="toast-slot" aria-live="assertive">
                // Keyed on `leaving` too, so a toast re-renders into its exit.
                <For
                    each=move || toasts.get()
                    key=|t| (t.id, t.leaving)
                    children=|t| {
                        view! {
                            <div class="toast" class:warn=t.warn class:leaving=t.leaving aria-hidden=t.leaving.then_some("true")>
                                {t.text}
                            </div>
                        }
                    }
                />
            </div>
            <Pad kind=config.mode.kind() mult=mult on_dart=on_dart on_undo=on_undo can_undo=can_undo dead=dead/>
        </main>
    }
    .into_any()
}
