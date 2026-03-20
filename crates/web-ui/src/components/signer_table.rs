use leptos::prelude::*;

use crate::api::StatusSigner;

#[component]
pub fn SignerTable(signers: Signal<Vec<StatusSigner>>) -> impl IntoView {
    view! {
        <div class="wobbly-md border-[3px] border-pencil bg-white p-6 shadow-hard rotate-[0.3deg]">
            <h3 class="font-kalam text-xl text-pencil mb-4">"Signers"</h3>
            <div class="overflow-x-auto">
                <table class="w-full font-hand text-pencil">
                    <thead>
                        <tr class="border-b-[3px] border-dashed border-pencil">
                            <th class="text-left py-2 px-3">"ID"</th>
                            <th class="text-left py-2 px-3">"Nostr Public Key"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            signers
                                .get()
                                .into_iter()
                                .map(|signer| {
                                    let npub_short = if signer.npub.len() > 20 {
                                        format!("{}...{}", &signer.npub[..12], &signer.npub[signer.npub.len()-8..])
                                    } else {
                                        signer.npub.clone()
                                    };
                                    view! {
                                        <tr class="border-b-2 border-dashed border-pencil/20 hover:bg-erased/30 transition-colors">
                                            <td class="py-2 px-3 font-kalam">{signer.signer_id}</td>
                                            <td class="py-2 px-3 font-mono text-sm" title=signer.npub.clone()>
                                                {npub_short}
                                            </td>
                                        </tr>
                                    }
                                })
                                .collect_view()
                        }}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
