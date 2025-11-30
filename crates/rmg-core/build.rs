// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Generate canonical rule ids (domain-separated) for zero-CPU runtime.
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest = out_dir.join("rule_ids.rs");

    // Motion rule id: blake3("rule:motion/update")
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"rule:motion/update");
    let bytes: [u8; 32] = hasher.finalize().into();

    let generated = format!(
        "/// Canonical family id for `rule:motion/update` (BLAKE3).\npub const MOTION_UPDATE_FAMILY_ID: [u8; 32] = {:?};\n",
        bytes
    );
    fs::write(dest, generated).expect("write rule_ids.rs");
}
