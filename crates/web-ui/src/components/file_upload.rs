use leptos::prelude::*;

use crate::components::icons::UploadIcon;

#[component]
pub fn FileUpload(
    on_hash: Callback<String>,
    #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
    let (drag_over, set_drag_over) = signal(false);
    let (file_name, set_file_name) = signal(Option::<String>::None);
    let (hashing, set_hashing) = signal(false);

    let _hash_file = move |file_name_str: String, data: Vec<u8>| {
        set_hashing.set(true);
        set_file_name.set(Some(file_name_str));
        // Hash with SHA-256
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = hex::encode(hasher.finalize());
        on_hash.run(hash);
        set_hashing.set(false);
    };

    #[cfg(feature = "hydrate")]
    let handle_file_input = {
        let hash_file = _hash_file.clone();
        move |ev: leptos::ev::Event| {
            use wasm_bindgen::JsCast;
            let target = ev.target().unwrap();
            let input: web_sys::HtmlInputElement = target.unchecked_into();
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    let name = file.name();
                    let hash_file = hash_file.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        let array_buffer =
                            wasm_bindgen_futures::JsFuture::from(file.array_buffer())
                                .await
                                .unwrap();
                        let data = js_sys::Uint8Array::new(&array_buffer).to_vec();
                        hash_file(name, data);
                    });
                }
            }
        }
    };

    #[cfg(not(feature = "hydrate"))]
    let handle_file_input = move |_ev: leptos::ev::Event| {};

    #[cfg(feature = "hydrate")]
    let handle_drop = {
        let hash_file = _hash_file.clone();
        move |ev: leptos::ev::DragEvent| {
            ev.prevent_default();
            set_drag_over.set(false);
            if let Some(dt) = ev.data_transfer() {
                if let Some(files) = dt.files() {
                    if let Some(file) = files.get(0) {
                        let name = file.name();
                        let hash_file = hash_file.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let array_buffer =
                                wasm_bindgen_futures::JsFuture::from(file.array_buffer())
                                    .await
                                    .unwrap();
                            let data = js_sys::Uint8Array::new(&array_buffer).to_vec();
                            hash_file(name, data);
                        });
                    }
                }
            }
        }
    };

    #[cfg(not(feature = "hydrate"))]
    let handle_drop = move |_ev: leptos::ev::DragEvent| {};

    view! {
        <div
            class=move || {
                let base = "wobbly border-[3px] border-dashed p-8 md:p-12 text-center transition-all duration-100 cursor-pointer tack-decoration";
                if drag_over.get() {
                    format!("{base} border-marker bg-marker/5")
                } else {
                    format!("{base} border-pencil bg-white hover:border-pen")
                }
            }
            on:dragover=move |ev: leptos::ev::DragEvent| {
                ev.prevent_default();
                set_drag_over.set(true);
            }
            on:dragleave=move |_| set_drag_over.set(false)
            on:drop=handle_drop
        >
            <label class="flex flex-col items-center gap-4 cursor-pointer">
                <div class="text-pencil/40">
                    <UploadIcon size="48" />
                </div>
                <div class="font-hand text-xl text-pencil/60">
                    {move || {
                        if hashing.get() {
                            "Hashing...".to_string()
                        } else if let Some(name) = file_name.get() {
                            name
                        } else {
                            "Drop a file here or click to browse".to_string()
                        }
                    }}
                </div>
                <input
                    type="file"
                    class="hidden"
                    on:change=handle_file_input
                    disabled=disabled
                />
            </label>
        </div>
    }
}
