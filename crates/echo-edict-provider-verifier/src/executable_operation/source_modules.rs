// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The source side of an effect program's lowering artifacts.
//!
//! This module owns the rule that binds an authenticated source closure
//! (`edict.source-closure/v1`) to the modules it retains: every `use module`
//! pin names a retained module by its exact `edict.source/v1` identity, and
//! the pins from the entry module reach every retained module. A single
//! source (`edict.source/v1`) retains no modules, so any pin in it refuses.
//!
//! Helpers from source modules need no rule of their own: the compiler links
//! them into the one Core module, where they are judged as entry helpers are.
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::{BTreeMap, BTreeSet};

type Check<T> = Result<T, ()>;

/// One `use module` pin: a module coordinate and its quoted digest literal.
pub(super) type Pin<'a> = (&'a str, &'a str);

/// The retained sources: the entry module's text, and each other module's
/// text and quoted `"sha256:…"` identity.
pub(super) struct Sources<'a> {
    root: &'a str,
    modules: BTreeMap<&'a str, (&'a str, String)>,
}

impl<'a> Sources<'a> {
    /// Reads a source artifact: one source string, or a closure whose
    /// envelope and per-module identities are valid.
    pub(super) fn new(source: &'a Value, root: &str) -> Check<Self> {
        if let Value::Bytes(text) = source {
            let root = std::str::from_utf8(text).map_err(|_| ())?;
            return Ok(Self {
                root,
                modules: BTreeMap::new(),
            });
        }
        let root_text = echo_edict_canonical::source_closure::source_closure_root_v1(source, root)
            .map_err(|_| ())?;
        let mut modules = BTreeMap::new();
        for (coordinate, unit) in members(field(source, "modules")?)? {
            let Value::Text(coordinate) = coordinate else {
                return Err(());
            };
            if coordinate == root {
                continue;
            }
            let Value::Bytes(text) = field(unit, "source")? else {
                return Err(());
            };
            let Value::Array(digest) = field(unit, "digest")? else {
                return Err(());
            };
            let [_, Value::Bytes(digest)] = digest.as_slice() else {
                return Err(());
            };
            let text = std::str::from_utf8(text).map_err(|_| ())?;
            modules.insert(coordinate.as_str(), (text, quoted(digest)));
        }
        Ok(Self {
            root: root_text,
            modules,
        })
    }

    /// The entry module's source text.
    pub(super) const fn root(&self) -> &'a str {
        self.root
    }

    /// Requires every pin, from the entry module's `root_pins` onward, to name
    /// a retained module by its exact identity, and every retained module to
    /// be reached. `pins` reads one module's own pins from its text.
    pub(super) fn require_pinned(
        &self,
        root_pins: Vec<Pin<'a>>,
        pins: impl Fn(&'a str, &'a str) -> Check<Vec<Pin<'a>>>,
    ) -> Check<()> {
        let mut reached = BTreeSet::new();
        let mut pending = root_pins;
        while let Some((coordinate, digest)) = pending.pop() {
            let (text, identity) = self.modules.get(coordinate).ok_or(())?;
            if digest != identity {
                return Err(());
            }
            if reached.insert(coordinate) {
                pending.extend(pins(coordinate, text)?);
            }
        }
        if reached.len() == self.modules.len() {
            Ok(())
        } else {
            Err(())
        }
    }
}

fn quoted(digest: &[u8]) -> String {
    let mut text = String::with_capacity(9 + digest.len() * 2);
    text.push_str("\"sha256:");
    for byte in digest {
        text.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
        text.push(char::from(b"0123456789abcdef"[usize::from(byte & 0x0f)]));
    }
    text.push('"');
    text
}

fn field<'v>(value: &'v Value, name: &str) -> Check<&'v Value> {
    super::map_field(value, name).ok_or(())
}

fn members(value: &Value) -> Check<&[(Value, Value)]> {
    if let Value::Map(values) = value {
        Ok(values)
    } else {
        Err(())
    }
}
