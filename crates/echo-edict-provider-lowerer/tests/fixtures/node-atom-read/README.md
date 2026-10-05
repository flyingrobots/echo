<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Bounded-read compiler request

These are unmodified canonical input artifacts captured from Edict
`2405a550e93e1e97fff640caa44bbd0f65ffff3c` while building the supplied `source/`
through its public lawpack/application JSONL API inside Docker. The source is
based on the Jim-owned node-atom-read probe at
`f694a741b3bc522b80a1f9b7c149284776f90802`.

The source lawpack selects the proposed `compiler-produced-bounded-read/v1`
configuration with `maxReads: 64` and `maxReadBytes: 1048576`. Its generated
manifest digest replaces the import digest. The authored operation body is
unchanged. No Core or Target IR was fabricated or rewritten.

For capture only, the compiler was instrumented immediately before
`validate_provider_lowering_request`: it copied `request.core.artifact.bytes`,
`request.target_profile.artifact.bytes`, and every semantic input's
`artifact.artifact.bytes` to diagnostic files. This adds observation to the
compiler; it changes no compilation or validation decisions. Hex files retain
those exact bytes. The native test reconstructs their role/coordinate envelopes
and recomputes canonical artifact identities.

**This capture is native test evidence, not a successful public application
build.** At capture time the ordered-schema candidate rejected configuration
at `05-target-configuration`. No executable package or accepted verifier report
was captured here. The separate `bounded-read-publication` Docker witness now
compiles this authored source through unmodified Edict and the promoted
components. Trusted-host private evaluation is implemented by the bounded-read
runtime. Admission, Tick, WAL, and receipts remain outside this fixture and
evaluator. Frozen Jim application pins and historical WIP remain unchanged.

| Artifact | Bytes | Raw SHA-256 |
| --- | ---: | --- |
| `01-lawpack-adapter.hex` | 928 | `68d5f97771024b8a5f03a78bb7d338f9e217112d886d8977da839fcf699cadf6` |
| `02-lawpack-exports.hex` | 1683 | `caa0727c16bc81881fefcf3b98f0c76af73172571e39c32ed6d905cc02f9aca2` |
| `03-lawpack.hex` | 787 | `43bc67da3857a7454c32db99fb6c415e5813588f7d1e52596dac679eea61c344` |
| `04-source.hex` | 725 | `e18481d96d88ea1fef711618e347b486977988743c3331c7e1c3125330106b15` |
| `05-target-configuration.hex` | 130 | `08caf6c4c55fdd5e1e515415aed24e246d8ec429261b292ad7e6d8b7055e3473` |
| `06-target-ir.hex` | 2252 | `8d2f807cae789541f3d0a9d48ae66f512cc93461d9dd225b039d7644aac5b0b7` |
| `07-result-projection.hex` | 238 | `abbeb0eebcfa8054c46c3791d6ce4f6e0d3a7953f5d81a56651fb92bebeb38b8` |
| `core.hex` | 2506 | `6b5050cb73f415ab5a1b6c2f9de99baea22c42bf34ece9360f607383e08fca6c` |
| `profile.hex` | 1787 | `1b105d1b1f6cdf5fecdef98b7adeb238525047d43581fe9fd8c44fd213e1788e` |
