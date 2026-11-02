<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public compiler witnesses

`list-worklist-publication.ts` sends authored Amblicit source through the public
compiler CLI and the selected checked provider components. Its companion
`list-worklist-sources.ts` contains the programs. It does not construct Core or
Target IR. The Rust example `source_functions_public_runtime` consumes the exact
accepted package/report pairs and checks the results against independent oracles.

Run these commands inside the project's existing guarded Docker worker, using
Node 22.18 or later and the pinned Rust toolchain:

```sh
node scripts/consumer-witnesses/list-worklist-publication.ts \
  --inputs /work/inputs.json --sha256 INPUT_MANIFEST_SHA256 \
  --work-root /work/new-worklist-witness
cargo +1.96.0 run --locked -p xtask --example source_functions_public_runtime -- \
  /work/new-worklist-witness EVIDENCE_JSON_SHA256
```

The SHA-256-selected input manifest has schema `echo.list-worklist-inputs/v1`:

| Field | Meaning |
| --- | --- |
| `revision` | Full reviewed Amblicit source commit |
| `compiler` | Absolute executable `path` and `sha256` |
| `compilerSource` | Complete verified compiler source inventory |
| `pureSeed`, `readSeed` | Selected public application/lawpack authoring inputs |
| `pureSource`, `readSource` | Seed-relative authored source paths |
| `provider`, `oldProvider` | Current checked package and the list-value-only predecessor |

Each inventory is `{ "root": "/absolute/path", "files": { "relative/path":
"sha256" } }`. Seeds must include their application configuration and every
referenced vendored resource; the read seed also includes `edict.lawpack.json`.
The read seed's `FactBytes` maximum is narrowed from 1048576 to 8 through a new
public lawpack build. Record compiler build provenance separately: matching
binary and source hashes alone do not establish that one was built from the other.

The driver checks selected input bytes before and after execution, repeats each
accepted application build, and retains authored inputs, compiler diagnostics,
package/report pairs and `evidence.json`. A failed run retains evidence without a
success outcome. Use a new output directory for each run. The driver bounds its
output tree and compiler process groups; the outer guard must still enforce the
aggregate project build/data/log budgets and stop every child on failure.

Compilation acceptance does not prove runtime execution. The Rust example's
`PUBLIC_WORKLIST_RUNTIME_ACCEPTED` marker is emitted only after the pure,
structural-alias, ordered-read and nominal cases execute, including exact and
one-unit-short resource checks. The source fixtures include a renamed application
coordinate to exercise generic dispatch.

The older `source-functions-publication.py` witness is retained for the earlier
source-function/list-value contract. Its language migration is tracked separately
in [Echo #954](https://github.com/flyingrobots/echo/issues/954).
