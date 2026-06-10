<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# External Contract Proof Fixture

Status: serious local fixture implemented; jedit release gate remains.

Depends on:

- [Contract-aware receipts and readings](./KERNEL_contract-aware-receipts-and-readings.md)
- [Contract reading identity and bounded payloads](./KERNEL_contract-reading-identity-and-bounded-payloads.md)
- [Contract retention and semantic lookup seams](./PLATFORM_contract-retention-and-semantic-lookup-seams.md)
- [Product-facing intent outcome API](./PLATFORM_product-facing-intent-outcome-api.md)
- external `jedit` contract/runtime work, if `jedit` supplies the serious
  consumer shape

## Why now

The generic contract-host path needs one serious external consumer proof before
Echo can claim `v0.1.0` buildability. The in-repo external fixture proved
the local mechanics. It is no longer enough for the release. The release gate is
now a real `jedit` proof from the sibling repository because it pressures
bounded readings, retained evidence, conflicts, replay, generated artifacts,
and application ergonomics without letting Echo core import application nouns.

## What it should look like

Use an application-owned Wesley contract fixture that proves this path:

```text
external contract
-> Wesley generated artifacts
-> Echo package install
-> generated intent submission
-> scheduler-owned execution
-> generated QueryView/Query reading
-> retained evidence
-> local replay proof
```

The fixture may use application-shaped operations and readings, but the nouns
remain in the external contract and generated payloads.

## Acceptance criteria

- [x] The fixture includes at least one mutation.
- [x] The fixture includes at least one `QueryView`/`Query` reading.
- [x] The mutation and query use non-trivial vars.
- [x] The reading evidence includes bounded basis, aperture, and budget
      identity.
- [x] Receipt and reading evidence can be retained and inspected.
- [x] At least one conflict, rejection, obstruction, or residual path is
      exercised.
- [x] Local replay reproduces the generic fixture outcome.
- [x] Echo core contains no application product APIs outside generated fixture
      payloads.
- [x] The fixture may declare retained tick/receipt obligations, but
      application code does not create ticks or `TickReceipt` values.
- [ ] The sibling `jedit` repository can run the documented real Echo witness
      against a clean Echo checkout or published artifact.
- [ ] sibling application code submits intent without access to scheduler
      control.
- [ ] trusted Echo host code owns package installation, scheduler control,
      until-idle policy, and fault recovery.
- [ ] jedit observes an outcome and bounded reading with retained evidence.
- [ ] jedit replay reproduces the same outcome and reading.

## Implemented local slice

`external_consumer_contract_fixture_tests` adds a serious
external-consumer-shaped fixture. The application names live only in the test
package. The fixture installs through the generic package boundary, submits
through the app-facing host handle, resolves one overlapping mutation as a
footprint conflict, observes a bounded query reading, and retains reading plus
receipt evidence through semantic coordinates.

## Remaining release gate

The next proof must live in `~/git/jedit`, not as another `warp-core` fixture.
The current opt-in jedit real-WASM witness is useful but stale: it still tries
to send scheduler control through app-facing dispatch. Echo correctly rejects
that as forbidden control intent. The fix is to update the jedit witness around
the app/host split, not to grant application code tick authority.

## Non-goals

- Do not build the `jedit` product UI.
- Do not author the `jedit` product contract inside Echo.
- Do not make the fixture a privileged Echo ontology.
- Do not add a special `jedit` ABI.
- Do not implement downstream automation in Echo core.
