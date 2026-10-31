//! Drag-to-reorder player list. The row tracks the finger 1:1 from where it
//! was grabbed, neighbours slide aside, and the drop lands where it looks like
//! it will. Past either end the row rubber-bands instead of stopping dead.

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
}

impl Drag {
    fn target(&self) -> usize {
        let shift = (self.dy / self.row_h).round() as isize;
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
            }));
            platform::vibrate(haptic::TICK);
        };
        let on_move = move |ev: web_sys::PointerEvent| {
            let Some(mut d) = drag.get_untracked() else { return };
            let raw = ev.client_y() as f64 - d.start_y;
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
        let on_up = move |_ev: web_sys::PointerEvent| {
            let Some(d) = drag.get_untracked() else { return };
            let to = d.target();
            if to != d.from {
                players.update(|v| {
                    let p = v.remove(d.from);
                    v.insert(to, p);
                });
            }
            drag.set(None);
        };

        view! {
            <li
                class="row reorder-row"
                class:lifted=move || drag.with(|d| d.is_some_and(|d| d.from == idx2()))
                style:transform=move || {
                    drag.with(|d| d.map(|d| format!("translate3d(0,{:.1}px,0)", d.offset(idx3()))))
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
