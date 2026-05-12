use dioxus::prelude::*;

const STYLE: Asset = asset!("/assets/style.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: STYLE }
        h1 { "hello with one asset!() under wasm-threads rustflags" }
    }
}
