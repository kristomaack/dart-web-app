use crate::app::use_app;
use crate::components::controls::{Segmented, Toggle};
use crate::components::reorder_list::ReorderList;
use crate::platform;
use dart_core::{FinishMode, MAX_PLAYERS, Match, MatchConfig, ModeConfig, ModeKind, Player};
use dart_core::modes::close_out;
use leptos::prelude::*;

const HIT_LABELS: [&str; 8] = ["0", "1", "2", "3", "4", "5", "6", "7"];
use leptos_router::hooks::{use_navigate, use_params_map};

#[component]
pub fn Setup() -> impl IntoView {
    let app = use_app();
    let navigate = use_navigate();
    let kind = use_params_map()
        .with_untracked(|p| p.get("mode").and_then(|s| ModeKind::from_slug(&s)))
        .unwrap_or(ModeKind::X01);

    // Rematch draft wins; otherwise default rules with the last group of players.
    let draft = app.draft.get_untracked().filter(|d| d.mode.kind() == kind);
    let is_rematch = draft.is_some();
    app.draft.set(None);
    let initial = draft.unwrap_or_else(|| {
        let last_players = app
            .active
            .with_untracked(|m| m.as_ref().map(|m| m.config.players.clone()))
            .or_else(|| app.history.with_untracked(|h| h.first().map(|m| m.config.players.clone())))
            .unwrap_or_default();
        MatchConfig { mode: kind.default_config(), players: last_players, best_of: 1, finish: FinishMode::FirstOut }
    });

    let players = RwSignal::new(initial.players);
    let mode = RwSignal::new(initial.mode);
    let best_of = RwSignal::new(initial.best_of);
    let finish = RwSignal::new(initial.finish);
    let name = RwSignal::new(String::new());

    let full = move || players.with(|v| v.len() >= MAX_PLAYERS);
    let add = move |n: String| {
        let n = n.trim().to_string();
        if n.is_empty() || full() || players.with(|v| v.iter().any(|p| p.name.eq_ignore_ascii_case(&n))) {
            return;
        }
        players.update(|v| v.push(Player { id: platform::uid(), name: n }));
        platform::vibrate(platform::haptic::TAP);
    };

    let suggestions = move || {
        let taken = players.get();
        app.names
            .get()
            .into_iter()
            .filter(|n| !taken.iter().any(|p| p.name.eq_ignore_ascii_case(n)))
            .take(12)
            .map(|n| {
                let label = n.clone();
                view! {
                    <button type="button" class="chip" disabled=full on:click=move |_| add(n.clone())>
                        "+ "{label}
                    </button>
                }
            })
            .collect_view()
    };

    let hits = move || match mode.get() {
        ModeConfig::CloseOut(c) => Some(c.hits),
        _ => None,
    };
    let x01 = move || match mode.get() {
        ModeConfig::X01(c) => Some(c),
        _ => None,
    };
    let set_x01 = move |f: &dyn Fn(&mut dart_core::modes::X01Config)| {
        mode.update(|m| {
            if let ModeConfig::X01(c) = m {
                f(c)
            }
        })
    };

    let start = move |_| {
        let config = MatchConfig {
            mode: mode.get_untracked(),
            players: players.get_untracked(),
            best_of: best_of.get_untracked(),
            finish: finish.get_untracked(),
        };
        if config.players.is_empty() {
            return;
        }
        app.remember_names(config.players.iter().map(|p| p.name.clone()));
        app.active.set(Some(Match::new(platform::uid(), platform::now(), config)));
        platform::vibrate(platform::haptic::TAP);
        navigate("/play", Default::default());
    };

    view! {
        <main class="page with-footer">
            <nav class="bar">
                <a class="bar-btn" href="/">"‹ Games"</a>
                <span class="bar-title">{if is_rematch { "Rematch" } else { kind.title() }}</span>
                <span class="bar-btn"></span>
            </nav>

            <section class="group">
                <div class="group-head">
                    <h2 class="group-title">
                        "Players " <span class="count">{move || players.with(Vec::len)}"/"{MAX_PLAYERS}</span>
                    </h2>
                    <button
                        type="button"
                        class="text-btn"
                        disabled=move || players.with(|v| v.len() < 2)
                        on:click=move |_| {
                            players.update(|v| platform::shuffle(v));
                            platform::vibrate(platform::haptic::TAP);
                        }
                    >
                        "Shuffle"
                    </button>
                </div>
                <ReorderList players=players/>
                <form
                    class="add-player"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        add(name.get_untracked());
                        name.set(String::new());
                    }
                >
                    <input
                        class="field"
                        type="text"
                        placeholder=move || if full() { "Max 8 players" } else { "Add player" }
                        enterkeyhint="done"
                        autocomplete="off"
                        autocapitalize="words"
                        maxlength="20"
                        disabled=full
                        prop:value=move || name.get()
                        on:input=move |ev| name.set(event_target_value(&ev))
                    />
                    <button type="submit" class="btn-small" disabled=move || full() || name.with(|n| n.trim().is_empty())>
                        "Add"
                    </button>
                </form>
                <div class="chips">{suggestions}</div>
                <p class="footnote">"Drag ≡ to change throwing order."</p>
            </section>

            <Show when=move || x01().is_some()>
                            <section class="group">
                                <h2 class="group-title">"Rules"</h2>
                                <div class="list">
                                    <div class="row">
                                        <span class="row-title grow">"Start"</span>
                                        <Segmented
                                            options=vec![(301u16, "301"), (501, "501")]
                                            value=Signal::derive(move || x01().map(|c| c.start).unwrap_or(501))
                                            on_change=Callback::new(move |v: u16| set_x01(&|c| c.start = v))
                                        />
                                    </div>
                                    <Toggle
                                        label="Double in"
                                        detail="Scoring starts with a double"
                                        value=Signal::derive(move || x01().is_some_and(|c| c.double_in))
                                        on_change=Callback::new(move |v: bool| set_x01(&|c| c.double_in = v))
                                    />
                                    <Toggle
                                        label="Double out"
                                        detail="Finish on a double or bullseye"
                                        value=Signal::derive(move || x01().is_some_and(|c| c.double_out))
                                        on_change=Callback::new(move |v: bool| set_x01(&|c| c.double_out = v))
                                    />
                                </div>
                            </section>
            </Show>

            <Show when=move || hits().is_some()>
                <section class="group">
                    <h2 class="group-title">"Rules"</h2>
                    <div class="list">
                        <div class="row">
                            <span class="row-title grow">"Hits to close"</span>
                            <Segmented
                                options=close_out::HIT_OPTIONS.iter().map(|&h| (h, HIT_LABELS[h as usize])).collect()
                                value=Signal::derive(move || hits().unwrap_or(3))
                                on_change=Callback::new(move |h: u8| {
                                    mode.update(|m| {
                                        if let ModeConfig::CloseOut(c) = m {
                                            c.hits = h
                                        }
                                    })
                                })
                            />
                        </div>
                    </div>
                    <p class="footnote">"Single = 1 hit, double = 2, triple = 3. Bullseye counts as 2."</p>
                </section>
            </Show>

            <section class="group">
                <h2 class="group-title">"Match"</h2>
                <div class="list">
                    <div class="row">
                        <span class="row-title grow">"Best of"</span>
                        <Segmented
                            options=vec![(1u8, "1"), (3, "3"), (5, "5"), (7, "7")]
                            value=best_of
                            on_change=Callback::new(move |v| best_of.set(v))
                        />
                    </div>
                    <div class="row stacked">
                        <span class="row-title">"When someone wins"</span>
                        <Segmented
                            options=vec![(FinishMode::FirstOut, "Game ends"), (FinishMode::PlayForPlaces, "Play for places")]
                            value=finish
                            on_change=Callback::new(move |v| finish.set(v))
                        />
                    </div>
                </div>
                <p class="footnote">
                    {move || match (finish.get(), best_of.get()) {
                        (FinishMode::FirstOut, 1) => "The first player to finish wins; the rest are ranked by progress.",
                        (FinishMode::FirstOut, _) => "The first player to win the majority of legs wins the match.",
                        (FinishMode::PlayForPlaces, 1) => "Finished players sit out while the rest play on for 2nd, 3rd…",
                        (FinishMode::PlayForPlaces, _) => "Match winners sit out while the rest play more legs for the next places.",
                    }}
                </p>
            </section>

            <footer class="footer">
                <button
                    type="button"
                    class="btn-primary"
                    disabled=move || players.with(Vec::is_empty)
                    on:click=start
                >
                    {move || match players.with(Vec::len) {
                        0 => "Add a player to start".to_string(),
                        1 => "Start practice".to_string(),
                        n => format!("Start game · {n} players"),
                    }}
                </button>
            </footer>
        </main>
    }
}
