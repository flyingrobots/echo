<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `echo-wesley-gen` carries local generator behavior that wants to live in Wesley

Legend: `PLATFORM`

## The smell

`crates/echo-wesley-gen/src/main.rs` currently carries generator
behavior that should eventually live upstream in wesley-core / the
Wesley emitter, with echo's copy collapsing on a dependency bump.
The duplication exists because echo could not wait for wesley-core
to ship the matching behavior before PR #382 (universal LE binary
codec) landed; the echo-side fixes were the minimum-viable response
to keep #382 mergeable.

Concrete examples currently in the file:

- **`fnv1_step` and the duplicated `stable_op_id` derivation.** The
  source comment is explicit: "Must stay bytewise identical to
  `wesley_core::stable_op_id` (added in wesley-core ≥0.0.5). The
  duplication will collapse when echo bumps its wesley-core
  dependency to 0.0.5+; until then both copies are pinned to the
  same outputs in unit tests." Cross-language op-id drift is the
  exact class of thing that waits patiently and then bites during a
  release.
- **`ir.codec_id` normalization.** PR #382 patched the generator to
  force `ir.codec_id = DEFAULT_CODEC_ID` after parsing, so the
  artifact hash / observer identity / footprint certificate
  preimages cannot be computed under a stale codec id. The right
  long-term home for that policy is the Wesley IR validator, not a
  per-emitter prologue.
- **Codec trait imports inside generated modules** (`use
echo_wasm_abi::codec::{Decode as _, Encode as _};`). Added because
  generated `impl Encode / Decode` bodies call method-syntax
  `.encode(w)` / `Type::decode(r)` on nested user types. The Wesley
  emitter should own this prelude shape rather than having every
  generator that targets the codec trait re-derive it.
- **no_std ID list element encoding.** The fixes in PR #382's
  `scalar_list_element_encoder` / `_decoder` (and the prior
  scalar-helper fix in `8ef2fc97`) belong in the canonical Wesley
  TypeScript / Rust emitters that target `no_std` consumers, not
  duplicated in echo-wesley-gen alone.

The sibling concern in the jedit repo is
`jedit/docs/method/backlog/bad-code/generated-rope-codec-manual-fixes.md`,
which records the same shape one layer downstream: jedit carries
hand-edits on a generated codec file because the TS emitter does
not yet produce the trailing-byte check. Same root cause; different
half of the stack.

## Why this matters

- A cross-language op-id contract (`stable_op_id` / `fnv1_step`)
  that has two implementations in two repos must stay byte-wise
  identical forever. Until the duplication collapses, one repo can
  drift and the only thing that catches it is the unit test that
  pins both sides — and only if it stays current with both
  implementations.
- The codec-id normalization patch in the generator prologue is
  load-bearing for artifact identity (the hash preimage formerly
  could be derived under one codec while the artifact advertised
  another). Putting that policy in the IR validator would make it
  impossible to skip — the current shape is a per-binary prologue
  that a sibling generator can omit.
- Every additional emitter-specific fix in echo-wesley-gen widens
  the eventual collapse-onto-wesley-core diff and makes the version
  bump scarier. Today this is ~50 lines; left for two more PR
  cycles it will be ~500.

## The fix shape

- Upstream the byte-step / `stable_op_id` derivation to wesley-core
  ≥0.0.5 (or whichever version actually ships it), bump echo's
  wesley-core dependency, and delete the local copy + its pinned
  test. The pinned vectors stay; they just become a regression
  against the upstream function.
- Move `ir.codec_id` normalization to the wesley-core IR
  validator. Echo's generator then trusts the IR rather than
  patching it post-parse.
- Push the codec trait prelude into the Wesley emitter for codec-
  trait-targeting generators. Echo's generator drops the manual
  `tokens.extend` for those imports.
- For no_std ID list element handling: ensure the canonical Wesley
  Rust emitter targets the `no_std` mapping correctly in list
  contexts (this is the same issue PR #382 fixed at scalar AND list
  level locally). The jedit-side sibling card tracks the TS emitter
  half.

## Out of scope here

- Implementing any of the above in the Wesley repo itself. Each
  belongs in its own Wesley cycle / PR with the matching pinned
  vectors and test surface.
- Premature collapse. Do not delete the local copies until the
  upstream replacement has shipped, the dependency bump has landed
  on echo's main, and the pinned regression tests stay green
  against the upstream surface.

## Trigger / acceptance

Resolve this card when:

1. wesley-core ships `stable_op_id` / `fnv1_step` (or named
   equivalents) and echo bumps the dep.
2. `crates/echo-wesley-gen/src/main.rs` deletes the duplicated
   helper and its doc comment; the pinned op-id regression test
   keeps the contract.
3. wesley-core IR validator (or equivalent) owns codec_id
   normalization; the generator drops the post-parse
   `ir.codec_id = ...` patch.
4. The Wesley emitter owns the codec trait prelude and the no_std
   ID list element mapping; the generator drops the manual
   `tokens.extend` for those.

## Companion

- `jedit/docs/method/backlog/bad-code/generated-rope-codec-manual-fixes.md`
  — same root cause, downstream half. The two cards should resolve
  together; resolving one alone leaves the other carrying the
  emitter gap.
- `docs/method/backlog/cool-ideas/PLATFORM_wesley-gen-test-loop-speedup.md`
  — orthogonal but adjacent (it touches echo-wesley-gen's test
  harness, this card touches echo-wesley-gen's emit logic).
