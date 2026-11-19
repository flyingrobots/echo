// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The source side of an authored effect program's lowering artifacts.
//!
//! This module owns the runtime's rule for an authenticated source closure
//! (`edict.source-closure/v1`): every `use module` pin names a retained
//! module by its exact `edict.source/v1` identity, and the pins from the
//! entry module reach every retained module. A single source
//! (`edict.source/v1`) retains no modules, so any pin in it refuses.
//!
//! The compiler links source-module helpers into the one Core module, so the
//! runtime judges and runs them exactly as it does the entry module's own.
use crate::edict_pure::{
    values::{field, map},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::{BTreeMap, BTreeSet};

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
    pub(super) fn new(source: &'a Value, root: &str) -> Result<Self, Error> {
        if let Value::Bytes(text) = source {
            return Ok(Self {
                root: std::str::from_utf8(text).map_err(|_| Error::InvalidArtifact)?,
                modules: BTreeMap::new(),
            });
        }
        let root_text = echo_edict_canonical::source_closure::source_closure_root_v1(source, root)
            .map_err(|_| Error::InvalidArtifact)?;
        let mut modules = BTreeMap::new();
        for (coordinate, unit) in map(field(source, "modules")?)? {
            let Value::Text(coordinate) = coordinate else {
                return Err(Error::InvalidArtifact);
            };
            if coordinate == root {
                continue;
            }
            let (Value::Bytes(text), Value::Array(digest)) =
                (field(unit, "source")?, field(unit, "digest")?)
            else {
                return Err(Error::InvalidArtifact);
            };
            let [_, Value::Bytes(digest)] = digest.as_slice() else {
                return Err(Error::InvalidArtifact);
            };
            let text = std::str::from_utf8(text).map_err(|_| Error::InvalidArtifact)?;
            modules.insert(
                coordinate.as_str(),
                (text, format!("\"sha256:{}\"", hex::encode(digest))),
            );
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
        pins: impl Fn(&'a str, &'a str) -> Result<Vec<Pin<'a>>, Error>,
    ) -> Result<(), Error> {
        let mut reached = BTreeSet::new();
        let mut pending = root_pins;
        while let Some((coordinate, digest)) = pending.pop() {
            let (text, identity) = self.modules.get(coordinate).ok_or(Error::InvalidArtifact)?;
            if digest != identity {
                return Err(Error::InvalidArtifact);
            }
            if reached.insert(coordinate) {
                pending.extend(pins(coordinate, text)?);
            }
        }
        if reached.len() == self.modules.len() {
            Ok(())
        } else {
            Err(Error::InvalidArtifact)
        }
    }
}
