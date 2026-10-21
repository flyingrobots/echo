<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Current effect publication schema control

`package.hex` is a synthetic, zero-effect serialization control: the exact output of `effect_publication::encode_effect_publication` for the crate-local `current-effect-wire` controls in `echo-edict-provider-lowerer`. A lowerer test requires it to equal the current emission, so a change to the package shape cannot leave it stale.

It is not a public compiler output, independent verifier result or execution receipt. Tests check that the current runtime decoder and the provider's owning metadata schema agree on its structure. The program remains opaque to that metadata schema; runtime preparation and semantic verification remain separate.
