<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# JS/WASM/Browser Client Release Surface

Status: v0.1.0 release blocker if JS, WASM, Node, or browser packages ship.

Depends on:

- [App-safe client surface](./PLATFORM_app-safe-client-surface.md)
- [Product-facing intent outcome API](./PLATFORM_product-facing-intent-outcome-api.md)
- [Package publish and versioning](./RELEASE_package-publish-and-versioning.md)

## Why now

The release bar now includes serious application use. If Echo publishes
JavaScript, WASM, Node, or browser artifacts, those artifacts must carry only
the app-safe client surface. They must not accidentally expose trusted runtime
control, tick authority, WAL append authority, recovery authority, or package
install authority to application code.

## Required behavior

Published client artifacts may expose:

- generated intent/query request helpers;
- canonical intent submission;
- intent outcome observation;
- bounded query reading requests;
- retained evidence posture;
- package/version compatibility checks.

Published client artifacts may not expose:

- scheduler step or tick;
- trusted runtime start/stop/drain control unless the artifact is explicitly a
  trusted-host package;
- WAL append or recovery mutation;
- package installation authority;
- raw kernel object mutation.

## Acceptance criteria

- [ ] Release documentation names which JS/WASM/browser artifacts ship.
- [ ] Shipped application-facing packages expose only app-safe APIs.
- [ ] Trusted-host APIs, if shipped, are packaged and documented separately from
      application client APIs.
- [ ] Browser and Node smoke tests prove generated requests can be submitted
      and observed without tick authority.
- [ ] Package metadata binds Echo ABI version, WASM ABI version, generated
      helper compatibility, and contract package version.
- [ ] Examples do not call trusted runtime control from application code.

## Test plan

- Add package export tests proving app-facing entry points do not re-export
  trusted runtime controls.
- Add Node smoke test for submit/observe/query through app-safe API.
- Add browser or WASM-bindgen smoke test if browser artifacts ship.
- Add docs/quickstart check for the published artifact set.

## Non-goals

- Do not require browser artifacts if the release explicitly ships Rust/local
  host APIs only.
- Do not build a UI framework.
- Do not implement streaming subscriptions.
- Do not expose raw privileged WASM exports to app code.
