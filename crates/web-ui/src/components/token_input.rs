use leptos::prelude::*;

#[component]
pub fn TokenInput(
    value: ReadSignal<String>,
    set_value: WriteSignal<String>,
    #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
    #[cfg(feature = "hydrate")]
    let handle_file = move |ev: leptos::ev::Event| {
        use wasm_bindgen::JsCast;
        let target = ev.target().unwrap();
        let input: web_sys::HtmlInputElement = target.unchecked_into();
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let set_value = set_value.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let text = wasm_bindgen_futures::JsFuture::from(file.text())
                        .await
                        .unwrap();
                    if let Some(s) = text.as_string() {
                        set_value.set(s);
                    }
                });
            }
        }
    };

    #[cfg(not(feature = "hydrate"))]
    let handle_file = move |_ev: leptos::ev::Event| {};

    view! {
        <div class="space-y-3">
            <label class="font-hand text-lg text-pencil/80">"Paste token JSON or upload .tst file:"</label>
            <textarea
                class="wobbly w-full border-[3px] border-pencil bg-white px-4 py-3 font-hand text-base text-pencil placeholder:text-pencil/30 focus:border-pen focus:ring-2 focus:ring-pen/20 focus:outline-none transition-colors duration-100 min-h-[200px] resize-y"
                placeholder=r#"{"serial_number": 1, "timestamp": ..., "file_hash": "...", "signature": "...", "group_public_key": "..."}"#
                prop:value=value
                on:input=move |ev| {
                    set_value.set(event_target_value(&ev));
                }
                disabled=disabled
            />
            <div class="flex items-center gap-3">
                <label class="btn-hand-secondary text-base cursor-pointer flex items-center gap-2">
                    "Upload .tst file"
                    <input
                        type="file"
                        accept=".tst,.json"
                        class="hidden"
                        on:change=handle_file
                        disabled=disabled
                    />
                </label>
            </div>
        </div>
    }
}
