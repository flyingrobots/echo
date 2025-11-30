<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Plugin System Specification (Phase 0.75)

Defines how plugins extend Echo while preserving determinism and security.

---

## Goals
- Deterministic registration order and namespace isolation.
- Capability-based access to sensitive domains.
- Version negotiation to accommodate evolving engine APIs.

---

## Plugin Manifest

```ts
interface EchoPluginManifest {
  id: string;               // e.g., "echo-physics-rapier"
  version: string;          // semver
  exports: string[];        // modules or adapters exposed
  capabilities: Capability[];
  schemaVersion: string;    // API version targeted
  entry: string;            // module entry point
  signature?: string;       // optional manifest signature
}
```

Manifest JSON canonicalized (sorted keys) and hashed; signature verified if present.

---

## Load Flow
1. Engine discovers manifests (plugins/ folder or config `plugins` array).
2. Sort manifests lexicographically by `id`, then semver `version` ascending.
3. Validate capabilities; if missing required tokens, deny load deterministically.
4. Execute plugin entry `register(api: EchoWorldAPI, context: PluginContext)`.

```ts
interface PluginContext {
  readonly config: EchoConfig;
  registerComponent(descriptor: ComponentTypeDescriptor): void;
  registerSystem(system: SystemDescriptor): void;
  registerAdapter(adapter: AdapterDescriptor): void;
  registerInspectorFrame(manifest: InspectorExtensionManifest): void;
}
```

Namespaces: plugin prefixes component/system IDs with `plugin.id:` to avoid collisions.

---

## Security & Capabilities
- Plugins request capability tokens in manifest; engine verifies before registration.
- Privileged tokens: `world:component`, `timeline:branch`, `renderer:resource`, etc.
- Issuing new tokens requires entry in capability manifest (see spec-capabilities-and-security.md).

---

## Determinism
- Registration order fixed by sorted manifests.
- `pluginsManifestHash = BLAKE3(sorted(manifestHashes))` recorded in block manifest.
- Runtime logs deterministic registration events for replay.

---

## Future Extensions
- Hot reload (editor-only) with determinism fences.
- Remote plugin registry with signed catalogs.
- Sandboxed execution for untrusted plugins.

---

This spec enables safely extensible Echo deployments without compromising determinism.
