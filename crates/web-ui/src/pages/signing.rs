use leptos::prelude::*;

use crate::components::file_upload::FileUpload;
use crate::components::hash_input::HashInput;
use crate::components::icons::StampIcon;
use crate::components::token_display::TokenDisplay;

#[component]
pub fn SigningPage() -> impl IntoView {
    let (hash, set_hash) = signal(String::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (token_json, set_token_json) = signal(String::new());

    let on_file_hash = Callback::new(move |h: String| {
        set_hash.set(h);
        set_error.set(None);
        set_token_json.set(String::new());
    });

    let disabled = Signal::derive(move || loading.get());

    let submit = move |_| {
        let h = hash.get();
        if h.len() != 64 {
            set_error.set(Some("Hash must be exactly 64 hex characters (SHA-256)".into()));
            return;
        }
        if hex::decode(&h).is_err() {
            set_error.set(Some("Invalid hex string".into()));
            return;
        }

        set_loading.set(true);
        set_error.set(None);
        set_token_json.set(String::new());

        #[cfg(feature = "hydrate")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                match crate::api::client::post_timestamp(&h).await {
                    Ok(token) => {
                        let json = serde_json::to_string_pretty(&token).unwrap_or_default();
                        set_token_json.set(json);
                    }
                    Err(e) => {
                        set_error.set(Some(e));
                    }
                }
                set_loading.set(false);
            });
        }

        #[cfg(not(feature = "hydrate"))]
        {
            set_loading.set(false);
        }
    };

    view! {
        <div class="space-y-8">
            // Hero section
            <div class="text-center space-y-3 -rotate-[0.5deg]">
                <h1 class="font-kalam text-4xl md:text-5xl text-pencil">
                    "Timestamp a Document"
                </h1>
                <p class="font-hand text-xl text-pencil/60 max-w-2xl mx-auto">
                    "Upload a file or paste its SHA-256 hash to receive a threshold-signed timestamp proof."
                </p>
            </div>

            // Decorative hand-drawn arrow (desktop only)
            <div class="hidden md:flex justify-center -my-2">
                <svg width="60" height="40" viewBox="0 0 60 40" class="text-pencil/30">
                    <path d="M10 5 Q30 0 50 20 Q55 25 45 30" stroke="currentColor" stroke-width="2.5" fill="none" stroke-dasharray="4 4" />
                    <path d="M45 30 L50 22 L42 26" stroke="currentColor" stroke-width="2.5" fill="none" />
                </svg>
            </div>

            // Upload zone
            <FileUpload on_hash=on_file_hash disabled=disabled />

            // Divider
            <div class="flex items-center gap-4">
                <div class="flex-1 border-t-2 border-dashed border-pencil/20" />
                <span class="font-hand text-pencil/40 text-lg">"or"</span>
                <div class="flex-1 border-t-2 border-dashed border-pencil/20" />
            </div>

            // Manual hash input
            <HashInput value=hash set_value=set_hash disabled=disabled />

            // Submit button
            <div class="flex justify-center">
                <button
                    class="btn-hand text-xl flex items-center gap-3 px-8 py-4"
                    on:click=submit
                    disabled=move || loading.get() || hash.get().is_empty()
                >
                    <StampIcon size="24" />
                    {move || if loading.get() { "Signing..." } else { "Request Timestamp" }}
                </button>
            </div>

            // Error message
            {move || {
                error.get().map(|e| {
                    view! {
                        <div class="wobbly border-[3px] border-marker bg-marker/10 p-4 text-center rotate-[0.5deg]">
                            <p class="font-hand text-lg text-marker">{e}</p>
                        </div>
                    }
                })
            }}

            // Token result
            {move || {
                let json = token_json.get();
                if json.is_empty() {
                    None
                } else {
                    Some(view! { <TokenDisplay token_json=token_json /> })
                }
            }}
        </div>
    }
}
