use dart_core::{Dart, ModeKind, Mult, Segment};
use leptos::prelude::*;

/// Score entry: pick a ring (defaults to single, resets after every dart),
/// then tap where the dart landed.
#[component]
pub fn Pad(
    kind: ModeKind,
    mult: RwSignal<Mult>,
    #[prop(into)] on_dart: Callback<Dart>,
    #[prop(into)] on_undo: Callback<()>,
    #[prop(into)] can_undo: Signal<bool>,
) -> impl IntoView {
    let numbers: Vec<u8> = match kind {
        ModeKind::X01 => (1..=20).collect(),
        ModeKind::RoundRobinhood => (15..=20).collect(),
    };
    let fire = move |segment: Segment| {
        if let Some(d) = Dart::new(segment, mult.get_untracked()) {
            mult.set(Mult::Single);
            on_dart.run(d);
        }
    };
    let ring = move |m: Mult, label: &'static str| {
        view! {
            <button
                type="button"
                role="radio"
                class=format!("segment ring-{}", label.to_lowercase())
                aria-checked=move || (mult.get() == m).to_string()
                on:click=move |_| mult.update(|cur| *cur = if *cur == m { Mult::Single } else { m })
            >
                {label}
            </button>
        }
    };
    let mult_class = move || match mult.get() {
        Mult::Single => "pad",
        Mult::Double => "pad pad-double",
        Mult::Triple => "pad pad-triple",
    };

    view! {
        <div class=mult_class>
            <div class="segmented rings" role="radiogroup" aria-label="Ring">
                {ring(Mult::Single, "Single")}
                {ring(Mult::Double, "Double")}
                {ring(Mult::Triple, "Triple")}
            </div>
            <div class=format!("keys keys-{}", kind.slug())>
                {numbers
                    .into_iter()
                    .map(|n| {
                        view! {
                            <button type="button" class="key" on:click=move |_| fire(Segment::Num(n))>
                                {n}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <div class="keys keys-extra">
                <button
                    type="button"
                    class="key key-bull"
                    disabled=move || mult.get() == Mult::Triple
                    on:click=move |_| fire(Segment::Bull)
                >
                    "Bull"
                    <small>{move || if mult.get() == Mult::Double { "50" } else { "25" }}</small>
                </button>
                <button type="button" class="key key-miss" on:click=move |_| fire(Segment::Miss)>
                    "Miss"
                </button>
                <button
                    type="button"
                    class="key key-undo"
                    disabled=move || !can_undo.get()
                    on:click=move |_| on_undo.run(())
                >
                    "Undo"
                </button>
            </div>
        </div>
    }
}
