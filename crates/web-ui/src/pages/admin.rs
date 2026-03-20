use leptos::prelude::*;

use crate::api::StatusResponse;
use crate::components::icons::ServerIcon;
use crate::components::signer_table::SignerTable;
use crate::components::status_card::StatusCard;

#[component]
pub fn AdminPage() -> impl IntoView {
    let (status, _set_status) = signal(Option::<StatusResponse>::None);
    let (error, _set_error) = signal(Option::<String>::None);
    let (loading, _set_loading) = signal(true);

    // Fetch status on mount
    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                match crate::api::client::get_status().await {
                    Ok(s) => {
                        _set_status.set(Some(s));
                    }
                    Err(e) => {
                        _set_error.set(Some(e));
                    }
                }
                _set_loading.set(false);
            });
        });
    }

    let health_value = Signal::derive(move || {
        status
            .get()
            .map(|s| {
                if s.healthy {
                    "Healthy".to_string()
                } else {
                    "Unhealthy".to_string()
                }
            })
            .unwrap_or_else(|| "...".to_string())
    });

    let threshold_value = Signal::derive(move || {
        status
            .get()
            .map(|s| format!("{}-of-{}", s.k, s.n))
            .unwrap_or_else(|| "...".to_string())
    });

    let sessions_value = Signal::derive(move || {
        status
            .get()
            .map(|s| s.active_sessions.to_string())
            .unwrap_or_else(|| "...".to_string())
    });

    let group_key_value = Signal::derive(move || {
        status
            .get()
            .map(|s| {
                if s.group_public_key.len() > 16 {
                    format!("{}...", &s.group_public_key[..16])
                } else {
                    s.group_public_key.clone()
                }
            })
            .unwrap_or_else(|| "...".to_string())
    });

    let signers_signal = Signal::derive(move || {
        status
            .get()
            .map(|s| s.signers)
            .unwrap_or_default()
    });

    let relay_urls = Signal::derive(move || {
        status
            .get()
            .map(|s| s.relay_urls)
            .unwrap_or_default()
    });

    view! {
        <div class="space-y-8">
            // Hero
            <div class="text-center space-y-3 -rotate-[0.3deg]">
                <h1 class="font-kalam text-4xl md:text-5xl text-pencil flex items-center justify-center gap-3">
                    <ServerIcon size="36" />
                    "Admin Dashboard"
                </h1>
                <p class="font-hand text-xl text-pencil/60">
                    "System overview and signer information"
                </p>
            </div>

            // Loading state
            {move || {
                if loading.get() {
                    Some(view! {
                        <div class="text-center py-12">
                            <p class="font-hand text-xl text-pencil/40 animate-pulse">"Loading status..."</p>
                        </div>
                    })
                } else {
                    None
                }
            }}

            // Error message
            {move || {
                error.get().map(|e| {
                    view! {
                        <div class="wobbly border-[3px] border-marker bg-marker/10 p-4 text-center">
                            <p class="font-hand text-lg text-marker">{e}</p>
                        </div>
                    }
                })
            }}

            // Status cards grid
            {move || {
                if status.get().is_some() {
                    Some(view! {
                        <div class="grid grid-cols-2 md:grid-cols-4 gap-4 md:gap-6">
                            <StatusCard label="Health" value=health_value rotation="rotate-[-1deg]" />
                            <StatusCard label="Threshold" value=threshold_value rotation="rotate-[0.5deg]" />
                            <StatusCard label="Active Sessions" value=sessions_value rotation="rotate-[1deg]" />
                            <StatusCard label="Group Key" value=group_key_value rotation="rotate-[-0.5deg]" />
                        </div>
                    })
                } else {
                    None
                }
            }}

            // Signer table
            {move || {
                if !signers_signal.get().is_empty() {
                    Some(view! { <SignerTable signers=signers_signal /> })
                } else {
                    None
                }
            }}

            // Relay URLs
            {move || {
                let urls = relay_urls.get();
                if !urls.is_empty() {
                    Some(view! {
                        <div class="wobbly border-[3px] border-pencil bg-white p-6 shadow-hard -rotate-[0.3deg]">
                            <h3 class="font-kalam text-xl text-pencil mb-3">"Relay URLs"</h3>
                            <ul class="space-y-1">
                                {urls
                                    .into_iter()
                                    .map(|url| {
                                        view! {
                                            <li class="font-mono text-sm text-pen">{url}</li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        </div>
                    })
                } else {
                    None
                }
            }}

            // Future: Signature activity logs placeholder
            {move || {
                if status.get().is_some() {
                    Some(view! {
                        <div class="wobbly-md border-[3px] border-dashed border-pencil/30 bg-white p-8 rotate-[0.2deg]">
                            <h3 class="font-kalam text-xl text-pencil/40 mb-4">"Signature Activity Log"</h3>
                            <div class="notebook-lines min-h-[160px] flex items-center justify-center">
                                <p class="font-hand text-lg text-pencil/30 italic">
                                    "Activity logging coming soon..."
                                </p>
                            </div>
                        </div>
                    })
                } else {
                    None
                }
            }}
        </div>
    }
}
