// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Spec-000 scaffold: Leptos CSR app wired for trunk/wasm32.

use leptos::*;
use wasm_bindgen::prelude::*;

/// Top-level Spec-000 Leptos component (WASM).
#[allow(missing_docs)]
#[component]
pub fn App() -> impl IntoView {
    let (epoch, set_epoch) = create_signal(0usize);

    view! {
        <div class="container">
            <h1>"Spec-000: Everything Is a Rewrite"</h1>
            <p class="subtitle">
                "Living spec harness — the same Rust compiled to native and WASM."
            </p>

            <div class="panel">
                <div class="metric">
                    <span class="label">"Current Epoch:"</span>
                    <span class="value">{move || epoch.get()}</span>
                </div>
                <button on:click=move |_| set_epoch.update(|n| *n += 1)>
                    "Commit Next Epoch"
                </button>
            </div>

            <div class="note">
                "Hook this component to the real kernel bindings to drive rewrites, "
                "render the RMG graph, and record completion hashes."
            </div>
        </div>
    }
}

/// WASM entry point required by `trunk serve`.
#[allow(missing_docs)]
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(|| view! { <App/> })
}
