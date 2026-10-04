<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Compiler-produced unsigned subtraction

The hex carriers contain the unmodified canonical package and accepted verifier
report emitted by Edict's public JSONL application build in a copied-source
Docker environment. They were not constructed from an oracle or edited after
compilation.

- Application base: jedit `83ab52a3744babb63c8ce15cd94d917012e364a2`,
  `edict/replace-range/edict.application.json`.
- Authored source: the adjacent `ReplaceRange.edict`. Compared with the base,
  it adds and returns `deletedByteCount = input.endByte - input.startByte`
  under the existing ordered-range constraint.
- Compiler: Edict `de77085277eba157cb55b3353544bd84ce82f1c2`.
- Provider: Echo `49e9efb68001dfd78563d18bac9359a87671e431`.
- Compiler toolchain: Rust 1.94.0.
- Package: 11,030 bytes; raw SHA-256
  `44705e27e2fe085fcbbb0b3defddb3636ba10d53287b0de9d125aefb83190e29`.
- Report: 930 bytes; raw SHA-256
  `98e4526eaecb091b8d523c68ffca2ed304abbe284875f26d515090a4263e531a`.
- Domain-framed package identity:
  `sha256:c94cfd0321ef7eb1053f831adb6ddd46bf03561318fd8fdf3639e4289304fa02`.

To reproduce, copy the base application into an isolated Docker filesystem,
replace only its `src/ReplaceRange.edict` with the adjacent source, and copy the
pinned provider's `schemas/edict-provider/package/v1` into `.build/echo-provider`.
From the application directory, send this request to the pinned compiler's
public JSONL CLI:

```json
{"schema":"edict.compiler.settings/v1","type":"compilerSettings","operation":"build","application":"edict.application.json"}
```

Compare the emitted `.build/application/*.cbor` with the decoded hex carriers.
The original jedit source-closure and producer locks intentionally remain
unchanged; this external experiment is not a repin of its existing proof.
Evaluation produces an ordinary value, not a rope rewrite, admission, Tick,
receipt, WAL record, or recovery evidence.
