//! Thin wrappers over browser APIs that may be missing (vibration on iOS,
//! wake lock on older browsers). Every call degrades to a no-op.

use js_sys::{Function, Promise, Reflect};
use std::cell::{Cell, RefCell};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

pub fn now() -> f64 {
    js_sys::Date::now()
}

pub fn uid() -> String {
    format!("{:x}{:08x}", now() as u64, (js_sys::Math::random() * u32::MAX as f64) as u32)
}

pub fn shuffle<T>(v: &mut [T]) {
    for i in (1..v.len()).rev() {
        let j = (js_sys::Math::random() * (i + 1) as f64) as usize;
        v.swap(i, j);
    }
}

pub fn confirm(message: &str) -> bool {
    web_sys::window().and_then(|w| w.confirm_with_message(message).ok()).unwrap_or(true)
}

fn method(target: &JsValue, name: &str) -> Option<Function> {
    Reflect::get(target, &name.into()).ok()?.dyn_into().ok()
}

pub mod haptic {
    pub const TAP: &[u32] = &[8];
    pub const TICK: &[u32] = &[4];
    pub const BUST: &[u32] = &[40, 60, 40];
    pub const WIN: &[u32] = &[30, 40, 90];
}

pub fn vibrate(pattern: &[u32]) {
    let Some(nav) = web_sys::window().map(|w| JsValue::from(w.navigator())) else { return };
    if let Some(f) = method(&nav, "vibrate") {
        let arr = pattern.iter().map(|&n| JsValue::from(n)).collect::<js_sys::Array>();
        let _ = f.call1(&nav, &arr);
    }
}

thread_local! {
    static WANT_AWAKE: Cell<bool> = const { Cell::new(false) };
    static SENTINEL: RefCell<Option<JsValue>> = const { RefCell::new(None) };
}

fn release(sentinel: JsValue) {
    if let Some(f) = method(&sentinel, "release") {
        let _ = f.call0(&sentinel);
    }
}

fn request_wake_lock() {
    let Some(nav) = web_sys::window().map(|w| JsValue::from(w.navigator())) else { return };
    let Ok(lock) = Reflect::get(&nav, &"wakeLock".into()) else { return };
    let Some(request) = (!lock.is_undefined()).then(|| method(&lock, "request")).flatten() else { return };
    let Ok(promise) = request.call1(&lock, &"screen".into()).and_then(|p| p.dyn_into::<Promise>()) else {
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(sentinel) = wasm_bindgen_futures::JsFuture::from(promise).await {
            if WANT_AWAKE.get() {
                if let Some(old) = SENTINEL.replace(Some(sentinel)) {
                    release(old);
                }
            } else {
                release(sentinel);
            }
        }
    });
}

/// Keep the screen on while a game is being scored.
pub fn keep_awake(on: bool) {
    WANT_AWAKE.set(on);
    if on {
        request_wake_lock();
    } else if let Some(s) = SENTINEL.take() {
        release(s);
    }
}

pub fn init() {
    // Browsers drop wake locks when the tab is hidden; take it back on return.
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let on_visibility = Closure::<dyn Fn()>::new(|| {
        let visible = web_sys::window()
            .and_then(|w| w.document())
            .is_some_and(|d| d.visibility_state() == web_sys::VisibilityState::Visible);
        if visible && WANT_AWAKE.get() {
            request_wake_lock();
        }
    });
    let _ = doc.add_event_listener_with_callback("visibilitychange", on_visibility.as_ref().unchecked_ref());
    on_visibility.forget();
}
