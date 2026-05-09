<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-wesley-gen

CLI tool that emits Echo Rust structs, operation registries, and optic helper
functions from Wesley contract data.

The preferred input is GraphQL SDL lowered directly through the published
`wesley-core` crate. The older `echo-ir/v1` JSON stdin path is retained for
fixtures and compatibility while consumers move off the historical JavaScript
generator.

## Usage

```bash
# Generate Rust code directly from GraphQL SDL
cargo run -p echo-wesley-gen -- --schema schema.graphql

# Write generated Rust from GraphQL SDL to a file
cargo run -p echo-wesley-gen -- --schema schema.graphql --out generated.rs

# Generate Rust code to stdout
cat ir.json | cargo run -p echo-wesley-gen --

# Write to a file
cat ir.json | cargo run -p echo-wesley-gen -- --out generated.rs
```

## Notes

- Supports ENUM and OBJECT kinds from Wesley IR.
- Preserves per-operation directive metadata as `OpDef::directives_json`; Echo
  admission tooling owns any interpretation of `wes_footprint`.
- Emits footprint certificate constants for operations with `@wes_footprint`;
  those certificates include the generated Rust artifact manifest hash and the
  operation argument shape, and hosts can verify them through
  `echo_registry_api::verify_contract_artifact` before treating the generated
  artifact as compile-time-certified.
- GraphQL SDL operation ids are derived deterministically and fail closed on
  collision. The generator never increments a collided id because operation ids
  are persisted ABI.
- Optional fields become `Option<T>`; lists become `Vec<T>` (wrapped in Option when not required).
- Unknown scalar names are emitted as identifiers as-is (so ensure upstream IR types are valid Rust idents).
