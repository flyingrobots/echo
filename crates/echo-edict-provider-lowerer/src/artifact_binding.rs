// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns the canonical artifact binding check: a bound artifact's
//! reference is well formed, its bytes decode as `edict.canonical-cbor/v1`,
//! they are the value's canonical encoding, and its domain-framed digest is
//! the bound digest.
//!
//! A failed binding names the check that refused as a stable reason code
//! (`artifact-binding.<check>`, the scheme of the effect-program reason
//! codes), and the artifact's domain, so a refusal identifies which of a
//! request's bound artifacts failed and why.

use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_v1, CanonicalValueErrorKind, CanonicalValueV1,
};

use crate::{BoundArtifact, Diagnostic, DiagnosticSeverity, DigestAlgorithm, ProviderRefusalV1};

/// The binding check that refused, as a stable machine-readable code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BindingCheck {
    /// The reference has an empty coordinate or domain, or a digest that is
    /// not 32 SHA-256 bytes.
    Reference,
    /// The bytes are not one `edict.canonical-cbor/v1` value within the
    /// decoder's published bounds.
    Decode,
    /// The bytes decode, but are not the value's canonical encoding.
    CanonicalForm,
    /// The value's domain-framed digest is not the bound digest.
    Digest,
}

impl BindingCheck {
    /// The stable reason code. Codes never change meaning.
    const fn code(self) -> &'static str {
        match self {
            Self::Reference => "artifact-binding.reference",
            Self::Decode => "artifact-binding.decode",
            Self::CanonicalForm => "artifact-binding.canonical-form",
            Self::Digest => "artifact-binding.digest",
        }
    }
}

/// Why one bound artifact failed its canonical binding.
#[derive(Debug)]
pub(crate) struct BindingFailure {
    check: BindingCheck,
    detail: String,
}

impl BindingFailure {
    fn new(check: BindingCheck, detail: impl Into<String>) -> Self {
        Self {
            check,
            detail: detail.into(),
        }
    }

    /// Extend `refusal` with a diagnostic naming this failure.
    ///
    /// The added code extends the refusal's own first code, so the
    /// diagnostic list stays strictly ascending. The message names the
    /// artifact by domain; the refusal's subject already names it by
    /// coordinate or role.
    pub(crate) fn refuse(
        self,
        mut refusal: ProviderRefusalV1,
        bound: &BoundArtifact,
    ) -> ProviderRefusalV1 {
        let base = refusal
            .diagnostics
            .first()
            .map_or("echo.provider.invalid-semantic-artifact", |first| {
                first.code.as_str()
            });
        let code = format!("{base}.{}", self.check.code());
        refusal.diagnostics.push(Diagnostic {
            code,
            severity: DiagnosticSeverity::Error,
            message: format!(
                "{} artifact failed {}: {}",
                bound.artifact.domain,
                self.check.code(),
                self.detail
            ),
            repair: None,
        });
        refusal
    }
}

/// Check one bound artifact's canonical binding and return its value.
pub(crate) fn validate_binding(bound: &BoundArtifact) -> Result<CanonicalValueV1, BindingFailure> {
    if bound.reference.coordinate.is_empty() {
        return Err(BindingFailure::new(
            BindingCheck::Reference,
            "the reference coordinate is empty",
        ));
    }
    if bound.reference.digest.algorithm != DigestAlgorithm::Sha256
        || bound.reference.digest.bytes.len() != 32
    {
        return Err(BindingFailure::new(
            BindingCheck::Reference,
            "the reference digest is not 32 SHA-256 bytes",
        ));
    }
    if bound.artifact.domain.is_empty() {
        return Err(BindingFailure::new(
            BindingCheck::Reference,
            "the artifact domain is empty",
        ));
    }
    let value = decode_canonical_cbor_v1(&bound.artifact.bytes).map_err(|error| {
        let check = if error.kind() == CanonicalValueErrorKind::NonCanonical {
            BindingCheck::CanonicalForm
        } else {
            BindingCheck::Decode
        };
        BindingFailure::new(check, error.to_string())
    })?;
    let computed = digest_canonical_value_v1(&bound.artifact.domain, &value)
        .map_err(|error| BindingFailure::new(BindingCheck::Digest, error.to_string()))?;
    let bound_digest = crate::digest_review(&bound.reference.digest);
    if computed != bound_digest {
        return Err(BindingFailure::new(
            BindingCheck::Digest,
            format!("the bound digest {bound_digest} is not the computed digest {computed}"),
        ));
    }
    Ok(value)
}
