use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::components::footer::Footer;
use crate::components::navbar::Navbar;
use crate::pages::admin::AdminPage;
use crate::pages::signing::SigningPage;
use crate::pages::verify::VerifyPage;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/web-ui.css" />
        <Title text="Threshold Timestamp Server" />
        <Meta
            name="description"
            content="FROST threshold signature timestamp authority"
        />
        <Link
            rel="preconnect"
            href="https://fonts.googleapis.com"
        />
        <Link
            rel="preconnect"
            href="https://fonts.gstatic.com"
            crossorigin="anonymous"
        />
        <Link
            href="https://fonts.googleapis.com/css2?family=Kalam:wght@700&family=Patrick+Hand&display=swap"
            rel="stylesheet"
        />

        <Router>
            <div class="min-h-screen flex flex-col">
                <Navbar />
                <main class="flex-1 max-w-5xl mx-auto w-full px-6 py-10">
                    <Routes fallback=|| view! { <p class="font-hand text-xl text-pencil">"Page not found!"</p> }>
                        <Route path=path!("/") view=SigningPage />
                        <Route path=path!("/verify") view=VerifyPage />
                        <Route path=path!("/admin") view=AdminPage />
                    </Routes>
                </main>
                <Footer />
            </div>
        </Router>
    }
}
