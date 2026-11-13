use crate::app::use_app;
use crate::format;
use crate::platform;
use crate::rules::rules;
use dart_core::ModeKind;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// A game in the "New game" list: the row opens setup, and the (i) beside the
/// chevron opens a card with the rules. The card grows out of the (i) and
/// shrinks back into it, so it's clear where it came from and where it went.
#[component]
fn ModeRow(kind: ModeKind) -> impl IntoView {
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    // Centre of the (i) in viewport coordinates, measured when it's tapped.
    let origin = StoredValue::new((0.0f64, 0.0f64));
    let closing = StoredValue::new(false);
    let slug = kind.slug();
    let title_id = format!("rules-{slug}");
    let labelled_by = title_id.clone();

    // Transform that collapses the card onto the (i).
    let collapsed = move |d: &web_sys::HtmlDialogElement| {
        if platform::reduced_motion() {
            return "none".to_string();
        }
        let r = d.get_bounding_client_rect();
        let (ox, oy) = origin.get_value();
        let (dx, dy) = (ox - (r.left() + r.width() / 2.0), oy - (r.top() + r.height() / 2.0));
        format!("translate({dx:.1}px,{dy:.1}px) scale(0.2)")
    };

    let open = move |ev: web_sys::MouseEvent| {
        if let Some(btn) = ev.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
            let r = btn.get_bounding_client_rect();
            origin.set_value((r.left() + r.width() / 2.0, r.top() + r.height() / 2.0));
        }
        let Some(d) = dialog.get_untracked() else { return };
        // Drop a held closing frame, if any, so it can't hide the card again.
        platform::cancel_animations(&d);
        closing.set_value(false);
        if d.show_modal().is_err() {
            return;
        }
        let from = collapsed(&d);
        let frames = serde_json::json!([{ "transform": from, "opacity": 0 }, { "transform": "none", "opacity": 1 }]);
        platform::animate_part(&d, None, false, &frames.to_string(), 340.0, platform::EASE, None);
        let fade = serde_json::json!([{ "opacity": 0 }, { "opacity": 1 }]).to_string();
        platform::animate_part(&d, Some("::backdrop"), false, &fade, 250.0, "ease-out", None);
    };

    let close = move || {
        let Some(d) = dialog.get_untracked() else { return };
        if closing.get_value() || !d.open() {
            return;
        }
        closing.set_value(true);
        let to = collapsed(&d);
        let frames = serde_json::json!([{ "transform": "none", "opacity": 1 }, { "transform": to, "opacity": 0 }]);
        let fade = serde_json::json!([{ "opacity": 1 }, { "opacity": 0 }]).to_string();
        platform::animate_part(&d, Some("::backdrop"), true, &fade, 220.0, "ease-in", None);
        let done = d.clone();
        // Held until `close()`, so the card can't flash back for a frame.
        platform::animate_part(
            &d,
            None,
            true,
            &frames.to_string(),
            220.0,
            "ease-in",
            Some(Box::new(move || {
                done.close();
                platform::cancel_animations(&done);
                closing.set_value(false);
            })),
        );
    };

    let sections = rules(kind)
        .iter()
        .map(|s| {
            view! {
                <h3>{s.heading}</h3>
                <p>{s.body}</p>
            }
        })
        .collect_view();

    view! {
        <li class="mode-item">
            <a class="row link" href=format!("/setup/{slug}")>
                <span class=format!("mode-icon mode-{slug}") aria-hidden="true"></span>
                <div class="row-text">
                    <span class="row-title">{kind.title()}</span>
                    <span class="row-sub">{kind.tagline()}</span>
                </div>
                <span class="chev" aria-hidden="true">"›"</span>
            </a>
            <button
                type="button"
                class="info-btn"
                aria-label=format!("How to play {}", kind.title())
                aria-haspopup="dialog"
                on:click=open
            >
                <span aria-hidden="true">"i"</span>
            </button>
            <dialog
                node_ref=dialog
                class="rules"
                aria-labelledby=labelled_by
                // A click on the dimmed backdrop lands on the dialog itself.
                on:click=move |ev| {
                    if ev.target() == ev.current_target() {
                        close();
                    }
                }
                on:cancel=move |ev: web_sys::Event| {
                    ev.prevent_default();
                    close();
                }
            >
                <div class="rules-body">
                    <header class="rules-head">
                        <span class=format!("mode-icon mode-{slug}") aria-hidden="true"></span>
                        <h2 id=title_id>{kind.title()}</h2>
                        <button type="button" class="rules-close" aria-label="Close" on:click=move |_| close()>
                            <span aria-hidden="true">"✕"</span>
                        </button>
                    </header>
                    {sections}
                    <a class="btn-primary" href=format!("/setup/{slug}")>
                        {format!("Play {}", kind.title())}
                    </a>
                </div>
            </dialog>
        </li>
    }
}

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
                    {ModeKind::ALL.into_iter().map(|kind| view! { <ModeRow kind/> }).collect_view()}
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
