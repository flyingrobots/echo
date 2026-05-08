<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-registry-api

Generic registry interface for Echo WASM helpers. Provides the trait and data
types (`RegistryProvider`, `RegistryInfo`, `OpDef`) that an application-specific
registry crate implements. `warp-wasm` links only to this interface so Echo
stays generic; apps supply their own registry at build time.

`OpDef` preserves authored operation directive metadata as JSON. Echo admission
tooling can interpret entries such as `wes_footprint`, but this crate only
carries the data so the generic runtime boundary stays application-neutral.
