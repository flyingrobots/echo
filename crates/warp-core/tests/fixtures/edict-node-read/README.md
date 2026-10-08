<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Compiler-produced node-read witnesses

These hexadecimal files retain unchanged canonical CBOR from public Edict
`2405a550e93e1e97fff640caa44bbd0f65ffff3c`, using Echo's explicit ordered
publication at `2b76c5deb4fe2ebb8c354f5de9ff98f6f41dccf7`.

Source fixtures are owned by
`crates/echo-edict-provider-lowerer/tests/fixtures/node-atom-read/`.
The public compiler witness
`scripts/consumer-witnesses/bounded-read-publication.py` builds both programs
twice and checks the compiler's successful output boundary. The runtime target
independently checks the accepted reports. The package and report outputs were retained here
without decoding/re-encoding or synthesizing compiler artifacts.

| Program | Package domain-framed digest (`echo.operation-package/v1`) |
| --- | --- |
| Single | `a2ae35d39ae74a26da916544464529bc6c7af32a76b6e6f354ef79c5063ab109` |
| Pair | `359f1ff5d5fd82ebbbb8e3e3e2d454014ae66bbf77184f5cc497edfc57abddb5` |

The runtime test fixes these pins independently of package selection and checks
that each retained accepted report binds that exact package. The pair reads
two independently addressed stored atoms with an intervening guard. Test data
can come from fresh public compiler output through `EDICT_READ_OUTPUT_ROOT`;
that selection never changes the expected pins or bypasses Docker isolation.
Run compilation and evaluation sequentially in the guarded reusable worker
described by [application contract hosting](../../../../../docs/architecture/application-contract-hosting.md).
Its continuous guard must cover compiler output, runtime data, temporary files,
and logs; a pre/post cache measurement is insufficient. The retained provider
predates subsequent pure-package validation hardening; its accepted read reports
do not demonstrate that later provider code was exercised.

These are private evaluation witnesses, not installation or admitted causal
history. They do not prove Tick/WAL/receipt/reading or recovery behavior.
