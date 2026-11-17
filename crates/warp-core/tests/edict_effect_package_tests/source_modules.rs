// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Effect programs whose helpers come from digest-pinned source modules
//! (flyingrobots/echo#1009).
//!
//! The control is the compiler's package for
//! `fixtures/edict-effect-source-module/source`: an `ensureNode` effect, then
//! `leaf.same(input.v)` from the module `jedit.probe.leaf@1`, which the entry
//! module pins by digest. The compiler links that helper into the one Core
//! module; the lowering artifacts retain the `edict.source-closure/v1`
//! envelope of both modules.
//!
//! Each forgery keeps the envelope internally valid (every module's digest
//! is its own source identity), so only the `use module` pins refuse it.
use super::*;
use echo_edict_canonical::digest_canonical_value_bytes_v1 as source_digest;

const PACKAGE: &str = include_str!(
    "../fixtures/edict-effect-source-module/source-module-executable-operation-package.hex"
);
const ENTRY: &str = "consumer.identity.program@1";
const LEAF: &str = "jedit.probe.leaf@1";

/// One edit of the source-module control that every judge must refuse, or none.
#[derive(Clone, Copy)]
pub(super) enum Forgery {
    /// The control itself.
    None,
    /// The retained leaf is another source than the one the entry pins.
    Substituted,
    /// The entry module no longer pins the leaf the closure retains.
    Unpinned,
}

fn decode(raw: &V) -> V {
    let V::Bytes(raw) = raw else { panic!("bytes") };
    echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("canonical")
}

/// One closure module, carrying its exact `edict.source/v1` identity.
fn unit(source: &str) -> V {
    let source = V::Bytes(source.as_bytes().to_vec());
    let identity = source_digest("edict.source/v1", &source).expect("source identity");
    map([
        (
            "digest",
            V::Array(vec![text("sha256"), V::Bytes(identity.to_vec())]),
        ),
        ("source", source),
    ])
}

fn module_source(modules: &V, coordinate: &str) -> String {
    let V::Bytes(source) = get(get(modules, coordinate), "source") else {
        panic!("module source")
    };
    String::from_utf8(source.clone()).expect("UTF-8 source")
}

/// The retained compiler package.
pub(super) fn source_module_package() -> V {
    echo_edict_canonical::decode_canonical_cbor_v1(
        &hex::decode(PACKAGE.trim()).expect("retained hex"),
    )
    .expect("canonical package")
}

/// The compiler's source-module package under `forgery`.
pub(super) fn source_module_program(forgery: Forgery) -> V {
    let mut package = source_module_package();
    let mut program = decode(get(&package, "program"));
    let mut bundle = decode(get(&program, "compiled_program"));
    let lowering = value_field_mut(&mut bundle, "lowering_artifacts");
    let mut closure = decode(get(lowering, "source_artifact"));
    let modules = value_field_mut(&mut closure, "modules");
    match forgery {
        Forgery::None => {}
        Forgery::Substituted => {
            let leaf = module_source(modules, LEAF).replace("return v;", "return 0u64;");
            set(modules, LEAF, unit(&leaf));
        }
        Forgery::Unpinned => {
            let entry = module_source(modules, ENTRY)
                .lines()
                .filter(|line| !line.starts_with("use module "))
                .collect::<Vec<_>>()
                .join("\n");
            set(modules, ENTRY, unit(&entry));
        }
    }
    set(lowering, "source_artifact", V::Bytes(encoded(&closure)));
    set(&mut program, "compiled_program", V::Bytes(encoded(&bundle)));
    set(&mut package, "program", V::Bytes(encoded(&program)));
    package
}
