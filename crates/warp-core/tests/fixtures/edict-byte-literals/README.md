# Compiler-produced byte literal fixture

The hex carriers are the exact package and separate accepted verification report
from Edict's public JSONL application build. `Literal.edict` is the authored
source, based on Jedit `9e6466150e43afc83966f9fe67c6ddce2f58d5bc` and adding a
compiler-owned binding for `b"\r\0\xFF"` plus its projected result field.
No package bytes were hand-authored or reconstructed from a schema.

- Compiler: Edict `6e6a0fa35607f9f2810cf3b3ad30c45bfb35363a`, PR #250.
- Provider: Echo `9b3b9396b635cb0cda6b894ab4804f0c93bf7b6a`.
- Environment: Rust 1.96.0, Node 22.23.1, npm 10.9.8 in Docker.
- Source SHA-256: `a797ab88981c61308d576b618781df684d2a63546d220b96ca04eff7d9033423`.
- Package: 10,518 bytes, raw SHA-256 `a0f158d22193e31a7e988c32717e93f9185daa90644f52adf2b4799262a57718`.
- Report: 930 bytes, raw SHA-256 `bfd254dfd9e592bdf170424ef62b8b1ca1b356dd2356d8c05f49b1eefb7c973b`.

To reproduce, copy Jedit's `edict/replace-range` application root at the stated
commit, replace its `src/ReplaceRange.edict` with `Literal.edict`, and copy the
pinned Echo `schemas/edict-provider/package/v1` into `.build/echo-provider`.
Build the pinned Edict CLI, then send this request to it from the copied root:

```json
{"schema":"edict.compiler.settings/v1","type":"compilerSettings","operation":"build","application":"edict.application.json"}
```

Compare `.build/application` package/report bytes with the decoded carriers.
The test checks the report's accepted outcome and exact package identity before
evaluation, then independently expects the three octets `[13, 0, 255]`.
Repeated execution and exact/one-below work, allocation and output budgets are
checked. The result projection cites the named binding, as required by the
existing projection ABI. An inline constant in the return record is outside
that ABI and is not claimed supported here.

This fixture returns values only. It establishes no graph mutation, admission,
Tick, receipt or WAL recovery claim.
