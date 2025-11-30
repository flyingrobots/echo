<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Runtime Configuration Specification (Phase 0.75)

Details deterministic configuration schema, load order, and hashing for Echo.

---

## Principles
- Config files produce identical bytes across platforms after canonicalization.
- Configuration changes recorded and hashable for provenance.
- No environment-specific defaults; explicit overrides only.

---

## Schema

```ts
interface EchoConfig {
  version: string;
  mathMode: "float32" | "fixed32";
  chunkSize: number;
  backpressureMode: "throw" | "dropOldest" | "dropNewest";
  traceLevel: "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR";
  entropyWeights: Record<string, number>;
  inspector: {
    enabled: boolean;
    port: number;
  };
  plugins: string[];
}
```

Canonical ordering: keys sorted lexicographically, numeric fields clamped to valid ranges.

---

## Load Pipeline
1. Load `echo.config.json` from project root.
2. Apply optional overlay `echo.config.local.json` (must be deterministic in CI).
3. Validate against JSON Schema.
4. Canonicalize: sort maps, clamp numeric precision, convert floats via `Math.fround`.
5. Compute `configHash = BLAKE3(canonicalBytes)`; store in block manifest.

---

## Overrides & Diff
- Configuration cannot be mutated at runtime except via explicit `config/update` events (requires capability `world:config`).
- Each update produces `ConfigDiffRecord` with old/new values; replay reproduces sequence.

---

## CLI Commands
- `echo config --dump` – prints canonical config JSON + hash.
- `echo config --verify` – recomputes hash to detect tampering.
- `echo config --schema` – outputs JSON Schema.

---

## Determinism
- Hash recorded in determinism log; mismatches trigger `ERR_CONFIG_HASH_MISMATCH`.
- Config load order (base, overlay) must be identical for all deployments.

---

This spec ensures configuration is deterministic, auditable, and reproducible across environments.
