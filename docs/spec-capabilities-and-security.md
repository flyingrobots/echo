# Capabilities & Security Specification (Phase 0.5)

Defines capability tokens, signer policies, and deterministic security faults for Echo subsystems.

---

## Capability Tokens
Tokens grant permission to mutate specific domains.

```ts
type Capability =
  | "world:entity"     // create/destroy entities
  | "world:component"  // mutate components
  | "physics:body"     // modify physics bodies
  | "renderer:resource"
  | "timeline:branch"
  | "timeline:merge"
  | "cb:cross-branch"
  | "ai:proposal";
```

Handlers declare `requiresCaps`. Events carry `caps` (tokens the emitter holds).
- Enforcement: `requiresCaps ⊆ evt.caps` before handler invocation.
- Failure emits deterministic `ERR_CAPABILITY_DENIED`.

### Capability Issuance
- Configured at bootstrap via capability manifest (JSON): component/adapter → tokens.
- Manifest recorded in determinism log; modifications require restart to keep replay consistent.

---

## Signatures & Verification

### Security Envelope
```ts
interface SecurityEnvelope {
  readonly hash: string;       // BLAKE3(canonical bytes)
  readonly signature?: string; // Ed25519 over hash
  readonly signerId?: string;
}
```

- Snapshots, diffs, events may carry envelope.
- Signatures optional in development, enforced in secure builds.
- Verification failure emits `ERR_ENVELOPE_TAMPERED` and halts tick.

### Signer Registry
- `signerId` resolves to public key; stored in block manifest header.
- Registry modifications recorded in decision log.

---

## Capability Scopes
Scope determines default tokens per subsystem:

| Subsystem | Tokens |
| --------- | ------ |
| ECS core | `world:entity`, `world:component`
| Physics adapter | `physics:body`
| Renderer adapter | `renderer:resource`
| Codex’s Baby | `timeline:branch`, `timeline:merge`, `cb:cross-branch`
| AI copilot | `ai:proposal`, optionally `timeline:branch`

Applications may extend tokens; must keep names deterministic (lowercase, colon-separated).

---

## Fault Codes
- `ERR_CAPABILITY_DENIED` – missing required token.
- `ERR_ENVELOPE_TAMPERED` – signature/hash mismatch.
- `ERR_CAPABILITY_REVOKED` – token revoked mid-run; event quarantined.
- `ERR_CAPABILITY_UNKNOWN` – unknown token in manifest.

Faults recorded in timeline as synthetic nodes for replay.

---

## Revocation Policy
- Tokens can be revoked by emitting `security/revoke` event; requires `timeline:branch` + `timeline:merge` by trusted signer.
- Revocation triggers audit of pending events; those lacking token removed deterministically with logged drop records.

---

## Inspector View
Expose capability map for debugging:

```ts
interface CapabilityInspectorFrame {
  tick: ChronosTick;
  actors: Array<{
    id: string;
    caps: Capability[];
  }>;
  revoked: Capability[];
}
```

---

This specification ensures capability checks and signatures align with deterministic replay and security requirements.
