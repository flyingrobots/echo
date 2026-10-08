<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Compiler-produced byte-slice fixture

The hex carriers preserve exact bytes from Edict's public application build.
They are not handwritten executable artifacts. The adjacent source is the
unchanged Jim-owned leaf-byte pressure witness.

- Jim source and retained public-build evidence:
  [`0cd880224fafcd1f26f78552e7b34ad170ae6a04`](https://github.com/flyingrobots/jedit/tree/0cd880224fafcd1f26f78552e7b34ad170ae6a04/edict/replace-range-probes/leaf-slice).
- Compiler: Edict `0835f398336ce1c693b5531b262228cd909c0b4b`, Rust 1.95.0.
- Provider: Echo `49e9efb68001dfd78563d18bac9359a87671e431`.
- Provider manifest raw SHA-256:
  `c5b9fb2fe3a0dc4dad282621a97413225c555be0071f3502b3272952069d42dc`.
- Package: 7,586 bytes; raw SHA-256
  `bfacd029cf2dc9247cbcee47d747eca3f8298c7184cac3866eb36f3ab55573c1`.
- Report: 924 bytes; raw SHA-256
  `943ac86ea20c40d68881cc8413b926f98ba2a4ee99408e3dcede4eead1e7bb86`.
- Domain-framed package identity:
  `sha256:1f123ca0182537ab5cb2cd6859cd3bdc3bed714e145296b9cab5484784cf99f2`.

The compiler build ran twice with identical package/report output. The report
records `accepted`; that establishes the observed package relation, not runtime
primitive availability. Jim's frozen compiler/provider/evaluator locks remain
unchanged. This retained candidate compiler coordinate is historical evidence,
not a moving build prerequisite or producer migration.

Reproduce with copied source in the existing guarded Docker worker, reusing the
accounted compiler cache. Jim's linked `run.py --boundary package` witness checks
the exact copied compiler tree, builds the unchanged source and two controls,
and records artifact hashes. Do not create another worker/image/cache or bake
compiler outputs into image layers. Decode these hex carriers and compare them
with the unmodified emitted `.build/application/*.cbor` files.

The runtime test transcribes all twelve literal cases from the same Jim
`cases.json`: eight exact outputs and four authored input-constraint failures.
The expected bytes are literal conformance data, never application executable
input. The operation slices raw bytes; UTF-8 policy and rope facts remain
application responsibilities.

Negative tests deliberately mutate Target expressions and give them matching
test-host pins to exercise runtime defenses. These forged packages are not
accepted verifier artifacts, admission evidence, or an alternative operation
authoring route. This pure test surface has no graph, mutation, Tick, receipt,
WAL, or recovery authority.

A separate test-only alpha-renaming control rewrites the application coordinate,
operation, result record type, and input fields, then compares outputs and work
and storage charges. It uses matching test-host pins and is not an additional
accepted compiler artifact. It checks that application names do not choose the
primitive's behavior; the original compiler package remains unchanged.
