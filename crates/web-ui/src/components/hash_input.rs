use leptos::prelude::*;

#[component]
pub fn HashInput(
    value: ReadSignal<String>,
    set_value: WriteSignal<String>,
    #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <label class="font-hand text-lg text-pencil/80">"Or paste a SHA-256 hash:"</label>
            <input
                type="text"
                class="wobbly w-full border-[3px] border-pencil bg-white px-4 py-3 font-hand text-lg text-pencil placeholder:text-pencil/30 focus:border-pen focus:ring-2 focus:ring-pen/20 focus:outline-none transition-colors duration-100"
                placeholder="e.g. a1b2c3d4e5f6..."
                maxlength="64"
                prop:value=value
                on:input=move |ev| {
                    set_value.set(event_target_value(&ev));
                }
                disabled=disabled
            />
        </div>
    }
}
