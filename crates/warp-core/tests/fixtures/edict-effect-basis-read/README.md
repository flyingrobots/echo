<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# jedit#296 basis-read witness package

`basis-read-executable-operation-package.hex` is unchanged public compiler
output: jedit#296's basis-read witness (`jedit.text.basis_read@1.basisRead`),
compiled by Amblicit `738d3da7` against Echo `0d1f74a0`'s
`schemas/edict-provider/package/v1`. Its `echo:operation-package:v1` identity
is `8e99859aa259a4f0ca4dff7c487292efddc4fcb0305a5e26d73a134a17ead865`.

Echo `0d1f74a0`'s provider lowerer and verifier accepted it, while warp-core
admission refused it (echo#1023). Its footprint law declares
`max_writes = 1024`, above the effect profile's 256, so admission was right.
`public_basis_read_effect.rs` pins the runtime's refusal and its reason code,
and the verifier's `runtime_admission_agreement.rs` requires the provider to
refuse it for the same check. The package is retained as a refused witness;
it is not rebuilt when Echo's provider changes.
