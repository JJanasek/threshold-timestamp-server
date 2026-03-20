use leptos::prelude::*;

#[component]
pub fn StatusCard(
    label: &'static str,
    value: Signal<String>,
    #[prop(default = "rotate-[-1deg]")] rotation: &'static str,
) -> impl IntoView {
    view! {
        <div class=format!(
            "wobbly border-[3px] border-pencil bg-[#fff9c4] p-5 shadow-hard transition-transform duration-100 hover:rotate-1 {rotation}"
        )>
            <p class="font-hand text-pencil/60 text-base mb-1">{label}</p>
            <p class="font-kalam text-2xl text-pencil">{value}</p>
        </div>
    }
}
