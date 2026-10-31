//! Persistence boundary. Today everything lives in localStorage; a backend
//! would add an `ApiRepo` implementing the same trait (likely keeping local
//! storage as an offline cache) and swap it in `app.rs`.

use dart_core::Match;
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;

pub trait Repo: Send + Sync {
    fn load_active(&self) -> Option<Match>;
    fn save_active(&self, m: &Match);
    fn clear_active(&self);
    /// Previously used player names, most recent first.
    fn names(&self) -> Vec<String>;
    fn save_names(&self, names: &[String]);
    /// Finished matches, most recent first.
    fn history(&self) -> Vec<Match>;
    fn save_history(&self, matches: &[Match]);
}

pub type SharedRepo = Arc<dyn Repo>;

pub struct LocalRepo;

const ACTIVE: &str = "dart.active.v1";
const NAMES: &str = "dart.names.v1";
const HISTORY: &str = "dart.history.v1";

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn read<T: DeserializeOwned>(key: &str) -> Option<T> {
    let raw = storage()?.get_item(key).ok().flatten()?;
    serde_json::from_str(&raw).ok()
}

fn write<T: Serialize + ?Sized>(key: &str, value: &T) {
    if let (Some(s), Ok(json)) = (storage(), serde_json::to_string(value)) {
        let _ = s.set_item(key, &json);
    }
}

impl Repo for LocalRepo {
    fn load_active(&self) -> Option<Match> {
        read(ACTIVE)
    }
    fn save_active(&self, m: &Match) {
        write(ACTIVE, m)
    }
    fn clear_active(&self) {
        if let Some(s) = storage() {
            let _ = s.remove_item(ACTIVE);
        }
    }
    fn names(&self) -> Vec<String> {
        read(NAMES).unwrap_or_default()
    }
    fn save_names(&self, names: &[String]) {
        write(NAMES, names)
    }
    fn history(&self) -> Vec<Match> {
        read(HISTORY).unwrap_or_default()
    }
    fn save_history(&self, matches: &[Match]) {
        write(HISTORY, matches)
    }
}
