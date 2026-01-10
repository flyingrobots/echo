<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
// SPDX-License-Identifier: Apache-2.0

# echo-wesley-gen

CLI tool that reads Wesley IR (JSON) from stdin and emits Rust structs/enums
for Echo. Intended to be driven by the JavaScript generator (packages/wesley-generator-echo)
which now outputs `ir.json` instead of handwritten Rust.

## Usage

```bash
# Generate Rust code to stdout
cat ir.json | cargo run -p echo-wesley-gen --

# Write to a file
cat ir.json | cargo run -p echo-wesley-gen -- --out generated.rs
```

## Notes
- Supports ENUM and OBJECT kinds from Wesley IR.
- Optional fields become `Option<T>`; lists become `Vec<T>` (wrapped in Option when not required).
- Unknown scalar names are emitted as identifiers as-is (so ensure upstream IR types are valid Rust idents).
