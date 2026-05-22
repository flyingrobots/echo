<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Versioned Contract And API Compatibility

Status: v0.1.0 release blocker.

Depends on:

- [Contract-aware receipts and readings](./KERNEL_contract-aware-receipts-and-readings.md)
- [External contract proof fixture](./PLATFORM_external-contract-proof-fixture.md)

## Why now

Developers need clear "this generated package fits this Echo runtime" behavior.
Ambiguous schema, ABI, codec, or generator drift should be rejected at install
or call time, not discovered through malformed receipts.

## Required identity

`v0.1.0` needs stable version identity for:

- Echo ABI version;
- Wesley generator version;
- contract package version;
- schema hash;
- artifact hash;
- codec id;
- generated helper compatibility;
- package install compatibility checks.

## Acceptance criteria

- Package installation records version and compatibility metadata.
- Version, schema, artifact, codec, or helper mismatch fails closed.
- Receipts and readings can cite the installed package identity they came from.
- The external proof fixture documents the Echo/Wesley versions used.
- Release notes identify the supported ABI/package compatibility set.

## Non-goals

- Do not support arbitrary historical package migrations in this slice.
- Do not accept drift by falling back to best-effort decoding.
- Do not encode application-domain version policy in Echo core.
