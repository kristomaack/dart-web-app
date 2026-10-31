use crate::pages::{home::Home, play::Play, results::Results, setup::Setup};
use crate::repo::{LocalRepo, SharedRepo};
use dart_core::{Match, MatchConfig};
use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};
use std::sync::Arc;

const HISTORY_LIMIT: usize = 30;
const NAMES_LIMIT: usize = 24;

/// App-wide reactive state; every change is written through to the repo.
#[derive(Clone, Copy)]
pub struct AppState {
    pub active: RwSignal<Option<Match>>,
    pub history: RwSignal<Vec<Match>>,
    pub names: RwSignal<Vec<String>>,
    /// Pre-filled setup, e.g. from "Rematch".
    pub draft: RwSignal<Option<MatchConfig>>,
}

impl AppState {
    /// Move a finished match into history and clear the active slot.
    pub fn archive_active(&self) {
        if let Some(m) = self.active.get_untracked() {
            if m.state().finished && !m.throws.is_empty() {
                self.history.update(|h| {
                    h.retain(|x| x.id != m.id);
                    h.insert(0, m);
                    h.truncate(HISTORY_LIMIT);
                });
            }
        }
        self.active.set(None);
    }

    pub fn remember_names(&self, used: impl IntoIterator<Item = String>) {
        self.names.update(|names| {
            for n in used.into_iter().collect::<Vec<_>>().into_iter().rev() {
                names.retain(|x| !x.eq_ignore_ascii_case(&n));
                names.insert(0, n);
            }
            names.truncate(NAMES_LIMIT);
        });
    }
}

pub fn use_app() -> AppState {
    expect_context()
}

#[component]
pub fn App() -> impl IntoView {
    let repo: SharedRepo = Arc::new(LocalRepo);
    let state = AppState {
        active: RwSignal::new(repo.load_active()),
        history: RwSignal::new(repo.history()),
        names: RwSignal::new(repo.names()),
        draft: RwSignal::new(None),
    };
    provide_context(state);

    let r = repo.clone();
    Effect::new(move |_| {
        state.active.with(|m| match m {
            Some(m) => r.save_active(m),
            None => r.clear_active(),
        })
    });
    let r = repo.clone();
    Effect::new(move |_| state.history.with(|h| r.save_history(h)));
    let r = repo;
    Effect::new(move |_| state.names.with(|n| r.save_names(n)));

    view! {
        <Router>
            <Routes fallback=|| view! { <Home/> }>
                <Route path=path!("/") view=Home/>
                <Route path=path!("/setup/:mode") view=Setup/>
                <Route path=path!("/play") view=Play/>
                <Route path=path!("/results") view=Results/>
            </Routes>
        </Router>
    }
}
