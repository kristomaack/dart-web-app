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

/// Whether the user asked for less motion. Script-driven animations must
/// check this themselves; the CSS media query doesn't reach them.
pub fn reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| method(&w, "matchMedia")?.call1(&w, &"(prefers-reduced-motion: reduce)".into()).ok())
        .and_then(|mq| Reflect::get(&mq, &"matches".into()).ok())
        .is_some_and(|m| m.is_truthy())
}

/// Runs a Web Animations API animation on `el` (`keyframes` is a JSON array),
/// calling `on_finish` when it ends. Without the API, `on_finish` runs at once.
pub fn animate(el: &web_sys::Element, keyframes: &str, ms: f64, easing: &str, on_finish: Option<Box<dyn FnOnce()>>) {
    animate_part(el, None, false, keyframes, ms, easing, on_finish)
}

/// Like [`animate`], optionally on a pseudo-element of `el` such as
/// `::backdrop`. With `hold`, the end state stays applied after it finishes
/// (until the element is closed or removed); leave it off for anything whose
/// inline style changes later, or the held frame will override it.
pub fn animate_part(
    el: &web_sys::Element,
    pseudo: Option<&str>,
    hold: bool,
    keyframes: &str,
    ms: f64,
    easing: &str,
    on_finish: Option<Box<dyn FnOnce()>>,
) {
    let anim = (|| {
        let frames = js_sys::JSON::parse(keyframes).ok()?;
        let opts = js_sys::Object::new();
        Reflect::set(&opts, &"duration".into(), &ms.into()).ok()?;
        Reflect::set(&opts, &"easing".into(), &easing.into()).ok()?;
        if hold {
            Reflect::set(&opts, &"fill".into(), &"forwards".into()).ok()?;
        }
        if let Some(p) = pseudo {
            Reflect::set(&opts, &"pseudoElement".into(), &p.into()).ok()?;
        }
        method(el, "animate")?.call2(el, &frames, &opts).ok()
    })();
    match (anim, on_finish) {
        (Some(anim), Some(f)) => {
            let _ = Reflect::set(&anim, &"onfinish".into(), &Closure::once_into_js(f));
        }
        (None, Some(f)) => f(),
        _ => {}
    }
}

/// Cancels every running or held animation on `el`, pseudo-elements included.
pub fn cancel_animations(el: &web_sys::Element) {
    let opts = js_sys::Object::new();
    let _ = Reflect::set(&opts, &"subtree".into(), &true.into());
    let Some(list) = method(el, "getAnimations").and_then(|f| f.call1(el, &opts).ok()) else { return };
    for anim in js_sys::Array::from(&list).iter() {
        if let Some(cancel) = method(&anim, "cancel") {
            let _ = cancel.call0(&anim);
        }
    }
}

/// A custom property's computed value on `el`, e.g. `--lift`.
pub fn css_var(el: &web_sys::Element, name: &str) -> Option<String> {
    let w = web_sys::window()?;
    let style = method(&w, "getComputedStyle")?.call1(&w, el).ok()?;
    let v = method(&style, "getPropertyValue")?.call1(&style, &name.into()).ok()?.as_string()?;
    let v = v.trim();
    (!v.is_empty()).then(|| v.to_string())
}

/// The `--ease` curve from app.css, for script-driven animations.
pub const EASE: &str = "cubic-bezier(0.2, 0.8, 0.2, 1)";

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
