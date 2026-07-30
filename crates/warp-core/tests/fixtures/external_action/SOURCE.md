<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Compiler-Owned External-Request Fixture

These bytes were copied without modification from Edict commit
`97ef5eaf21f114abae74108d564b4a0c0f0b5d59`:

- `fixtures/lawpack/workspace-snapshot/observe-workspace.core.cbor`
- `fixtures/lawpack/workspace-snapshot/observe-workspace.target-ir.cbor`

Edict owns regeneration through:

```sh
cargo xtask lawpack-goldens --write
```

Echo treats the files as received compiler artifacts. It independently checks
canonical encoding, the reviewed source-Core digest, the exact capability
closure, request-only shape, runtime request fields, and target identity.
