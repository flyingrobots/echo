<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-ttd-gen

TTD Protocol Code Generator for Echo.

Consumes TTD IR (JSON) emitted by Wesley's TTD compiler and generates Rust artifacts:

- **Types**: Rust structs/enums for channels, ops, rules
- **Registries**: Channel, op, and rule lookup tables
- **CBOR Codecs**: Canonical encode/decode implementations (planned)
- **Footprint Specs**: Static footprint declarations for rules

## Architecture

```text
Wesley (JS/TS)                    Echo (Rust)
┌─────────────────┐              ┌──────────────────┐
│ GraphQL SDL     │              │                  │
│ + TTD Directives│──────────────│ echo-ttd-gen     │
│        │        │   TTD IR     │   (syn/quote)    │
│        ▼        │   (JSON)     │        │         │
│ TTD Compiler    │──────────────│        ▼         │
│        │        │              │ Rust Artifacts   │
│        ▼        │              │ - types.rs       │
│ ttd-ir.json     │              │ - registry.rs    │
└─────────────────┘              │ - footprints.rs  │
                                 └──────────────────┘
```

## Usage

```bash
# Pipe TTD IR from Wesley to echo-ttd-gen
cat ttd-ir.json | cargo run -p echo-ttd-gen -- -o src/generated/ttd.rs

# Or in a build script / xtask
wesley compile-ttd schema.graphql | echo-ttd-gen -o src/generated/ttd.rs
```

## TTD IR Schema

The TTD IR extends the base Wesley IR with TTD-specific concepts:

```json
{
  "ir_version": "ttd-ir/v1",
  "schema_hash": "abc123...",
  "channels": [...],
  "ops": [...],
  "rules": [...],
  "invariants": [...],
  "emission_contracts": [...],
  "footprint_specs": [...]
}
```

See `docs/specs/ttd-ir-v1.md` for the full schema specification.

## License

Apache-2.0
