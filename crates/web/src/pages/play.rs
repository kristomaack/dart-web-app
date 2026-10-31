use crate::app::use_app;
use crate::components::pad::Pad;
use crate::components::player_card::{CurrentCard, PlayerRow};
use crate::format;
use crate::platform::{self, haptic};
use dart_core::{Dart, Event, MatchState, Mult};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use std::time::Duration;

#[derive(Clone, PartialEq)]
struct Toast {
    id: u64,
    text: String,
    warn: bool,
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
    let toast = RwSignal::new(None::<Toast>);

    platform::keep_awake(true);
    on_cleanup(|| platform::keep_awake(false));

    let show_toast = move |text: String, warn: bool| {
        let id = platform::now() as u64;
        toast.set(Some(Toast { id, text, warn }));
        set_timeout(
            move || toast.update(|t| if t.as_ref().is_some_and(|t| t.id == id) { *t = None }),
            Duration::from_millis(1600),
        );
    };

    let names: Vec<String> = config.players.iter().map(|p| p.name.clone()).collect();
    let progress_unit = match config.mode {
        dart_core::ModeConfig::X01(_) => "",
        dart_core::ModeConfig::CloseOut(_) => " marks",
    };
    let nav = navigate.clone();
    let on_dart = Callback::new(move |d: Dart| {
        let mut event = None;
        app.active.update(|m| {
            if let Some(Ok(s)) = m.as_mut().map(|m| m.throw(d)) {
                event = s.last_event;
            }
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
        app.active.update(|m| {
            if let Some(m) = m {
                m.undo();
            }
        });
        toast.set(None);
        mult.set(Mult::Single);
        platform::vibrate(haptic::TICK);
    });
    let can_undo = Signal::derive(move || app.active.with(|m| m.as_ref().is_some_and(|m| !m.throws.is_empty())));

    let nav = navigate.clone();
    let end_game = move |_| {
        if platform::confirm("End this game? It won't be saved to history.") {
            app.active.set(None);
            nav("/", Default::default());
        }
    };

    // Keyed on the turn, so each new thrower gets a fresh card (and its entrance).
    let turn_key = Memo::new(move |_| state.with(|s| s.as_ref().map(|s| (s.current, s.leg, s.players[s.current].stats.turns))));
    let legs_to_win = config.legs_to_win();
    let card_players = config.players.clone();
    let card_mode = config.mode.clone();
    let card = view! {
        <For
            each=move || [turn_key.get()]
            key=|k| *k
            children=move |_| {
                view! {
                    <CurrentCard
                        state=state
                        players=card_players.clone()
                        mode=card_mode.clone()
                        legs_to_win=legs_to_win
                    />
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
                {move || toast.get().map(|t| view! { <div class="toast" class:warn=t.warn>{t.text}</div> })}
            </div>
            <Pad kind=config.mode.kind() mult=mult on_dart=on_dart on_undo=on_undo can_undo=can_undo/>
        </main>
    }
    .into_any()
}
