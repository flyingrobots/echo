<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Checked Echo Edict Provider Package

[`v1/`](v1/) is the first exact digest-locked Echo Edict provider
distribution. Its inventory is fixed at 25 files:

- the exact lowerer and verifier component bytes under `components/`;
- the checked 22-file #652 artifact corpus under `generated/`; and
- the derived `provider-manifest.echo.json`.

The manifest never inventories itself. Its `provider.digest` names an
Echo-owned, domain-framed canonical-CBOR closure over the manifest contract,
routes, schema bindings, and raw identities of all 24 non-manifest members.
The manifest file has a separate raw content identity.

Regenerate the package only from the repository root:

```bash
cargo +1.90.0 run --locked -p echo-wesley-gen \
  --bin echo-edict-provider-package --
```

Check the exact tree without repairing or rewriting it:

```bash
cargo +1.90.0 run --locked -p echo-wesley-gen \
  --bin echo-edict-provider-package -- --check
```

The publisher fails before filesystem access unless every packaged
`generated/` member is byte-identical to the checked #652 corpus. The checked
package proves a reproducible distribution occurrence. It does not by itself
prove Edict schema admission, component-host readiness, Echo installation,
runtime authority, invocation, execution, commitment, observation, or receipt.
