use crate::app::use_app;
use crate::format;
use dart_core::{FinishMode, MatchConfig, ModeConfig};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

#[component]
pub fn Results() -> impl IntoView {
    let app = use_app();
    let navigate = use_navigate();

    let Some(m) = app.active.get_untracked().filter(|m| m.state().finished) else {
        return view! {
            <main class="page empty">
                <p>"No finished game to show."</p>
                <a class="btn-primary" href="/">"Choose a game"</a>
            </main>
        }
        .into_any();
    };
    let s = m.state();
    let ranking = s.ranking();
    let winner = m.config.players[ranking[0]].name.clone();
    let x01 = matches!(m.config.mode, ModeConfig::X01(_));
    let multi_leg = m.config.legs_to_win() > 1;

    let rows = ranking
        .iter()
        .map(|&i| {
            let p = &s.players[i];
            let st = p.stats;
            let mut parts = vec![
                format!("{} {:.1}", if x01 { "Avg" } else { "MPR" }, st.per_round()),
                format!("{} darts", st.darts),
            ];
            if x01 {
                parts.push(format!("Best {}", st.best_turn));
                if let Some(c) = st.best_checkout {
                    parts.push(format!("Out {c}"));
                }
            }
            if multi_leg {
                parts.insert(0, format!("{} legs", p.legs_won));
            }
            let place = p.place.unwrap_or(0);
            view! {
                <li class="row standing">
                    <span class=format!("place place-{}", place.min(4))>{place}</span>
                    <div class="row-text">
                        <span class="row-title">{m.config.players[i].name.clone()}</span>
                        <span class="row-sub">{parts.join(" · ")}</span>
                    </div>
                </li>
            }
        })
        .collect_view();

    // Rematch: same rules, worst finisher throws first.
    let rematch_config = MatchConfig {
        players: ranking.iter().rev().map(|&i| m.config.players[i].clone()).collect(),
        ..m.config.clone()
    };
    let slug = m.config.mode.kind().slug();
    let nav = navigate.clone();
    let rematch = move |_| {
        app.archive_active();
        app.draft.set(Some(rematch_config.clone()));
        nav(&format!("/setup/{slug}"), Default::default());
    };
    let nav = navigate.clone();
    let done = move |_| {
        app.archive_active();
        nav("/", Default::default());
    };
    let nav = navigate;
    let undo = move |_| {
        app.active.update(|m| {
            if let Some(m) = m {
                m.undo();
            }
        });
        nav("/play", Default::default());
    };

    let subtitle = match (m.config.finish, m.config.best_of) {
        (_, 1) => m.config.mode.label(),
        (FinishMode::FirstOut, n) => format!("{} · Best of {n}", m.config.mode.label()),
        (FinishMode::PlayForPlaces, n) => format!("{} · Best of {n} · Places", m.config.mode.label()),
    };

    view! {
        <main class="page with-footer">
            <header class="hero">
                <div class="trophy" aria-hidden="true">"🏆"</div>
                <h1>{winner}" wins"</h1>
                <p class="row-sub">{subtitle}</p>
            </header>
            <section class="group">
                <h2 class="group-title">"Standings"</h2>
                <ol class="list">{rows}</ol>
                <p class="footnote">{format!("Finished {}", format::date(crate::platform::now()))}</p>
            </section>
            <footer class="footer stack">
                <button type="button" class="btn-primary" on:click=rematch>"Rematch"</button>
                <div class="footer-row">
                    <button type="button" class="btn-secondary" on:click=undo>"Undo last dart"</button>
                    <button type="button" class="btn-secondary" on:click=done>"Done"</button>
                </div>
            </footer>
        </main>
    }
    .into_any()
}
