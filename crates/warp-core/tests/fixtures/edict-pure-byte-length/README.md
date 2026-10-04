<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Compiler-produced byte-length fixture

The hex carriers preserve the exact package and separate accepted verification
report from Edict's public build. The retained source adds an inserted-byte
length to the guarded deleted-byte difference. It is an isolated consumer
experiment; jedit's checked-in proof and producer locks were not repinned.

- Application base: jedit `83ab52a3744babb63c8ce15cd94d917012e364a2`,
  `edict/replace-range/edict.application.json`.
- Source: adjacent `ReplaceRange.edict`.
- Edict compiler: `77f4a91080a6c2c875fe349da6fc6a64525947a2` (bounded byte length and independent target validation).
- Echo provider: `49e9efb68001dfd78563d18bac9359a87671e431`.
- Compiler host: Rust 1.95.0 in a copied-source Docker environment.
- Package: 11,897 bytes; raw SHA-256
  `48d3cabbe19f3ef1fc549849d26a87cd16b36c25324c7338a43d569090e3a1e5`.
- Report: 930 bytes; raw SHA-256
  `ceb2c283cb8c4e853a6eff81454883c7251daa7e09a75e4df3c9ca39e3f89f8e`.
- Domain-framed package identity:
  `sha256:cf3a9c1eaefc60d6826e639d51ad8c7beaefad93cdac49b80c759d334992d85b`.

Reproduce inside Docker by copying the base application, replacing only
`src/ReplaceRange.edict` with the adjacent source, and copying the pinned
provider's `schemas/edict-provider/package/v1` into `.build/echo-provider`.
Build the pinned compiler and submit this JSONL request from the application
directory to its `edict` CLI:

```json
{"schema":"edict.compiler.settings/v1","type":"compilerSettings","operation":"build","application":"edict.application.json"}
```

Compare `.build/application/*.cbor` with the decoded carriers. The fixture was
reproduced from the committed compiler source; package and report bytes match
the preceding source experiment exactly. No emitted artifact was edited.

Mutated packages in negative tests have deliberately matching test-host pins to
exercise interpreter checks. They are not accepted compiler or admission
evidence. This boundary program still returns ordinary values; it is not a rope
rewrite, graph mutation, Tick, receipt, WAL, or recovery witness.

## Inline byte-type control

The `inline-*.hex` carriers came from the same committed compiler and provider,
with only `text.ReplacementBytes` replaced by `Bytes<max=1048576>` in the retained
`InlineBytes.edict` source. The public build emitted a fresh accepted package;
this control exercises structural type decoding in both input and operation.

- Package: 11,797 bytes; raw SHA-256
  `556e3bd9ad6eb884ddf29a3fc5af014b2bd368de77a0baf7f429c1d6b3432ab8`.
- Report: 930 bytes; raw SHA-256
  `b7e580d167bf3fa47004e56da800a3114d1c35cd1932cb13978ebb92103269bb`.
- Domain-framed package identity:
  `sha256:d8f9aa0bd316de75007035d1c814cb10ef96655ea13cdca0d74343eed39d7f8c`.
