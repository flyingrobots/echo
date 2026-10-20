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
enforce Edict's exact 128-container nesting boundary. Decoding additionally
caps one materialized value tree at 65,536 nodes, charging map keys and values
separately. A matching cumulative reservation budget is charged before every
container capacity allocation, so nested declarations cannot reuse the same
budget to amplify host allocation. Those ceilings are Echo host resource
bounds, not new Edict byte-identity claims.

Failures expose stable `CanonicalValueErrorKind` values rather than Rust debug
spellings. The implementation is deterministic and pure: it performs no
filesystem, registry, environment, or network discovery. Version `0.1.0` is a
publishable leaf consumed by `echo-wesley-gen`; the checked provider component
uses the same implementation without making its build crate part of the public
registry dependency graph.

This codec authenticates neither an artifact's schema nor its authority. A
caller must separately validate the decoded value against the owning admitted
CDDL root and must not treat a domain-framed digest as runtime Echo admission.

## Source-closure encoding

`source_closure::source_closure_root_v1` checks the distinct
`edict.source-closure/v1` wire envelope and borrows its exact root source. It
checks closed fields, unique module coordinates, 1–64 units, the 1 MiB aggregate
original-byte ceiling, every unit's typed SHA-256 source digest, UTF-8 and the
expected root coordinate. It shares the canonical digest primitives used by the
provider models and bounded-read decoder. It performs no source parsing,
import-pin verification, graph traversal or runtime admission. Edict's compiler
still owns those source authentication obligations. The caller also checks the
canonical outer artifact and its domain-framed identity.

Run `cargo test -p echo-edict-canonical --test source_closure` for envelope,
substitution and exact byte-budget boundary checks.
