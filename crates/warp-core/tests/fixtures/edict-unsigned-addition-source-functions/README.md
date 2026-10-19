<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public source-function unsigned addition fixture

These package and verifier-report carriers are unmodified outputs of Edict's
public JSONL application build against a freshly published SourceFunctions
provider package. The authored source computes a guarded U64 sum, passes it
through an ordinary source function containing `value + 0`, and computes a
separate guarded U32 increment. The package therefore exercises the strict
source-function provider judgments as well as the runtime arithmetic path.

The accepted report binds package pin
`a8774d9e8ba5dbe1e4eaffee4ec8f10f7b80ee9812b5569e8c2d290e57de9443`.
`provenance.json` records the compiler, independently built component identities,
provider publication and exact fixture hashes. The rebuild checks the generated
provider reference against that recorded publication before invoking the compiler. The four vendored lawpack
carriers and application manifest retain the complete public build inputs.
A repeated public build produced identical package and report bytes.

The integration tests in `edict_unsigned_addition_tests` check exact outputs at
both unsigned width boundaries and structured refusal outside the proved input
domain. This is bounded ordinary computation. It does not create graph effects,
admit an Action, commit a Tick, or establish receipt/WAL/recovery behavior.

To rebuild in a guarded Docker worker with copied sources, use the source and
component identities recorded in the provenance. Set `ECHO_SOURCE` to the copied
Echo root at the recorded publication snapshot, `ADDITION_FIXTURE` to this
retained fixture directory, `EDICT_BIN` to its companion Edict CLI binary and
`ADDITION_APP` to a new absolute scratch directory. The fixture may come from
a later checkout than the recorded producer snapshot. The provider publication example requires Docker.

```sh
set -eu
python3 - "$ADDITION_FIXTURE" "$ADDITION_APP" <<'PY'
from pathlib import Path
import shutil, sys
source, app = map(Path, sys.argv[1:])
shutil.copytree(source, app)
for carrier in app.rglob('*.hex'):
    carrier.with_suffix('').write_bytes(bytes.fromhex(carrier.read_text().strip()))
(app / '.build').mkdir()
PY
cargo +1.96.0 run --locked --manifest-path "$ECHO_SOURCE/Cargo.toml" \
  -p echo-wesley-gen --example source_functions_publication_witness -- \
  "$ADDITION_APP/.build/echo-provider"
python3 "$ADDITION_APP/verify-provider-provenance.py" "$ADDITION_APP"
cd "$ADDITION_APP"
printf '%s\n' '{"schema":"edict.compiler.settings/v1","type":"compilerSettings","operation":"build","application":"edict.application.json"}' | "$EDICT_BIN"
cmp executable-operation-package.cbor .build/application/executable-operation-package.cbor
cmp verification-report.cbor .build/application/verification-report.cbor
```
