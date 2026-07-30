<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Compiler-Owned Workspace-Patch Fixture

These bytes were copied without modification from Edict merge commit
`cf8c17f917b7262be2c89fa136898e01dab7f40a`:

- `fixtures/lawpack/workspace-patch/apply-validated-patch.core.cbor`
- `fixtures/lawpack/workspace-patch/apply-validated-patch.core.sha256`
- `fixtures/lawpack/workspace-patch/apply-validated-patch.target-ir.cbor`
- `fixtures/lawpack/workspace-patch/apply-validated-patch.target-ir.sha256`

Edict emits the `.cbor` files and their reviewed `sha256:` identities together.
Echo copies all four files unchanged; it does not recompute or regenerate the
digest fixtures. Edict owns their refresh through:

```sh
cargo xtask lawpack-goldens --write
```

Refresh the Edict goldens first, copy the four exact files from one committed
Edict revision, and update the source commit above in the same Echo change.
Echo's `lawpack-goldens` task does not own these external compiler artifacts.

Echo treats the files as received compiler artifacts. It independently checks
canonical encoding, the reviewed source-Core digest, the exact capability
closure, request-only shape, runtime request fields, and target identity.
