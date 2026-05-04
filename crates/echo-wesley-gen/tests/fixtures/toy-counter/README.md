<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Toy Counter Fixture

This is the smallest shared Echo/Wesley contract-hosting fixture.

It exists to prove:

- generated operation ids and registry metadata;
- typed operation variable generation;
- canonical operation variable encoding;
- EINT v1 packing;
- observation request generation;
- generated output compilation in a standalone consumer crate;
- future installed-host contract smoke tests.

It is not:

- a `jedit` fixture;
- a dynamic loading fixture;
- a GraphQL execution fixture;
- a host-side registry validation fixture;
- a text-editing or product-domain fixture.

Tests should consume `echo-ir-v1.json` through `include_str!(...)`. Do not copy
the toy counter IR into new tests. If the fixture needs to change, update this
single source and make the contract boundary change explicit in the test that
requires it.
