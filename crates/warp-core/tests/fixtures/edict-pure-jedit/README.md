<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Compiler-produced pure evaluation fixture

These hex files retain the exact canonical bytes emitted by Jedit's public
Edict build on 2026-09-18. Hex is only a text carrier. The tests decode it before
passing the original bytes to Echo. Neither file was constructed by a fixture
builder or reconstructed from the Jedit schema or oracle.

- Jedit source: `a894c7c4c6d150c0fb210d2e0ca4c27bf518b4c7`
- Application: `edict/replace-range/edict.application.json`
- Edict: `3f81f759e921a69b04fe8cf8e62e62f8f3dc7b7e`
- Echo provider: `49e9efb68001dfd78563d18bac9359a87671e431`
- Rust: `1.94.0`; Node: `22.23.1`; npm: `10.9.8`
- Package: 9,969 bytes, raw SHA-256
  `1860ab8a6cb9a4bfd63a1db99d1c2bc00aa53dae52a9d1998295654379a95d85`
- Report: 930 bytes, raw SHA-256
  `d96e5d21eecbc1e5bbf6ce76ab438d55cc8e74f6843121d4ffb0f4343e648267`
- Domain-framed package identity:
  `sha256:b9052d3c40878a9fcba7768be67c8dba50cf0f751682339dfcdd04155871415b`

Reproduce in the exact Jedit checkout with clean pinned compiler/provider
checkouts and the stated Node on PATH:

```sh
EDICT_REPO=/path/to/pinned-edict ECHO_REPO=/path/to/pinned-echo \
  ./edict/replace-range/tests/build.sh
```

Compare each `.build/application/*.cbor` with its decoded hex carrier. The build
checks the full source closure, exact toolchain and provider component bytes,
separate accepted verification, and executable-subject identity. The input and
expected output in the Rust test are independently written literal values.

This source returns a boundary record with a pure conditional and an imported
authored helper. It does not yet implement the rope algorithm. Evaluating it is
not graph mutation, installed invocation, a Tick, a Receipt, or WAL evidence.
