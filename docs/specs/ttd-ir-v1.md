<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# TTD IR v1 Schema Specification

This document specifies the TTD Intermediate Representation (IR) schema, version `ttd-ir/v1`. The TTD IR is the contract between the Wesley TTD compiler (TypeScript) and the Echo TTD code generator (Rust).

## Overview

The TTD IR is a JSON format that captures all the information needed to generate Rust artifacts for the Echo Time Travel Debugger. Wesley compiles GraphQL SDL with WARP directives into this IR, which `echo-ttd-gen` then consumes to produce type-safe Rust code.

```text
┌─────────────────┐         ┌──────────────────┐         ┌──────────────────┐
│ GraphQL SDL     │         │                  │         │                  │
│ + WARP          │────────▶│ Wesley Compiler  │────────▶│ echo-ttd-gen     │
│ Directives      │         │ (TypeScript)     │         │ (Rust)           │
└─────────────────┘         └──────────────────┘         └──────────────────┘
                                    │                            │
                                    ▼                            ▼
                            TTD IR (JSON)              Rust artifacts:
                            ttd-ir/v1                  - types.rs
                                                       - registry.rs
                                                       - footprints.rs
```

## Root Schema

```json
{
  "ir_version": "ttd-ir/v1",
  "schema_sha256": "string",
  "generated_by": { "tool": "string", "version": "string" },
  "generated_at": "ISO8601 timestamp",
  "channels": [...],
  "ops": [...],
  "rules": [...],
  "invariants": [...],
  "emissions": [...],
  "footprints": [...],
  "registry": [...],
  "types": [...],
  "enums": [...],
  "codecs": [...],
  "metadata": {}
}
```

### Required Fields

| Field        | Type              | Description                            |
| ------------ | ----------------- | -------------------------------------- |
| `ir_version` | `string`          | Must be `"ttd-ir/v1"` for this version |
| `channels`   | `ChannelDef[]`    | Channel definitions                    |
| `ops`        | `OpDef[]`         | Operation definitions                  |
| `rules`      | `RuleDef[]`       | State machine transition rules         |
| `invariants` | `InvariantDef[]`  | Invariant definitions                  |
| `emissions`  | `EmissionDef[]`   | Event emission declarations            |
| `footprints` | `FootprintDef[]`  | Operation footprint specifications     |
| `registry`   | `RegistryEntry[]` | Type ID registry entries               |
| `types`      | `TypeDef[]`       | Type/struct definitions                |
| `enums`      | `EnumDef[]`       | Enum definitions                       |
| `codecs`     | `CodecDef[]`      | Custom codec specifications (future)   |
| `metadata`   | `object`          | Arbitrary metadata                     |

### Optional Fields

| Field           | Type          | Description                  |
| --------------- | ------------- | ---------------------------- |
| `schema_sha256` | `string`      | SHA256 hash of source schema |
| `generated_by`  | `GeneratedBy` | Tool that generated this IR  |
| `generated_at`  | `string`      | ISO8601 timestamp            |

## Type Definitions

### ChannelDef

Defines an event channel for pub/sub messaging.

```json
{
    "kind": "CHANNEL",
    "name": "counter",
    "version": 1,
    "eventTypes": ["CounterIncremented", "CounterDecremented"],
    "ordered": true,
    "persistent": false
}
```

| Field        | Type        | Required | Description                      |
| ------------ | ----------- | -------- | -------------------------------- |
| `kind`       | `"CHANNEL"` | Yes      | Discriminator                    |
| `name`       | `string`    | Yes      | Channel identifier               |
| `version`    | `number`    | No       | Channel version (default: 1)     |
| `eventTypes` | `string[]`  | Yes      | Event types this channel carries |
| `ordered`    | `boolean`   | Yes      | Whether events are ordered       |
| `persistent` | `boolean`   | Yes      | Whether events are persisted     |

### OpDef

Defines an operation (mutation or query).

```json
{
    "kind": "OP",
    "name": "increment",
    "args": [
        { "name": "counterId", "type": "ID", "required": true, "list": false },
        { "name": "amount", "type": "Int", "required": true, "list": false }
    ],
    "resultType": "Counter",
    "idempotent": false,
    "readonly": false,
    "op_id": 2348489527,
    "rules": []
}
```

| Field        | Type       | Required | Description                     |
| ------------ | ---------- | -------- | ------------------------------- |
| `kind`       | `"OP"`     | Yes      | Discriminator                   |
| `name`       | `string`   | Yes      | Operation name                  |
| `args`       | `ArgDef[]` | Yes      | Operation arguments             |
| `resultType` | `string`   | Yes      | Return type name                |
| `idempotent` | `boolean`  | Yes      | Whether op is idempotent        |
| `readonly`   | `boolean`  | Yes      | Whether op is read-only (query) |
| `op_id`      | `number`   | Yes      | Stable numeric identifier       |
| `rules`      | `string[]` | Yes      | Associated rule names           |

### ArgDef

Defines an argument for an operation or field.

```json
{
    "name": "counterId",
    "type": "ID",
    "required": true,
    "list": false
}
```

| Field      | Type      | Required | Description                          |
| ---------- | --------- | -------- | ------------------------------------ |
| `name`     | `string`  | Yes      | Argument name                        |
| `type`     | `string`  | Yes      | Type name (GraphQL scalar or custom) |
| `required` | `boolean` | Yes      | Whether argument is required         |
| `list`     | `boolean` | Yes      | Whether argument is a list           |

### RuleDef

Defines a state machine transition rule.

```json
{
    "kind": "RULE",
    "name": "decrement_rule",
    "from": ["COUNTING"],
    "to": "COUNTING",
    "guard": "value >= amount",
    "opName": "decrement"
}
```

| Field    | Type       | Required | Description                              |
| -------- | ---------- | -------- | ---------------------------------------- |
| `kind`   | `"RULE"`   | Yes      | Discriminator                            |
| `name`   | `string`   | Yes      | Rule identifier                          |
| `from`   | `string[]` | Yes      | Valid source states                      |
| `to`     | `string`   | Yes      | Target state                             |
| `opName` | `string`   | Yes      | Associated operation                     |
| `guard`  | `string`   | No       | Guard expression (TTD expression syntax) |

### InvariantDef

Defines a system invariant (law).

```json
{
    "kind": "INVARIANT",
    "name": "value_non_negative",
    "expr": "forall c in Counter: c.value >= 0",
    "severity": "error"
}
```

| Field      | Type          | Required | Description                                   |
| ---------- | ------------- | -------- | --------------------------------------------- |
| `kind`     | `"INVARIANT"` | Yes      | Discriminator                                 |
| `name`     | `string`      | Yes      | Invariant identifier                          |
| `expr`     | `string`      | Yes      | Invariant expression                          |
| `severity` | `string`      | No       | `"error"` or `"warning"` (default: `"error"`) |

### EmissionDef

Declares when an operation emits an event.

```json
{
    "kind": "EMISSION",
    "channel": "counter",
    "event": "CounterIncremented",
    "opName": "increment",
    "condition": "amount > 0"
}
```

| Field       | Type         | Required | Description            |
| ----------- | ------------ | -------- | ---------------------- |
| `kind`      | `"EMISSION"` | Yes      | Discriminator          |
| `channel`   | `string`     | Yes      | Target channel name    |
| `event`     | `string`     | No       | Event type name        |
| `opName`    | `string`     | Yes      | Triggering operation   |
| `condition` | `string`     | No       | Conditional expression |
| `withinMs`  | `number`     | No       | Timing constraint (ms) |

### FootprintDef

Declares the read/write footprint of an operation.

```json
{
    "kind": "FOOTPRINT",
    "opName": "increment",
    "reads": ["Counter"],
    "writes": ["Counter"],
    "creates": [],
    "deletes": []
}
```

| Field     | Type          | Required | Description              |
| --------- | ------------- | -------- | ------------------------ |
| `kind`    | `"FOOTPRINT"` | Yes      | Discriminator            |
| `opName`  | `string`      | Yes      | Operation name           |
| `reads`   | `string[]`    | Yes      | Types read by this op    |
| `writes`  | `string[]`    | Yes      | Types written by this op |
| `creates` | `string[]`    | Yes      | Types created by this op |
| `deletes` | `string[]`    | Yes      | Types deleted by this op |

### RegistryEntry

Maps type names to stable numeric IDs for serialization.

```json
{
    "kind": "REGISTRY_ENTRY",
    "typeName": "CounterIncremented",
    "id": 1,
    "deprecated": false
}
```

| Field        | Type               | Required | Description                |
| ------------ | ------------------ | -------- | -------------------------- |
| `kind`       | `"REGISTRY_ENTRY"` | Yes      | Discriminator              |
| `typeName`   | `string`           | Yes      | Type name                  |
| `id`         | `number`           | Yes      | Stable numeric ID          |
| `deprecated` | `boolean`          | Yes      | Whether type is deprecated |

### TypeDef

Defines a struct/object type.

```json
{
    "name": "Counter",
    "fields": [
        { "name": "id", "type": "ID", "required": true, "list": false },
        { "name": "value", "type": "Int", "required": true, "list": false },
        {
            "name": "state",
            "type": "CounterState",
            "required": true,
            "list": false
        }
    ]
}
```

| Field    | Type         | Required | Description |
| -------- | ------------ | -------- | ----------- |
| `name`   | `string`     | Yes      | Type name   |
| `fields` | `FieldDef[]` | Yes      | Type fields |

### FieldDef

Defines a field within a type.

```json
{
    "name": "value",
    "type": "Int",
    "required": true,
    "list": false
}
```

| Field      | Type      | Required | Description               |
| ---------- | --------- | -------- | ------------------------- |
| `name`     | `string`  | Yes      | Field name                |
| `type`     | `string`  | Yes      | Type name                 |
| `required` | `boolean` | Yes      | Whether field is required |
| `list`     | `boolean` | Yes      | Whether field is a list   |

### EnumDef

Defines an enumeration.

```json
{
    "name": "CounterState",
    "values": ["IDLE", "COUNTING", "PAUSED", "COMPLETED"]
}
```

| Field    | Type       | Required | Description   |
| -------- | ---------- | -------- | ------------- |
| `name`   | `string`   | Yes      | Enum name     |
| `values` | `string[]` | Yes      | Enum variants |

### CodecDef

Reserved for custom codec specifications (future).

```json
{
    "name": "CustomType",
    "codec": "cbor"
}
```

## Type Mapping

GraphQL scalar types map to Rust as follows:

| GraphQL   | Rust     | Notes                                 |
| --------- | -------- | ------------------------------------- |
| `Boolean` | `bool`   |                                       |
| `String`  | `String` |                                       |
| `Int`     | `i32`    |                                       |
| `Float`   | `f32`    |                                       |
| `ID`      | `String` | Semantic string identifier            |
| Custom    | Struct   | Generated from `TypeDef` or `EnumDef` |

## Expression Syntax

Guard expressions and invariant expressions use TTD expression syntax:

- Comparison: `value >= amount`, `count < max`
- Quantifiers: `forall c in Counter: c.value >= 0`
- Boolean logic: `active && !paused`, `a || b`
- Field access: `entity.field.subfield`

The expression syntax is designed to be compilable to Rust predicates in future versions.

## Versioning

The `ir_version` field follows semantic versioning principles:

- `ttd-ir/v1` - Current stable version
- Breaking changes require a major version bump (`ttd-ir/v2`)
- Additive changes are backwards compatible within a major version

## Example: Complete Counter IR

```json
{
    "ir_version": "ttd-ir/v1",
    "schema_sha256": "23dc0e310ad5658b898358ca389b32dc476d811a9b9a557729c7a42ca1637b46",
    "generated_by": {
        "tool": "@wesley/generator-ttd",
        "version": "0.1.0"
    },
    "generated_at": "2026-01-25T13:17:35.338Z",
    "channels": [
        {
            "kind": "CHANNEL",
            "name": "counter",
            "version": 1,
            "eventTypes": [
                "CounterIncremented",
                "CounterDecremented",
                "CounterReset"
            ],
            "ordered": true,
            "persistent": false
        }
    ],
    "ops": [
        {
            "kind": "OP",
            "name": "increment",
            "args": [
                {
                    "name": "counterId",
                    "type": "ID",
                    "required": true,
                    "list": false
                },
                {
                    "name": "amount",
                    "type": "Int",
                    "required": true,
                    "list": false
                }
            ],
            "resultType": "Counter",
            "idempotent": false,
            "readonly": false,
            "op_id": 2348489527,
            "rules": []
        }
    ],
    "rules": [
        {
            "kind": "RULE",
            "name": "stay_counting",
            "from": ["COUNTING"],
            "to": "COUNTING",
            "opName": "increment"
        },
        {
            "kind": "RULE",
            "name": "decrement_rule",
            "from": ["COUNTING"],
            "to": "COUNTING",
            "guard": "value >= amount",
            "opName": "decrement"
        }
    ],
    "invariants": [
        {
            "kind": "INVARIANT",
            "name": "value_non_negative",
            "expr": "forall c in Counter: c.value >= 0",
            "severity": "error"
        }
    ],
    "emissions": [
        {
            "kind": "EMISSION",
            "channel": "counter",
            "event": "CounterIncremented",
            "opName": "increment"
        }
    ],
    "footprints": [
        {
            "kind": "FOOTPRINT",
            "opName": "increment",
            "reads": ["Counter"],
            "writes": ["Counter"],
            "creates": [],
            "deletes": []
        }
    ],
    "registry": [
        {
            "kind": "REGISTRY_ENTRY",
            "typeName": "CounterIncremented",
            "id": 1,
            "deprecated": false
        }
    ],
    "types": [
        {
            "name": "Counter",
            "fields": [
                { "name": "id", "type": "ID", "required": true, "list": false },
                {
                    "name": "value",
                    "type": "Int",
                    "required": true,
                    "list": false
                },
                {
                    "name": "state",
                    "type": "CounterState",
                    "required": true,
                    "list": false
                }
            ]
        }
    ],
    "enums": [
        {
            "name": "CounterState",
            "values": ["IDLE", "COUNTING", "PAUSED", "COMPLETED"]
        }
    ],
    "codecs": [],
    "metadata": {
        "extractedAt": "2026-01-25T13:17:35.311Z",
        "ttdVersion": "1.0.0"
    }
}
```

## Changelog

### v1 (Initial Release)

- Initial schema specification
- Support for channels, ops, rules, invariants, emissions, footprints
- Type system with enums and structs
- Registry for stable type IDs
