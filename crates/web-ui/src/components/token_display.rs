use leptos::prelude::*;

use crate::components::icons::DownloadIcon;

#[component]
pub fn TokenDisplay(token_json: ReadSignal<String>) -> impl IntoView {
    #[cfg(feature = "hydrate")]
    let download_token = move |_| {
        use wasm_bindgen::JsCast;
        let json = token_json.get();
        if json.is_empty() {
            return;
        }
        let blob = web_sys::Blob::new_with_str_sequence_and_options(
            &js_sys::Array::of1(&json.into()),
            web_sys::BlobPropertyBag::new().type_("application/json"),
        )
        .unwrap();
        let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let a: web_sys::HtmlElement = document.create_element("a").unwrap().unchecked_into();
        a.set_attribute("href", &url).unwrap();
        a.set_attribute("download", "timestamp_token.tst").unwrap();
        a.click();
        web_sys::Url::revoke_object_url(&url).unwrap();
    };

    #[cfg(not(feature = "hydrate"))]
    let download_token = move |_: leptos::ev::MouseEvent| {};

    view! {
        <div class="wobbly-md border-[3px] border-pencil bg-white p-6 shadow-hard tape-decoration mt-8 rotate-[0.5deg]">
            <h3 class="font-kalam text-xl text-pencil mb-4 mt-2">"Signed Token"</h3>
            <pre class="font-mono text-sm text-pencil/80 bg-paper p-4 wobbly-sm border-2 border-pencil/20 overflow-x-auto whitespace-pre-wrap break-all">
                {move || token_json.get()}
            </pre>
            <div class="mt-4 flex justify-end">
                <button
                    class="btn-hand-secondary flex items-center gap-2 text-base"
                    on:click=download_token
                >
                    <DownloadIcon size="18" />
                    "Download .tst"
                </button>
            </div>
        </div>
    }
}
