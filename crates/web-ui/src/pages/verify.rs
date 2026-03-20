use leptos::prelude::*;

use crate::components::icons::{CheckCircleIcon, ShieldCheckIcon, XCircleIcon};
use crate::components::token_input::TokenInput;

#[component]
pub fn VerifyPage() -> impl IntoView {
    let (token_text, set_token_text) = signal(String::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(Option::<String>::None);
    let (result, set_result) = signal(Option::<bool>::None);

    let disabled = Signal::derive(move || loading.get());

    let verify = move |_| {
        let text = token_text.get();
        if text.trim().is_empty() {
            set_error.set(Some("Please paste or upload a token".into()));
            return;
        }

        let _token: common::TimestampToken = match serde_json::from_str(&text) {
            Ok(t) => t,
            Err(e) => {
                set_error.set(Some(format!("Invalid token JSON: {e}")));
                return;
            }
        };

        set_loading.set(true);
        set_error.set(None);
        set_result.set(None);

        #[cfg(feature = "hydrate")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                match crate::api::client::post_verify(&_token).await {
                    Ok(resp) => {
                        set_result.set(Some(resp.valid));
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
            // Hero
            <div class="text-center space-y-3 rotate-[0.3deg]">
                <h1 class="font-kalam text-4xl md:text-5xl text-pencil">
                    "Verify a Token"
                </h1>
                <p class="font-hand text-xl text-pencil/60 max-w-2xl mx-auto">
                    "Check if a timestamp token's signature is valid against the group public key."
                </p>
            </div>

            // Token input
            <TokenInput value=token_text set_value=set_token_text disabled=disabled />

            // Verify button
            <div class="flex justify-center">
                <button
                    class="btn-hand-secondary text-xl flex items-center gap-3 px-8 py-4"
                    on:click=verify
                    disabled=move || loading.get() || token_text.get().trim().is_empty()
                >
                    <ShieldCheckIcon size="24" />
                    {move || if loading.get() { "Verifying..." } else { "Verify Token" }}
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

            // Result display (speech bubble style)
            {move || {
                result.get().map(|valid| {
                    let (icon_view, label, color_class) = if valid {
                        (
                            view! { <CheckCircleIcon size="32" /> }.into_any(),
                            "Valid!",
                            "border-green-600 bg-green-50 text-green-700",
                        )
                    } else {
                        (
                            view! { <XCircleIcon size="32" /> }.into_any(),
                            "Invalid!",
                            "border-marker bg-marker/10 text-marker",
                        )
                    };

                    view! {
                        <div class="flex justify-center">
                            <div class=format!(
                                "wobbly-md border-[3px] p-6 shadow-hard -rotate-1 flex items-center gap-4 {color_class}"
                            )>
                                {icon_view}
                                <span class="font-kalam text-2xl">{label}</span>
                            </div>
                        </div>
                    }
                })
            }}
        </div>
    }
}
