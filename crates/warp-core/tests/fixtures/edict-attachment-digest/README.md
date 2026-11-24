<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public Buffer-bump program under `echo.dpo@2`

`source/BumpBuffer.edict` is the shape jedit's `ReplaceRange` needs to bump
`Buffer.version` (flyingrobots/echo#1011). It reads the Buffer atom through
`echo.dpo@2.node-atom-read` (`readBuffer`), computes
`echo.attachmentValueDigest(input.buffer.typeId, current)` over the bytes it
read, and calls `echo.dpo@2.replace-atom` (`replaceAtom`) with that digest as
`expectedAtomDigest` and the caller's next atom encoding. It returns the
digest. The program does not encode the Buffer itself: the caller supplies the
next encoding, as jedit's codec modules do in Edict.

The lawpack `consumer.buffers@1` binds those two effects under the
`echo.dpo@2` profile. Its configuration carries the `echo.dpo.intrinsics/v2`
catalog the profile pins, a graph schema with one mutable node type (`7b…`)
holding one atom type (`02…`, the frozen `attachment-new.hex` vector's atom
type), and a footprint law that reads and writes that node type. The
application requires `echo.dpo@2.attachmentValueDigest` at
`echo.content-address/v1` through `target.pureIntrinsics`. The coordinates
under `consumer.buffers` are neutral.

`controls/unread-bytes/` holds the same lawpack and a program that differs in
one operand: it digests `input.next`, bytes it never read. It compiles and
evaluates, and Echo's derivation refuses its `ReplaceAtom` with
`PreconditionMismatch`.

## Provenance

The `bump-*.hex` and `controls/unread-bytes/unread-*.hex` files are the public
compiler's outputs for these sources, written through the checked provider
package by the `effect_step_publication` example with an unmodified Amblicit
`7dbcbc23d72705c5e224fc994a6c767881958912` binary. Amblicit
`738d3da7` reproduces the same bytes. `scripts/verify-edict-provider-host-v1.sh`
reruns both with `--check`, which also requires the program to be refused
under `echo.dpo@1` and under a substituted `echo.dpo@2` profile.

`crates/warp-core/tests/executable_operation_pipeline/public_attachment_digest_effect.rs`
runs the package against a Buffer: the digest of the atom `new` equals the
frozen vector, derivation accepts the replacement, the control is refused, and
a 64-step budget is refused. `crates/echo-edict-provider-verifier/tests/public_attachment_digest_effect.rs`
requires the native lowerer to reproduce the package and the separate verifier
to accept it.
