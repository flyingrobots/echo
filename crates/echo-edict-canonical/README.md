<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Edict Canonical Codec

`echo-edict-canonical` is Echo's pure implementation of the frozen
`edict.canonical-cbor/v1` value and byte contract plus
`edict.digest/v1` domain-framed SHA-256 identities.

The crate is intentionally smaller than either Echo's runtime codecs or the
provider generator. Both `echo-wesley-gen` and executable provider components
use this boundary so canonical bytes and digest propositions do not diverge.
It admits nulls, booleans, the CBOR major-zero and major-one integer range,
definite-length byte and UTF-8 text strings, arrays, and maps. Encoding sorts
maps by canonical key bytes and rejects duplicate canonical keys. Decoding
rejects indefinite-length, tagged, floating-point, simple, non-minimal,
trailing, malformed, duplicate-key, and noncanonical inputs. Both directions
enforce Edict's exact 128-container nesting boundary.

Failures expose stable `CanonicalValueErrorKind` values rather than Rust debug
spellings. The implementation is deterministic and pure: it performs no
filesystem, registry, environment, or network discovery. Version `0.1.0` is a
publishable leaf consumed by `echo-wesley-gen`; the checked provider component
uses the same implementation without making its build crate part of the public
registry dependency graph.

This codec authenticates neither an artifact's schema nor its authority. A
caller must separately validate the decoded value against the owning admitted
CDDL root and must not treat a domain-framed digest as runtime Echo admission.
