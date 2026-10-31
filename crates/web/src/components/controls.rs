use leptos::prelude::*;

/// iOS-style segmented control.
#[component]
pub fn Segmented<T>(
    options: Vec<(T, &'static str)>,
    #[prop(into)] value: Signal<T>,
    #[prop(into)] on_change: Callback<T>,
) -> impl IntoView
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    view! {
        <div class="segmented" role="radiogroup">
            {options
                .into_iter()
                .map(|(opt, label)| {
                    let o = opt.clone();
                    view! {
                        <button
                            type="button"
                            role="radio"
                            class="segment"
                            aria-checked=move || (value.get() == o).to_string()
                            on:click=move |_| on_change.run(opt.clone())
                        >
                            {label}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
pub fn Toggle(
    label: &'static str,
    #[prop(optional)] detail: &'static str,
    #[prop(into)] value: Signal<bool>,
    #[prop(into)] on_change: Callback<bool>,
) -> impl IntoView {
    view! {
        <label class="row toggle-row">
            <div class="row-text">
                <span class="row-title">{label}</span>
                {(!detail.is_empty()).then(|| view! { <span class="row-sub">{detail}</span> })}
            </div>
            <input
                type="checkbox"
                role="switch"
                class="switch"
                prop:checked=move || value.get()
                on:change=move |ev| on_change.run(event_target_checked(&ev))
            />
        </label>
    }
}
