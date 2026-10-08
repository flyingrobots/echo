<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Compiler-produced key type witness

Exact source and checked-in Core bytes copied from Edict `3f81f759e921a69b04fe8cf8e62e62f8f3dc7b7e`,
`fixtures/lawpack/hello-echo/`. This is a retained compiler artifact, not a fresh
compiler run. The witness resolves the imported input record and inline key type
in each native provider; it does not establish full build/execute integration.

Core SHA-256: `bbf7e69a9146d5dbffb7f7e234830821bbc38070159ff0821c1dbf191ddebeca`.

## Fresh public component-host build

`built/` retains hex carriers for the executable package and verification report
produced by a fresh public application build using Edict compiler commit
`2405a550e93e1e97fff640caa44bbd0f65ffff3c`, binary SHA-256
`3b082d61c8cd23b0f917efb55c4eebd54e75c0df5d31c72f90ad856853678bf5`.
This compiler pin differs from the provenance of the retained input fixture.
The lowerer component hash is
`0f330366e7e50fc59f62158ad28c4d0cd8b2e82191375bcbf6535d2eb8194244`;
the verifier hash is
`2d67816b9fa9128cbb0a5da761ce59fdb9c4410b28d7e1793cb5eedde47faeaa`.

`build.py` retains the preparation and public JSONL invocation. It changes the
configuration selector to v2, rebinds its adapter reference to the provider's
registered v1 artifact domain, and recomputes the adapter, lawpack and source
import identities. It does not construct or edit Core or Target IR. Its small
fixture-only CBOR codec requires exact canonical round trips of each input.

Run inside the project's guarded Docker worker with a new, owned output path:

```sh
python3 build.py --compiler /path/to/pinned/edict   --compiler-sha256 3b082d61c8cd23b0f917efb55c4eebd54e75c0df5d31c72f90ad856853678bf5   --provider-package /path/to/echo/schemas/edict-provider/package/v1   --output-directory /path/to/owned/new-build
```

Expected result: the compiler reports `errors: 0`, and writes
`build/executable-operation-package.cbor` and `build/verification-report.cbor`.
This proves the component-host build, not runtime execution or WAL replay of
these retained bytes.
