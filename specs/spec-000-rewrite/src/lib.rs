// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Spec-000 scaffold: Leptos CSR app wired for trunk/wasm32.

use leptos::*;

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
use wasm_bindgen::prelude::*;

mod spec_content;

/// Top-level Spec-000 Leptos component (WASM).
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

            <div class="panel" style="margin-top: 20px;">
                <h2>"SPEC-000: Everything Is a Rewrite"</h2>
                <pre class="spec">{spec_content::SPEC_MD}</pre>
            </div>
        </div>
    }
}

/// WASM entry point required by `trunk serve`.
#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(|| view! { <App/> })
}
