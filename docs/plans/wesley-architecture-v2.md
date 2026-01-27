<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley Architecture v2: Rust Core with Per-Type Versioning

**Status:** Proposal
**Date:** 2026-01-27
**Author:** Architecture Review
**Supersedes:** Current JS-based Wesley implementation

---

## Executive Summary

### Current Problems

1. **Wesley is JavaScript**: The schema compiler exists as a separate JS/Node project (`~/git/Wesley`). This creates:
    - Cross-language boundary friction (Node subprocess invocation from Rust tooling)
    - Version synchronization complexity (wesley.lock tracks external repo state)
    - Cannot be embedded directly in Rust build pipelines
    - Cannot compile to WASM for browser-based schema tooling

2. **App-Specific Schemas in Wesley**: The TTD protocol schema currently lives in the Wesley repo (`schemas/ttd-protocol.graphql`). This violates separation of concerns:
    - Wesley should be a generic compiler, not an app container
    - Echo-specific schemas should live in Echo
    - External repos shouldn't define Echo's protocol universe

3. **Scattered Generated Artifacts**: Current layout spreads Wesley outputs across:
    - `crates/ttd-manifest/` (JSON manifests)
    - `crates/ttd-protocol-rs/` (generated Rust)
    - `packages/ttd-protocol-ts/` (generated TypeScript)
    - `docs/wesley/wesley.lock` (provenance)

    No single "janus" subsystem groups TTD concerns.

4. **No Per-Type Versioning**: Current schema hashing is coarse-grained:
    - Single `schema_hash` covers entire schema
    - No tracking of individual type versions
    - No wire format compatibility tracking
    - Breaking changes require full schema bump

### Proposed Solution

1. **Wesley becomes a Rust crate** that exports a library API. The crate can also compile to WASM for browser/Node backward compatibility.

2. **Schemas live in Echo** under a clear `janus/` subsystem directory. Wesley is schema-agnostic.

3. **Unified project structure** with obvious naming conventions (`*.generated.rs`, `*.generated.ts`).

4. **Per-type versioning** with stable registry IDs, semantic versions, content hashes, and wire format compatibility tracking.

---

## Phase 1: Project Reorganization (Echo)

### New Directory Structure

```
echo/
├── janus/                              # Echo's TTD subsystem (time-travel debugging)
│   ├── wesley/                         # Wesley integration point
│   │   ├── schema/                     # Source schemas (SINGLE SOURCE OF TRUTH)
│   │   │   ├── ttd-protocol.graphql    # TTD protocol schema
│   │   │   └── echo-core.graphql       # Future: core Echo schemas
│   │   │
│   │   ├── generated/                  # All Wesley outputs (gitignored or committed)
│   │   │   ├── rust/
│   │   │   │   ├── ttd_protocol.generated.rs
│   │   │   │   ├── echo_core.generated.rs
│   │   │   │   └── mod.rs              # Re-exports all generated modules
│   │   │   │
│   │   │   ├── typescript/
│   │   │   │   ├── ttd-protocol.generated.ts
│   │   │   │   ├── ttd-protocol.zod.generated.ts
│   │   │   │   └── index.ts            # Re-exports
│   │   │   │
│   │   │   └── manifest/
│   │   │       ├── ttd-protocol.manifest.json
│   │   │       ├── ttd-protocol.ir.json
│   │   │       └── ttd-protocol.contracts.json
│   │   │
│   │   └── wesley.lock                 # Provenance + version tracking
│   │
│   ├── compliance/                     # Compliance engine (existing echo-ttd)
│   └── browser/                        # Browser WASM module (existing ttd-browser)
│
├── crates/
│   ├── wesley/                         # NEW: Wesley Rust core (library + CLI)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Public API
│   │       ├── parser/                 # GraphQL SDL parsing
│   │       ├── ir/                     # Intermediate representation
│   │       ├── codegen/                # Code generation backends
│   │       └── hash.rs                 # Schema/type hashing
│   │
│   ├── wesley-wasm/                    # WASM bindings for Wesley
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   │
│   ├── janus-types/                    # Replaces ttd-protocol-rs (generated code lives here)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   └── lib.rs                  # Includes generated code via include!()
│   │   └── build.rs                    # Optional: regenerate on schema change
│   │
│   └── janus-compliance/               # Replaces echo-ttd
│       └── ...
│
└── packages/
    └── janus-types-ts/                 # Replaces ttd-protocol-ts
        ├── package.json
        └── src/
            └── index.ts                # Re-exports generated types
```

### Migration Steps

#### Step 1: Create janus directory structure

```bash
# Create new structure
mkdir -p janus/wesley/schema
mkdir -p janus/wesley/generated/{rust,typescript,manifest}
mkdir -p janus/compliance
mkdir -p janus/browser

# Move schema from Wesley repo to Echo
cp ~/git/Wesley/schemas/ttd-protocol.graphql janus/wesley/schema/

# Remove schema from Wesley repo (Wesley becomes schema-agnostic)
rm ~/git/Wesley/schemas/ttd-protocol.graphql
```

#### Step 2: Update existing crate references

```toml
# In Cargo.toml workspace members, rename:
# - "crates/ttd-protocol-rs" -> "crates/janus-types"
# - "crates/echo-ttd" -> "crates/janus-compliance"
# - "crates/ttd-browser" -> "crates/janus-browser"
```

#### Step 3: Update xtask paths

```rust
// xtask/src/main.rs - update all paths:
const SCHEMA_DIR: &str = "janus/wesley/schema";
const GENERATED_RUST: &str = "janus/wesley/generated/rust";
const GENERATED_TS: &str = "janus/wesley/generated/typescript";
const GENERATED_MANIFEST: &str = "janus/wesley/generated/manifest";
const WESLEY_LOCK: &str = "janus/wesley/wesley.lock";
```

### Updated File Naming Conventions

| Old Pattern          | New Pattern          | Example                         |
| -------------------- | -------------------- | ------------------------------- |
| `lib.rs` (generated) | `*.generated.rs`     | `ttd_protocol.generated.rs`     |
| `types.ts`           | `*.generated.ts`     | `ttd-protocol.generated.ts`     |
| `zod.ts`             | `*.zod.generated.ts` | `ttd-protocol.zod.generated.ts` |
| `manifest.json`      | `*.manifest.json`    | `ttd-protocol.manifest.json`    |
| `ttd-ir.json`        | `*.ir.json`          | `ttd-protocol.ir.json`          |

**Rationale:** The `.generated.` suffix makes it immediately obvious which files are machine-generated and should never be manually edited.

---

## Phase 2: Wesley Rust Core

### Architecture Overview

```
                    ┌─────────────────────────────────────────┐
                    │             Wesley Rust Core            │
                    │                                         │
   GraphQL SDL ────►│  ┌─────────┐  ┌────┐  ┌────────────┐   │
                    │  │ Parser  │─►│ IR │─►│  Codegen   │   │────► Rust
                    │  │(apollo) │  │    │  │  Backends  │   │────► TypeScript
                    │  └─────────┘  └────┘  └────────────┘   │────► JSON Manifest
                    │       │                    │           │
                    │       ▼                    ▼           │
                    │  ┌─────────┐         ┌─────────┐      │
                    │  │Directive│         │  Hash   │      │
                    │  │Extractor│         │ Engine  │      │
                    │  └─────────┘         └─────────┘      │
                    │                                         │
                    └─────────────────────────────────────────┘
                                     │
                                     ▼
                    ┌─────────────────────────────────────────┐
                    │           wesley-wasm (WASM)            │
                    │  JavaScript/TypeScript compatibility    │
                    └─────────────────────────────────────────┘
```

### Module Structure

```rust
// crates/wesley/src/lib.rs
pub mod parser;      // GraphQL SDL parsing
pub mod ir;          // Intermediate representation
pub mod directive;   // Directive extraction and validation
pub mod hash;        // Schema and type hashing
pub mod codegen;     // Code generation backends
pub mod error;       // Error types

// Re-exports for convenient API
pub use parser::Parser;
pub use ir::{SchemaIR, TypeDef, ChannelDef, OpDef};
pub use hash::{SchemaHash, TypeHash};
pub use codegen::{RustBackend, TypeScriptBackend, ManifestBackend};
```

### Recommended Crates

```toml
# crates/wesley/Cargo.toml
[package]
name = "wesley"
version = "0.2.0"
edition = "2021"
description = "Schema compiler for deterministic protocol generation"

[features]
default = ["cli"]
cli = ["clap"]
wasm = ["wasm-bindgen"]

[dependencies]
# Parsing
apollo-parser = "0.8"           # Production-grade GraphQL parser
apollo-compiler = "1.0"         # Schema validation

# Code generation
proc-macro2 = "1.0"
quote = "1.0"
syn = { version = "2.0", features = ["full"] }
prettyplease = "0.2"            # Rust code formatting

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Hashing
blake3 = "1.5"

# CLI (optional)
clap = { version = "4.4", features = ["derive"], optional = true }

# Error handling
thiserror = "1.0"
miette = "7.0"                  # Pretty error diagnostics

# WASM (optional)
wasm-bindgen = { version = "0.2", optional = true }

[dev-dependencies]
insta = "1.34"                  # Snapshot testing for codegen
```

### Parser Module

```rust
// crates/wesley/src/parser.rs
use apollo_parser::{Parser as ApolloParser, SyntaxTree};
use crate::ir::SchemaIR;
use crate::error::WesleyError;

pub struct Parser {
    /// Known directive names for validation
    known_directives: HashSet<String>,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            known_directives: Self::default_directives(),
        }
    }

    /// Parse GraphQL SDL into Wesley IR
    pub fn parse(&self, source: &str) -> Result<SchemaIR, WesleyError> {
        let parser = ApolloParser::new(source);
        let tree: SyntaxTree = parser.parse();

        // Check for parse errors
        if tree.errors().len() > 0 {
            return Err(WesleyError::ParseErrors(tree.errors().collect()));
        }

        // Extract document
        let document = tree.document();

        // Build IR from AST
        self.build_ir(document)
    }

    fn default_directives() -> HashSet<String> {
        [
            // Determinism
            "canonicalCbor", "noFloat", "fixed", "sorted", "noUnorderedMap", "keyBytes",
            // Channels
            "wes_channel", "emitKey", "entryType",
            // Ops
            "wes_op", "wes_produces", "wes_emission", "wes_emitsTo", "wes_footprint",
            // Registry
            "wes_codec", "wes_registry", "wes_version", "wes_stateField", "wes_constraint",
            // Invariants
            "wes_invariant",
        ].into_iter().map(String::from).collect()
    }
}
```

### IR Module

```rust
// crates/wesley/src/ir.rs
use serde::{Serialize, Deserialize};

/// Complete schema intermediate representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaIR {
    /// IR format version for compatibility
    pub ir_version: String,

    /// Schema-level metadata
    pub metadata: SchemaMetadata,

    /// Type definitions
    pub types: Vec<TypeDef>,

    /// Enum definitions
    pub enums: Vec<EnumDef>,

    /// Channel definitions
    pub channels: Vec<ChannelDef>,

    /// Operation definitions
    pub ops: Vec<OpDef>,

    /// Invariant definitions
    pub invariants: Vec<InvariantDef>,

    /// Type registry (stable IDs)
    pub registry: Vec<RegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    /// Hash of the entire schema (for coarse compatibility)
    pub schema_hash: String,

    /// Generation timestamp
    pub generated_at: String,

    /// Source file paths
    pub source_files: Vec<String>,

    /// Wesley version used
    pub wesley_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub directives: Vec<DirectiveUse>,

    /// Per-type versioning
    pub version: TypeVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeVersion {
    /// Stable registry ID (never changes)
    pub registry_id: u32,

    /// Semantic version (major.minor)
    pub major: u16,
    pub minor: u16,

    /// Content hash of type definition
    pub content_hash: String,

    /// Wire format version (for compatibility tracking)
    pub wire_format: WireFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireFormat {
    /// CBOR tag (if applicable)
    pub cbor_tag: Option<u64>,

    /// Field ordering hash (canonical order)
    pub field_order_hash: String,

    /// Compatible with these previous wire versions
    pub backward_compatible_with: Vec<String>,
}
```

### Codegen Module

```rust
// crates/wesley/src/codegen/mod.rs
pub mod rust;
pub mod typescript;
pub mod manifest;

use crate::ir::SchemaIR;
use crate::error::WesleyError;

/// Code generation backend trait
pub trait CodegenBackend {
    type Output;

    fn generate(&self, ir: &SchemaIR) -> Result<Self::Output, WesleyError>;
}

/// Rust code generation options
pub struct RustOptions {
    /// Add #[derive(Debug, Clone, ...)] automatically
    pub auto_derives: bool,

    /// Generate serde impls
    pub with_serde: bool,

    /// Generate CBOR codecs
    pub with_cbor: bool,

    /// Module documentation
    pub module_doc: Option<String>,
}

impl Default for RustOptions {
    fn default() -> Self {
        Self {
            auto_derives: true,
            with_serde: true,
            with_cbor: true,
            module_doc: None,
        }
    }
}
```

```rust
// crates/wesley/src/codegen/rust.rs
use proc_macro2::TokenStream;
use quote::{quote, format_ident};
use crate::ir::SchemaIR;
use super::{CodegenBackend, RustOptions};

pub struct RustBackend {
    options: RustOptions,
}

impl RustBackend {
    pub fn new(options: RustOptions) -> Self {
        Self { options }
    }
}

impl CodegenBackend for RustBackend {
    type Output = String;

    fn generate(&self, ir: &SchemaIR) -> Result<String, WesleyError> {
        let mut tokens = self.generate_header(ir);
        tokens.extend(self.generate_constants(ir));
        tokens.extend(self.generate_enums(ir));
        tokens.extend(self.generate_types(ir));
        tokens.extend(self.generate_registry(ir));

        // Format with prettyplease
        let syntax_tree = syn::parse2(tokens)?;
        Ok(prettyplease::unparse(&syntax_tree))
    }
}

impl RustBackend {
    fn generate_header(&self, ir: &SchemaIR) -> TokenStream {
        let schema_hash = &ir.metadata.schema_hash;
        let generated_at = &ir.metadata.generated_at;
        let wesley_version = &ir.metadata.wesley_version;

        quote! {
            //! Generated by Wesley. DO NOT EDIT.
            //!
            //! Schema hash: #schema_hash
            //! Generated at: #generated_at
            //! Wesley version: #wesley_version

            #![allow(dead_code, non_snake_case, non_camel_case_types)]

            use serde::{Serialize, Deserialize};

            /// SHA256 hash of the source schema.
            pub const SCHEMA_HASH: &str = #schema_hash;

            /// Timestamp when this code was generated.
            pub const GENERATED_AT: &str = #generated_at;
        }
    }

    // ... additional methods for enums, types, registry
}
```

### WASM Compilation Strategy

```rust
// crates/wesley-wasm/src/lib.rs
use wasm_bindgen::prelude::*;
use wesley::{Parser, codegen::{RustBackend, TypeScriptBackend, ManifestBackend}};

/// Wesley WASM API for browser/Node.js usage
#[wasm_bindgen]
pub struct WesleyCompiler {
    parser: Parser,
}

#[wasm_bindgen]
impl WesleyCompiler {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
        }
    }

    /// Parse schema and return JSON IR
    #[wasm_bindgen]
    pub fn parse_to_ir(&self, source: &str) -> Result<String, JsValue> {
        let ir = self.parser.parse(source)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        serde_json::to_string_pretty(&ir)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Generate TypeScript types
    #[wasm_bindgen]
    pub fn generate_typescript(&self, source: &str) -> Result<String, JsValue> {
        let ir = self.parser.parse(source)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let backend = TypeScriptBackend::new(Default::default());
        backend.generate(&ir)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Compute schema hash
    #[wasm_bindgen]
    pub fn schema_hash(&self, source: &str) -> Result<String, JsValue> {
        let ir = self.parser.parse(source)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(ir.metadata.schema_hash.clone())
    }
}
```

### Library API Design

```rust
// Example usage of Wesley as a library
use wesley::{Parser, SchemaIR, codegen::{RustBackend, RustOptions}};

fn main() -> anyhow::Result<()> {
    // Parse schema
    let source = std::fs::read_to_string("janus/wesley/schema/ttd-protocol.graphql")?;
    let parser = Parser::new();
    let ir: SchemaIR = parser.parse(&source)?;

    // Generate Rust code
    let rust_backend = RustBackend::new(RustOptions {
        with_cbor: true,
        ..Default::default()
    });
    let rust_code = rust_backend.generate(&ir)?;

    // Write output
    std::fs::write(
        "janus/wesley/generated/rust/ttd_protocol.generated.rs",
        rust_code
    )?;

    Ok(())
}
```

---

## Phase 3: Per-Type Versioning

### Design Goals

1. **Stable Registry IDs**: Each type gets a permanent numeric ID that never changes
2. **Semantic Versioning**: Types have major.minor versions for compatibility signaling
3. **Content Hashing**: Types have content hashes for exact match detection
4. **Wire Format Tracking**: Track CBOR/binary representation compatibility

### TypeId/SchemaId Design

```rust
// crates/wesley/src/hash.rs
use blake3::Hasher;

/// Stable identifier for a type in the registry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeId(pub u32);

impl TypeId {
    /// Generate TypeId from type name (deterministic)
    pub fn from_name(name: &str) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(b"type_id:v1:");
        hasher.update(name.as_bytes());
        let hash = hasher.finalize();
        // Use first 4 bytes as u32
        let bytes: [u8; 4] = hash.as_bytes()[..4].try_into().unwrap();
        Self(u32::from_le_bytes(bytes))
    }
}

/// Complete type version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeVersion {
    /// Stable registry ID
    pub id: TypeId,

    /// Semantic version
    pub major: u16,
    pub minor: u16,

    /// Content hash (hash of normalized type definition)
    pub content_hash: ContentHash,

    /// Wire format compatibility
    pub wire: WireCompatibility,
}

/// Content hash of a type definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    /// Compute content hash from type definition
    pub fn compute(type_def: &TypeDef) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(b"content_hash:v1:");
        hasher.update(type_def.name.as_bytes());

        // Hash fields in canonical order
        let mut fields: Vec<_> = type_def.fields.iter().collect();
        fields.sort_by_key(|f| &f.name);

        for field in fields {
            hasher.update(b"|field:");
            hasher.update(field.name.as_bytes());
            hasher.update(b":");
            hasher.update(field.type_name.as_bytes());
            hasher.update(if field.required { b":req" } else { b":opt" });
            hasher.update(if field.list { b":list" } else { b":single" });
        }

        Self(hasher.finalize().into())
    }
}

/// Wire format compatibility tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireCompatibility {
    /// Current wire format version
    pub version: u16,

    /// CBOR tag (if using tagged CBOR)
    pub cbor_tag: Option<u64>,

    /// Hash of field order (for canonical encoding)
    pub field_order_hash: [u8; 8],

    /// Previous versions this is backward-compatible with
    pub backward_compat: Vec<u16>,

    /// Previous versions this can forward-read
    pub forward_compat: Vec<u16>,
}
```

### Registry Format

```json
// janus/wesley/generated/manifest/ttd-protocol.registry.json
{
    "registry_version": "v1",
    "schema_hash": "abc123...",
    "types": {
        "CursorMoved": {
            "id": 1234567890,
            "version": {
                "major": 1,
                "minor": 0
            },
            "content_hash": "def456...",
            "wire": {
                "version": 1,
                "cbor_tag": 1001,
                "field_order_hash": "789abc...",
                "backward_compat": [],
                "forward_compat": []
            }
        },
        "SeekCompleted": {
            "id": 2345678901,
            "version": {
                "major": 1,
                "minor": 0
            },
            "content_hash": "ghi789...",
            "wire": {
                "version": 1,
                "cbor_tag": 1002,
                "field_order_hash": "012def...",
                "backward_compat": [],
                "forward_compat": []
            }
        }
    },
    "enums": {
        "CursorRole": {
            "id": 3456789012,
            "version": {
                "major": 1,
                "minor": 0
            },
            "values": ["WRITER", "READER"]
        }
    },
    "channels": {
        "ttd.head": {
            "id": 4567890123,
            "version": 1,
            "policy": "STRICT_SINGLE",
            "reducer": "LAST"
        }
    }
}
```

### Wire Format Embedding

Types embed their registry ID in the wire format for self-describing messages:

```rust
// Generated code includes wire format helpers
impl CursorMoved {
    /// Registry ID for this type
    pub const REGISTRY_ID: u32 = 1234567890;

    /// CBOR tag for self-describing encoding
    pub const CBOR_TAG: u64 = 1001;

    /// Wire format version
    pub const WIRE_VERSION: u16 = 1;

    /// Encode to CBOR with registry tag
    pub fn encode_tagged(&self) -> Vec<u8> {
        let mut encoder = minicbor::Encoder::new(Vec::new());
        encoder.tag(minicbor::data::Tag::new(Self::CBOR_TAG)).unwrap();
        self.encode(&mut encoder).unwrap();
        encoder.into_inner()
    }

    /// Decode from CBOR, verifying tag
    pub fn decode_tagged(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut decoder = minicbor::Decoder::new(bytes);
        let tag = decoder.tag()?;
        if tag.as_u64() != Self::CBOR_TAG {
            return Err(DecodeError::TagMismatch {
                expected: Self::CBOR_TAG,
                actual: tag.as_u64(),
            });
        }
        Self::decode(&mut decoder)
    }
}
```

### Compatibility Checking

```rust
// crates/wesley/src/compat.rs

/// Check wire compatibility between schema versions
pub fn check_compatibility(
    old_registry: &Registry,
    new_registry: &Registry,
) -> CompatibilityReport {
    let mut report = CompatibilityReport::default();

    for (type_name, new_type) in &new_registry.types {
        match old_registry.types.get(type_name) {
            None => {
                // New type added (always compatible)
                report.additions.push(TypeChange::Added(type_name.clone()));
            }
            Some(old_type) => {
                // Check for breaking changes
                if old_type.id != new_type.id {
                    report.errors.push(CompatError::RegistryIdChanged {
                        type_name: type_name.clone(),
                        old_id: old_type.id,
                        new_id: new_type.id,
                    });
                }

                if new_type.version.major > old_type.version.major {
                    // Major version bump - breaking change expected
                    report.breaking.push(TypeChange::MajorBump {
                        type_name: type_name.clone(),
                        old: old_type.version.clone(),
                        new: new_type.version.clone(),
                    });
                }

                if new_type.content_hash != old_type.content_hash {
                    // Content changed
                    report.changes.push(TypeChange::Modified {
                        type_name: type_name.clone(),
                        old_hash: old_type.content_hash.clone(),
                        new_hash: new_type.content_hash.clone(),
                    });
                }
            }
        }
    }

    // Check for removed types
    for type_name in old_registry.types.keys() {
        if !new_registry.types.contains_key(type_name) {
            report.errors.push(CompatError::TypeRemoved(type_name.clone()));
        }
    }

    report
}
```

---

## Phase 4: Integration

### How Echo Consumes Wesley-as-Library

#### Option A: build.rs (Compile-time generation)

```rust
// crates/janus-types/build.rs
use wesley::{Parser, codegen::RustBackend};
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=../../janus/wesley/schema/");

    let schema_dir = Path::new("../../janus/wesley/schema");
    let output_dir = Path::new("../../janus/wesley/generated/rust");

    // Find all schema files
    for entry in std::fs::read_dir(schema_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map_or(false, |e| e == "graphql") {
            let stem = path.file_stem().unwrap().to_str().unwrap();
            let source = std::fs::read_to_string(&path).unwrap();

            let parser = Parser::new();
            let ir = parser.parse(&source).unwrap();

            let backend = RustBackend::new(Default::default());
            let code = backend.generate(&ir).unwrap();

            let output_path = output_dir.join(format!("{}.generated.rs", stem.replace('-', "_")));
            std::fs::write(&output_path, code).unwrap();
        }
    }
}
```

#### Option B: xtask (Recommended - Explicit invocation)

```rust
// xtask/src/wesley.rs
use anyhow::Result;
use wesley::{Parser, SchemaIR, codegen::{RustBackend, TypeScriptBackend, ManifestBackend}};
use std::path::Path;

pub fn sync(dry_run: bool) -> Result<()> {
    let schema_dir = Path::new("janus/wesley/schema");
    let generated_dir = Path::new("janus/wesley/generated");

    let parser = Parser::new();

    for entry in std::fs::read_dir(schema_dir)? {
        let path = entry?.path();
        if path.extension().map_or(false, |e| e == "graphql") {
            let stem = path.file_stem().unwrap().to_str().unwrap();
            let source = std::fs::read_to_string(&path)?;

            println!("Compiling: {}", path.display());

            let ir = parser.parse(&source)?;

            if !dry_run {
                // Generate Rust
                let rust_backend = RustBackend::new(Default::default());
                let rust_code = rust_backend.generate(&ir)?;
                let rust_path = generated_dir.join("rust")
                    .join(format!("{}.generated.rs", stem.replace('-', "_")));
                std::fs::write(&rust_path, &rust_code)?;
                println!("  Wrote: {}", rust_path.display());

                // Generate TypeScript
                let ts_backend = TypeScriptBackend::new(Default::default());
                let ts_code = ts_backend.generate(&ir)?;
                let ts_path = generated_dir.join("typescript")
                    .join(format!("{}.generated.ts", stem));
                std::fs::write(&ts_path, &ts_code)?;
                println!("  Wrote: {}", ts_path.display());

                // Generate manifest
                let manifest_backend = ManifestBackend::new();
                let manifest = manifest_backend.generate(&ir)?;
                let manifest_path = generated_dir.join("manifest")
                    .join(format!("{}.manifest.json", stem));
                std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
                println!("  Wrote: {}", manifest_path.display());
            }
        }
    }

    // Update wesley.lock
    update_lock_file(dry_run)?;

    Ok(())
}
```

**Recommendation:** Use xtask approach. Reasons:

1. **Explicit invocation** - No surprise regeneration during builds
2. **Better error messages** - Not buried in cargo build output
3. **CI-friendly** - Can run check without regenerating
4. **Avoids build script complexity** - build.rs can be finicky with dependencies

### CI/CD Changes

```yaml
# .github/workflows/ci.yml

jobs:
    wesley-check:
        name: Wesley Artifacts Check
        runs-on: ubuntu-latest
        steps:
            - uses: actions/checkout@v4

            - name: Install Rust
              uses: dtolnay/rust-action@stable

            - name: Check Wesley artifacts are up to date
              run: |
                  cargo xtask wesley sync --dry-run
                  if ! git diff --quiet janus/wesley/generated/; then
                    echo "ERROR: Wesley artifacts are out of date"
                    echo "Run 'cargo xtask wesley sync' locally and commit the changes"
                    git diff janus/wesley/generated/
                    exit 1
                  fi

    build:
        needs: wesley-check
        # ... rest of build
```

### Pre-commit Hook

```bash
#!/bin/bash
# .githooks/pre-commit

# Check if schema files changed
if git diff --cached --name-only | grep -q "janus/wesley/schema/"; then
    echo "Schema files changed, checking Wesley artifacts..."

    cargo xtask wesley check || {
        echo ""
        echo "ERROR: Wesley artifacts are stale."
        echo "Run 'cargo xtask wesley sync' to regenerate."
        exit 1
    }
fi
```

---

## Migration Path

### Step-by-Step Migration from Current State

#### Week 1: Directory Restructure

1. Create `janus/` directory structure
2. Move `schemas/ttd-protocol.graphql` to `janus/wesley/schema/`
3. Create symlinks or update imports for existing code
4. Update xtask paths
5. Verify CI still passes

**Backward Compatibility:** Keep old paths as symlinks temporarily

```bash
# Temporary symlinks during migration
ln -s janus/wesley/generated/manifest crates/ttd-manifest
ln -s janus/wesley/generated/rust/ttd_protocol.generated.rs crates/ttd-protocol-rs/lib.rs
```

#### Week 2: Wesley Rust Crate

1. Create `crates/wesley/` with parser and IR modules
2. Port GraphQL parsing from JS to Rust using apollo-parser
3. Implement basic Rust codegen (no CBOR yet)
4. Add snapshot tests comparing output with current JS Wesley

**Verification:** New Wesley should produce identical output to JS Wesley

#### Week 3: Per-Type Versioning

1. Add TypeId and version tracking to IR
2. Update codegen to emit version constants
3. Create registry.json manifest format
4. Add compatibility checking

#### Week 4: WASM Build

1. Create `crates/wesley-wasm/` with wasm-bindgen
2. Build WASM module for browser use
3. Update any JS tooling to use WASM Wesley instead of Node Wesley
4. Remove dependency on external Wesley repo

#### Week 5: Cleanup

1. Remove symlinks
2. Delete deprecated paths from workspace
3. Remove Wesley repo from external dependencies
4. Update all documentation
5. Archive JS Wesley repo (or mark deprecated)

### Backward Compatibility Considerations

1. **Schema Hash Stability**: New Wesley must produce same schema_hash as JS Wesley for identical input
2. **Generated Code API**: Keep same public API in generated Rust/TypeScript
3. **Manifest Format**: Use same JSON structure, add new fields only
4. **Wire Format**: CBOR encoding must be byte-identical

### Verification Checklist

- [ ] `cargo xtask wesley sync` works with new Rust Wesley
- [ ] Schema hash matches between JS and Rust Wesley
- [ ] Generated Rust compiles and has same public API
- [ ] Generated TypeScript has same exports
- [ ] All existing tests pass
- [ ] CI pipeline updated and green
- [ ] Documentation reflects new paths
- [ ] WASM build works in browser

---

## Appendix A: Directive Reference

### Current Directives (from ttd-protocol.graphql)

| Directive         | Target         | Purpose                                 |
| ----------------- | -------------- | --------------------------------------- |
| `@wes_channel`    | Type           | Declare a materialization channel       |
| `@wes_codec`      | Type           | Specify encoding format (cbor, json)    |
| `@wes_registry`   | Type           | Assign stable registry ID               |
| `@wes_version`    | Type           | Declare type version                    |
| `@wes_stateField` | Field          | Mark as state field (with optional key) |
| `@wes_constraint` | Field          | Add validation constraint               |
| `@wes_op`         | Mutation/Query | Declare an operation                    |
| `@wes_produces`   | Mutation       | List events this op can produce         |
| `@wes_emission`   | Mutation       | Declare emission to channel             |
| `@wes_emitsTo`    | Mutation       | Emission with timing constraint         |
| `@wes_footprint`  | Mutation/Query | Declare read/write sets                 |
| `@wes_invariant`  | Type           | Declare system invariant                |

### Proposed New Directives for v2

| Directive          | Target     | Purpose                           |
| ------------------ | ---------- | --------------------------------- |
| `@wes_typeId`      | Type       | Explicitly set stable registry ID |
| `@wes_wireVersion` | Type       | Declare wire format version       |
| `@wes_deprecated`  | Type/Field | Mark as deprecated with migration |
| `@wes_renamedFrom` | Type/Field | Track renames for compatibility   |

---

## Appendix B: Example Schema with Per-Type Versioning

```graphql
# janus/wesley/schema/ttd-protocol.graphql

scalar Hash
scalar Timestamp

type CursorMoved
    @wes_typeId(id: 1001)
    @wes_version(major: 1, minor: 0)
    @wes_codec(format: "cbor", canonical: true)
    @wes_registry(id: 1) {
    sessionId: Hash!
    cursorId: Hash!
    worldlineId: Hash!
    warpId: Hash!
    tick: Int!
    commitHash: Hash!
    timestamp: Timestamp!
}

# When adding a field, bump minor version:
type CursorMovedV1_1
    @wes_typeId(id: 1001) # Same ID - backward compatible
    @wes_version(major: 1, minor: 1)
    @wes_codec(format: "cbor", canonical: true) {
    sessionId: Hash!
    cursorId: Hash!
    worldlineId: Hash!
    warpId: Hash!
    tick: Int!
    commitHash: Hash!
    timestamp: Timestamp!
    reason: String # NEW: optional field (minor bump)
}

# When removing/changing fields, bump major version:
type CursorMovedV2
    @wes_typeId(id: 1001) # Same ID
    @wes_version(major: 2, minor: 0)
    @wes_codec(format: "cbor", canonical: true)
    @wes_renamedFrom(type: "CursorMoved", version: "1.x") {
    session: Hash! # CHANGED: renamed from sessionId
    cursor: Hash! # CHANGED: renamed from cursorId
    # worldlineId REMOVED - breaking change
    tick: Int!
    commit: Hash! # CHANGED: renamed from commitHash
    timestamp: Timestamp!
}
```

---

## Appendix C: File Reference

### Current Files (to be migrated)

| Current Path                   | New Path                                             |
| ------------------------------ | ---------------------------------------------------- |
| `~/git/Wesley/`                | Archived (functionality moves to `crates/wesley/`)   |
| `schemas/ttd-protocol.graphql` | `janus/wesley/schema/ttd-protocol.graphql`           |
| `crates/ttd-protocol-rs/`      | `crates/janus-types/`                                |
| `crates/ttd-manifest/`         | `janus/wesley/generated/manifest/`                   |
| `packages/ttd-protocol-ts/`    | `packages/janus-types-ts/`                           |
| `docs/wesley/wesley.lock`      | `janus/wesley/wesley.lock`                           |
| `crates/echo-ttd-gen/`         | Deprecated (functionality moves to `crates/wesley/`) |
| `crates/echo-wesley-gen/`      | Deprecated (functionality moves to `crates/wesley/`) |

### New Files (to be created)

| Path                                         | Purpose                   |
| -------------------------------------------- | ------------------------- |
| `crates/wesley/Cargo.toml`                   | Wesley Rust core crate    |
| `crates/wesley/src/lib.rs`                   | Wesley library API        |
| `crates/wesley-wasm/Cargo.toml`              | WASM bindings             |
| `janus/wesley/schema/`                       | Schema source directory   |
| `janus/wesley/generated/rust/mod.rs`         | Generated Rust re-exports |
| `janus/wesley/generated/typescript/index.ts` | Generated TS re-exports   |
