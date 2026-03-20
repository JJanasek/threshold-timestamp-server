use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Navbar() -> impl IntoView {
    view! {
        <nav class="border-b-[3px] border-dashed border-pencil bg-paper/80 backdrop-blur-sm">
            <div class="max-w-5xl mx-auto px-6 py-4 flex items-center justify-between">
                <A href="/" attr:class="font-kalam text-2xl md:text-3xl text-pencil hover:text-marker transition-colors duration-100 -rotate-1">
                    "Timestamp Server"
                </A>

                <div class="flex items-center gap-6 md:gap-8 font-hand text-lg md:text-xl">
                    <A
                        href="/"
                        attr:class="text-pencil hover:text-marker wavy-underline transition-colors duration-100 hover:rotate-1"
                    >
                        "Sign"
                    </A>
                    <A
                        href="/verify"
                        attr:class="text-pencil hover:text-marker wavy-underline transition-colors duration-100 hover:-rotate-1"
                    >
                        "Verify"
                    </A>
                    <A
                        href="/admin"
                        attr:class="text-pencil hover:text-marker wavy-underline transition-colors duration-100 hover:rotate-1"
                    >
                        "Admin"
                    </A>
                </div>
            </div>
        </nav>
    }
}
