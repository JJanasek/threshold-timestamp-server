use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="mt-auto">
            <div class="max-w-5xl mx-auto px-6">
                <div class="dashed-divider" />
                <div class="py-8 flex flex-col md:flex-row items-center justify-between gap-4">
                    <p class="font-hand text-pencil/60 text-lg">
                        "FROST Threshold Timestamp Authority"
                    </p>
                    <div class="flex items-center gap-6 font-hand text-pencil/60">
                        <span class="wavy-underline cursor-default">"PV204"</span>
                        <span>"k-of-n signatures"</span>
                    </div>
                </div>
            </div>
        </footer>
    }
}
