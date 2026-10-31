use crate::app::use_app;
use crate::format;
use dart_core::ModeKind;
use leptos::prelude::*;

#[component]
pub fn Home() -> impl IntoView {
    let app = use_app();

    let resume = move || {
        app.active.with(|m| {
            let m = m.as_ref()?;
            let s = m.state();
            let names = m.config.players.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ");
            let status = if s.finished {
                "Finished — see results".to_string()
            } else {
                format!("{} to throw", m.config.players[s.current].name)
            };
            let href = if s.finished { "/results" } else { "/play" };
            Some(view! {
                <a class="card resume" href=href>
                    <div class="resume-text">
                        <span class="eyebrow">"Game in progress"</span>
                        <span class="row-title">{m.config.mode.label()}</span>
                        <span class="row-sub">{names}</span>
                        <span class="row-sub accent">{status}</span>
                    </div>
                    <span class="chev" aria-hidden="true">"›"</span>
                </a>
            })
        })
    };

    let recent = move || {
        app.history.with(|h| {
            h.iter()
                .take(5)
                .map(|m| {
                    let s = m.state();
                    let winner = s.ranking().first().map(|&i| m.config.players[i].name.clone()).unwrap_or_default();
                    view! {
                        <li class="row">
                            <div class="row-text">
                                <span class="row-title">{winner}" won"</span>
                                <span class="row-sub">{m.config.mode.label()}" · "{format::date(m.created_at)}</span>
                            </div>
                            <span class="row-detail">{m.config.players.len()}" players"</span>
                        </li>
                    }
                })
                .collect_view()
        })
    };

    view! {
        <main class="page">
            <header class="large-title">
                <h1>"Darts"</h1>
            </header>
            {resume}
            <section class="group">
                <h2 class="group-title">"New game"</h2>
                <ul class="list">
                    {ModeKind::ALL
                        .into_iter()
                        .map(|k| {
                            view! {
                                <li>
                                    <a class="row link" href=format!("/setup/{}", k.slug())>
                                        <span class=format!("mode-icon mode-{}", k.slug()) aria-hidden="true"></span>
                                        <div class="row-text">
                                            <span class="row-title">{k.title()}</span>
                                            <span class="row-sub">{k.tagline()}</span>
                                        </div>
                                        <span class="chev" aria-hidden="true">"›"</span>
                                    </a>
                                </li>
                            }
                        })
                        .collect_view()}
                    <li class="row muted">
                        <span class="mode-icon mode-soon" aria-hidden="true"></span>
                        <div class="row-text">
                            <span class="row-title">"More games"</span>
                            <span class="row-sub">"Coming soon"</span>
                        </div>
                    </li>
                </ul>
            </section>
            <Show when=move || app.history.with(|h| !h.is_empty())>
                <section class="group">
                    <h2 class="group-title">"Recent"</h2>
                    <ul class="list">{recent}</ul>
                </section>
            </Show>
        </main>
    }
}
