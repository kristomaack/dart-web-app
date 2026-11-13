//! Drag-to-reorder player list. The row tracks the finger 1:1 from where it
//! was grabbed, neighbours slide aside, and the drop lands where it looks like
//! it will. Past either end the row rubber-bands instead of stopping dead.
//! A flick carries the row on (momentum projection), and on release every row
//! glides from where it was drawn to its new slot instead of jumping.

use crate::platform::{self, haptic};
use dart_core::Player;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, PartialEq)]
struct Drag {
    from: usize,
    start_y: f64,
    dy: f64,
    row_h: f64,
    len: usize,
    /// Pointer velocity in px/ms, for projecting a flick.
    vel: f64,
    last_y: f64,
    last_t: f64,
}

/// Where momentum would carry the row (Apple's scroll-deceleration
/// projection, at a snappy rate so a list of a few rows isn't overshot).
fn project(vel_px_per_ms: f64) -> f64 {
    const DECEL: f64 = 0.99;
    vel_px_per_ms * DECEL / (1.0 - DECEL)
}

impl Drag {
    fn target(&self) -> usize {
        self.target_at(self.dy)
    }

    fn target_at(&self, dy: f64) -> usize {
        let shift = (dy / self.row_h).round() as isize;
        (self.from as isize + shift).clamp(0, self.len as isize - 1) as usize
    }

    /// Visual offset for the row at `idx`.
    fn offset(&self, idx: usize) -> f64 {
        if idx == self.from {
            return self.dy;
        }
        let t = self.target();
        match () {
            _ if self.from < t && idx > self.from && idx <= t => -self.row_h,
            _ if t < self.from && idx >= t && idx < self.from => self.row_h,
            _ => 0.0,
        }
    }
}

fn rubberband(overshoot: f64, dimension: f64) -> f64 {
    const C: f64 = 0.55;
    overshoot * dimension * C / (dimension + C * overshoot.abs())
}

fn rows(list: &web_sys::Element) -> Vec<(String, web_sys::Element)> {
    let children = list.children();
    (0..children.length())
        .filter_map(|i| children.item(i))
        .filter_map(|el| Some((el.get_attribute("data-id")?, el)))
        .collect()
}

/// Where each row is drawn right now, transforms included.
fn row_tops(list: &web_sys::Element) -> Vec<(String, f64)> {
    rows(list).into_iter().map(|(id, el)| (id, el.get_bounding_client_rect().top())).collect()
}

fn settle(list: &web_sys::Element, before: &[(String, f64)], dropped: &str) {
    let lift = platform::css_var(list, "--lift").unwrap_or_else(|| "0 8px 28px rgba(0,0,0,0.18)".into());
    for (id, el) in rows(list) {
        let Some(&(_, top)) = before.iter().find(|(b, _)| *b == id) else { continue };
        let delta = top - el.get_bounding_client_rect().top();
        let frames = if id == dropped {
            serde_json::json!([
                { "transform": format!("translate3d(0,{delta:.1}px,0) scale(1.02)"), "boxShadow": lift, "zIndex": 2 },
                { "transform": "none", "boxShadow": "0 0 0 rgba(0,0,0,0)", "zIndex": 2 },
            ])
        } else if delta.abs() > 0.5 {
            serde_json::json!([{ "transform": format!("translate3d(0,{delta:.1}px,0)") }, { "transform": "none" }])
        } else {
            continue;
        };
        platform::animate(&el, &frames.to_string(), 260.0, platform::EASE, None);
    }
}

#[component]
pub fn ReorderList(players: RwSignal<Vec<Player>>) -> impl IntoView {
    let drag = RwSignal::new(None::<Drag>);
    let index_of = move |id: &str| players.with(|v| v.iter().position(|p| p.id == id));

    let row = move |p: Player| {
        let id = p.id.clone();
        let idx = {
            let id = id.clone();
            move || index_of(&id).unwrap_or(0)
        };
        let idx2 = idx.clone();
        let idx3 = idx.clone();
        let idx4 = idx.clone();
        let remove_id = id.clone();

        let on_down = move |ev: web_sys::PointerEvent| {
            ev.prevent_default();
            let Some(handle) = ev.current_target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else {
                return;
            };
            let _ = handle.set_pointer_capture(ev.pointer_id());
            let row_h = handle
                .closest("li")
                .ok()
                .flatten()
                .map(|li| li.get_bounding_client_rect().height())
                .unwrap_or(56.0);
            drag.set(Some(Drag {
                from: idx(),
                start_y: ev.client_y() as f64,
                dy: 0.0,
                row_h,
                len: players.with_untracked(Vec::len),
                vel: 0.0,
                last_y: ev.client_y() as f64,
                last_t: ev.time_stamp(),
            }));
            platform::vibrate(haptic::TICK);
        };
        let on_move = move |ev: web_sys::PointerEvent| {
            let Some(mut d) = drag.get_untracked() else { return };
            let (y, t) = (ev.client_y() as f64, ev.time_stamp());
            if t > d.last_t {
                // Lightly smoothed so one jittery event doesn't decide a flick.
                d.vel = 0.6 * (y - d.last_y) / (t - d.last_t) + 0.4 * d.vel;
                d.last_y = y;
                d.last_t = t;
            }
            let raw = y - d.start_y;
            let min = -(d.from as f64) * d.row_h;
            let max = (d.len - 1 - d.from) as f64 * d.row_h;
            d.dy = if raw < min {
                min + rubberband(raw - min, d.row_h * 2.0)
            } else if raw > max {
                max + rubberband(raw - max, d.row_h * 2.0)
            } else {
                raw
            };
            let before = drag.get_untracked().map(|o| o.target());
            if before != Some(d.target()) {
                platform::vibrate(haptic::TICK);
            }
            drag.set(Some(d));
        };
        let on_up = move |ev: web_sys::PointerEvent| {
            let Some(d) = drag.get_untracked() else { return };
            // No flick if the pointer was cancelled or had come to rest.
            let resting = ev.time_stamp() - d.last_t > 80.0;
            let fling = if ev.type_() == "pointercancel" || resting { 0.0 } else { project(d.vel) };
            let to = d.target_at(d.dy + fling);
            let list = ev
                .current_target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .and_then(|h| h.closest("ul").ok().flatten());
            let before = list.as_ref().map(row_tops).unwrap_or_default();
            if to != d.from {
                players.update(|v| {
                    let p = v.remove(d.from);
                    v.insert(to, p);
                });
            }
            drag.set(None);
            let (Some(list), false) = (list, platform::reduced_motion()) else { return };
            let dropped = players.with_untracked(|v| v[to].id.clone());
            // After the DOM has the new order but before it paints: start each
            // row where it was drawn and let it glide home.
            request_animation_frame(move || settle(&list, &before, &dropped));
        };

        view! {
            <li
                class="row reorder-row"
                class:lifted=move || drag.with(|d| d.is_some_and(|d| d.from == idx2()))
                data-id=id.clone()
                style:transform=move || {
                    drag.with(|d| {
                        d.map(|d| {
                            let i = idx3();
                            let lift = if d.from == i { " scale(1.02)" } else { "" };
                            format!("translate3d(0,{:.1}px,0){lift}", d.offset(i))
                        })
                    })
                    .unwrap_or_default()
                }
            >
                <span class="seat">{move || idx4() + 1}</span>
                <span class="row-title grow">{p.name.clone()}</span>
                <button
                    type="button"
                    class="icon-btn remove"
                    aria-label=format!("Remove {}", p.name)
                    on:click=move |_| players.update(|v| v.retain(|x| x.id != remove_id))
                >
                    "−"
                </button>
                <span
                    class="handle"
                    aria-label="Drag to reorder"
                    on:pointerdown=on_down
                    on:pointermove=on_move
                    on:pointerup=on_up
                    on:pointercancel=on_up
                >
                    <span></span>
                    <span></span>
                    <span></span>
                </span>
            </li>
        }
    };

    view! {
        <ul class="list reorder" class:dragging=move || drag.with(Option::is_some)>
            <For each=move || players.get() key=|p| p.id.clone() children=row/>
        </ul>
    }
}
