<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Compiler-produced byte equality

These hex carriers preserve exact public Edict application-build output from
Jim commit [`b1aeaaabd0c8911c31a506f5e273bd33ce9f31cc`](https://github.com/flyingrobots/jedit/commit/b1aeaaabd0c8911c31a506f5e273bd33ce9f31cc),
under `edict/replace-range-probes/basis-identity/`:

- `ReplaceRange.edict` compares nominal 32-byte IDs.
- `PayloadEquality.edict` compares bounded variable-length payloads.
- Both use the frozen `edict/replace-range/` application manifest and lawpack,
  Edict `3f81f759e921a69b04fe8cf8e62e62f8f3dc7b7e`, and Echo provider
  `49e9efb68001dfd78563d18bac9359a87671e431`.
- Compiler host: Rust 1.94.0 in Docker. Existing Jim producer pins are unchanged.

| Carrier | Bytes | Raw SHA-256 |
| --- | ---: | --- |
| `head-executable-operation-package.cbor.hex` | 6538 | `06438882bf7cd9f283abd45763cbfee085f8369f5c09c02d17b6b002dabe07a9` |
| `head-verification-report.cbor.hex` | 930 | `aff0303239aa0ff462b4411134ab7a4b3138055d1b157dcd3369ba624f72d859` |
| `payload-executable-operation-package.cbor.hex` | 6244 | `286bbadafdeeb9eaf529aa2811619ed971e38eb7c4eae673cd45a1f8894807f0` |
| `payload-verification-report.cbor.hex` | 930 | `cc54430442ad70d6dbf41883cfc1e13d58ff45f0bce382ea0a051b4a9e1c08b7` |

Sizes and hashes refer to the decoded bytes, not their hex transport. The
Edict domain-framed package pins are:

- Head: `sha256:3ac6f90290fe22683f8f4043761e5f0c409eeb51dd8ebd6a9025b2d7c28cdb96`.
- Payload: `sha256:4f850549859ef9eda931c10cd084415a7ecd4606c4e1d9a702660204d9e0d196`.

## Reproduction

Use an existing guarded Docker worker with copied sources and the exact pinned
compiler and provider above. Its exclusive lock, shared storage and log limits,
host/VM free-space floors, CPU/memory limits, and timeout must cover compiler
output, temporary application copies, logs, and child processes. Keep compiler
output in accounted reusable storage. The historical Jim image-build recipe
is provenance, not a supported reproduction command: it compiles into image
layers and does not enforce these requirements. No standalone runner is supplied
here.

The pinned Jim `build.sh` expects the compiler at `/edict/target/debug/edict`,
the provider under `/echo`, the frozen application under `/application-source`,
and the two source files under `/comparison-sources`. Its disposable
`/comparison-head`, `/comparison-payload`, and `/comparison-output` paths must
be fresh and included in the worker's accounting. The script replaces only the
authored source in each application copy, invokes Edict's public JSONL build,
and retains each emitted package and separate accepted report. Compare those
bytes with these decoded carriers. The two historical public builds produced
byte-identical packages and reports. No Core, Target IR, package, or report was
hand-authored or edited.

The evaluator tests validate each package pin and its separate report binding,
then check literal authored branch results and metered bounds. Mutated packages
in rejection cases deliberately use matching test-host pins; they are not
compiler-produced or independently accepted artifacts.

The recorded accepted reports describe the frozen provider above, which predates
later source/Core/Target relation and projection checks. Acceptance under that
provider alone is not a proof of complete source-to-target equivalence. The
source identity, decoded program, and literal runtime results are separate
evidence; these fixtures do not silently adopt a newer producer.

This is pure value computation. A supplied head ID is not a graph observation,
and byte equality grants no admission authority. Generic snapshot reads,
application fact decoding, rope semantics, Tick, receipt, WAL, and recovery
remain outside this witness.
