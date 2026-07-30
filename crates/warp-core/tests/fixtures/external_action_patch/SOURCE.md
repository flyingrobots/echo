<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Compiler-Owned Workspace-Patch Fixture

These bytes were copied without modification from Edict merge commit
`cf8c17f917b7262be2c89fa136898e01dab7f40a`:

- `fixtures/lawpack/workspace-patch/apply-validated-patch.core.cbor`
- `fixtures/lawpack/workspace-patch/apply-validated-patch.target-ir.cbor`

Edict owns regeneration through:

```sh
cargo xtask lawpack-goldens --write
```

Echo treats the files as received compiler artifacts. It independently checks
canonical encoding, the reviewed source-Core digest, the exact capability
closure, request-only shape, runtime request fields, and target identity.
