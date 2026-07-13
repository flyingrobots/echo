<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ARCHITECTURE — Superseded

> **Superseded:** This root architecture snapshot was frozen before Echo's
> “There Is No Graph” rewrite. The stable path remains as a signpost; it is not
> current architectural authority.

Start with the [documentation map](docs/README.md), then use the document that
owns the boundary you are changing:

- [There Is No Graph](docs/architecture/there-is-no-graph.md) — substrate and
  observer-reading ontology;
- [Continuum Transport](docs/architecture/continuum-transport.md) — witnessed
  causal-history exchange;
- [Application Contract Hosting](docs/architecture/application-contract-hosting.md)
  — application/runtime ownership;
- [Runtime Authority](docs/topics/RuntimeAuthority.md),
  [Runtime Constellation](docs/topics/RuntimeConstellation.md), and
  [WARP Optics](docs/topics/WarpOptics.md) — living cross-module doctrine;
- [specifications](docs/spec/), [invariants](docs/invariants/), and
  [ADRs](docs/adr/) — executable contracts and durable decisions.

The current north star is:

```text
There is witnessed causal history.
WARP optics chart it.
Holograms witness those charts.
Materialized graphs are optional readings.
Continuum is the protocol for lawful causal-history exchange.
```

The former graph-engine, Scene/TTD-port, and snapshot-pipeline description is
preserved in Git history rather than mixed into current doctrine.
