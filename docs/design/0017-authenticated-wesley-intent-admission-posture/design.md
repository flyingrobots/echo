<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0017 - Authenticated Wesley Intent Admission Posture

_Name the missing security and artifact-trust boundary between
Wesley-generated contract helpers and Echo tick admission._

Legend: [PLATFORM](../../method/legends/PLATFORM.md)

Depends on:

- [0013 - Wesley Compiled Contract Hosting Doctrine](../0013-wesley-compiled-contract-hosting-doctrine/design.md)
- [0014 - EINT, Registry, And Observation Boundary Inventory](../0014-eint-registry-observation-boundary-inventory/design.md)
- [0015 - Registry Provider Host Boundary Decision](../0015-registry-provider-host-boundary-decision/design.md)
- [0016 - Wesley To Echo Toy Contract Proof](../0016-wesley-to-echo-toy-contract-proof/design.md)
- [Authenticated Wesley Intent Admission Posture](../../method/backlog/up-next/PLATFORM_authenticated-wesley-intent-admission-posture.md)

## Status

Proposed.

## Hill

A Wesley-compiled intent must not become tick-admissible merely because it is
well-formed EINT bytes.

Before a submitted intent can enter a tick candidate set, Echo must be able to
prove three things:

1. The submitted canonical intent bytes have the claimed cryptographic
   identity.
2. The intent targets an operation from a registered Wesley contract artifact
   whose trust posture satisfies local policy.
3. The authenticated session or capability is authorized to submit that
   operation at the target coordinate under the active observer or ingress
   policy.

## Current Repo Truth

Current EINT v1 is intentionally app-blind:

```text
"EINT" || op_id:u32le || vars_len:u32le || vars
```

Current dispatch takes canonical bytes:

```text
dispatch_intent(intent_bytes)
```

The installed `WarpKernel` parses EINT, computes a content-addressed
`intent_id`, wraps the bytes as `IngressEnvelope::local_intent`, and ingests
the envelope into the default writer inbox.

That path is correct for the toy bridge but incomplete for production contract
hosting. It does not currently bind the submitted bytes to:

- a certified Wesley artifact;
- a generated footprint authority;
- a session or subject;
- a capability or observer policy;
- a target-coordinate authorization claim;
- a replay window;
- a signed or MACed admission transcript;
- an encrypted request or response channel.

The current default writer uses `InboxPolicy::AcceptAll`. Existing
`InboxPolicy` kinds are deterministic ingress filters and budgets. They are not
authentication, artifact certification, or transport security.

## Doctrine

Wesley exists to move footprint honesty out of runtime trust.

Echo must not accept caller-supplied footprint claims for tick scheduling or
independence decisions. A client may submit canonical operation variables, but
the footprint authority must come from a registered, verified contract artifact
or another explicitly trusted footprint authority.

The hard rule:

```text
No verified artifact authority, no tick-admissible Wesley intent.
No authenticated session or capability, no tick-admissible Wesley intent.
No caller-supplied footprint trust.
```

## Artifact Versus Intent

An intent instance should not register itself.

A Wesley-compiled contract artifact registers operation identity, codec
identity, registry identity, and footprint authority. A submitted intent points
at that registered artifact and supplies canonical operation variables.

The trust split is:

```text
artifact trust:
  Is this contract artifact the accepted output of the Wesley certification
  pipeline?

intent trust:
  Did an authorized subject or session submit exactly these canonical bytes for
  exactly this target under exactly this policy?
```

Both are required before tick admission.

## Trust Ramp

Echo should model artifact trust posture without pretending that every
production certification provider is implemented in this slice.

Initial posture vocabulary should leave room for this ramp:

```text
local_dev
local_digest_verified
generated_tests_verified
ci_attested
blade_certified
```

Exact names may change. The important rule is that each posture must be
explicit, and policy must be able to reject artifacts whose posture is too weak
for the requested observer or intent.

Holmes, WATSON, Moriarty, generated tests, and BLADE are production-ramp
certification providers. They belong in the certification chain, not in the
normal deterministic tick execution path.

For this slice, Echo should define the slots and policy vocabulary. It should
not implement those external certification systems.

## Candidate Identity Chain

The model needs separate identities for contract artifacts, intent bytes, and
admission.

Candidate identities:

```text
contract_artifact_id =
  H("echo.contract-artifact.v1" ||
    schema_digest ||
    ir_digest ||
    registry_digest ||
    codec_id ||
    compiler_profile_digest ||
    artifact_trust_posture)

intent_digest =
  H("echo.intent.v2" ||
    contract_artifact_id ||
    op_id ||
    canonical_vars_digest)

admission_id =
  H("echo.intent-admission.v1" ||
    intent_digest ||
    target_coordinate ||
    session_id ||
    policy_id ||
    replay_counter_or_challenge_digest)
```

Those formulas are design sketches, not committed wire formats. The committed
requirement is domain separation and explicit scope: a digest must say what it
commits to.

## Pre-Tick Admission Pipeline

The secure production path should be:

```text
Wesley-generated client
  -> canonicalize operation variables
  -> build inner EINT payload
  -> compute intent digest
  -> bind intent digest to contract artifact id
  -> bind target coordinate and policy/session transcript
  -> sign, MAC, or seal the submission
  -> send authenticated intent submission

Echo host boundary
  -> verify artifact registration and trust posture
  -> verify op id exists in the registered artifact
  -> verify vars decode canonically under the registered codec
  -> verify intent digest
  -> verify session, capability, auth method, and policy
  -> verify replay protection
  -> resolve footprint through the trusted artifact authority
  -> close footprint under policy
  -> emit admission certificate or obstruction

Echo runtime
  -> admit only certified ingress into tick candidate selection
  -> retain admission witness reference in tick receipt
```

Authentication, randomness, nonces, challenge material, and transport keys live
outside deterministic execution. If they affect admission, their digests or
verification results become explicit admission evidence.

## Observer Policy Relationship

Observer policies should eventually govern both reads and writes.

An observer or ingress policy may require:

- a minimum artifact trust posture;
- a specific certification profile;
- a session authentication method;
- step-up authentication for privileged operations;
- allowed operation ids;
- allowed target worldlines, strands, inboxes, or coordinates;
- allowed observation frames and projections;
- request and response protection;
- replay bounds;
- retention and audit behavior.

This design does not require every policy field now. It names the direction so
future API choices do not bake in an unauthenticated toy boundary.

## Request And Response Protection

Encryption should protect the host/client request-response channel, not become
hidden runtime state.

The future protected submission shape should wrap canonical payloads:

```text
protected request:
  authenticated/session-bound envelope over EINT or ObservationRequest bytes

protected response:
  authenticated/session-bound envelope over DispatchResponse or
  ObservationArtifact bytes
```

The deterministic kernel should still receive canonical plaintext DTOs after
host-boundary verification. The host-boundary transcript, not hidden ambient
state, records the security facts that justified admission.

## Failure Outcomes

Security and artifact failures should lower to typed admission outcomes before
tick candidate selection. Candidate categories:

- malformed payload;
- unknown artifact;
- artifact trust posture too weak;
- artifact digest mismatch;
- unknown operation id;
- canonical vars decode failure;
- policy denied;
- authentication required;
- authentication failed;
- replay detected;
- expired session;
- bad signature or MAC;
- footprint authority unavailable;
- footprint binding obstruction;
- policy closure over budget.

Which of these become public error codes, retained obstruction artifacts, or
private audit records is a later API decision. The design rule is that failed
security checks must not silently become ordinary tick candidates.

## RED Boundary

Current Echo cannot yet prove this requirement.

Focused REDs for the implementation slice should show:

- a well-formed EINT can currently be dispatched without a contract artifact
  id;
- a well-formed EINT can currently be dispatched without session or capability
  proof;
- a well-formed EINT can currently be dispatched without artifact trust
  posture;
- current dispatch has no replay counter, challenge, or signed transcript;
- current tick or ingress evidence cannot distinguish "well-formed bytes" from
  "authenticated, artifact-verified, policy-admitted intent";
- current `ReadingRightsPosture` cannot express policy-authorized observer
  session posture beyond `KernelPublic`.

## Non-Goals

- Do not implement WebAuthn, passkeys, TOTP, or transport encryption in this
  slice.
- Do not implement Holmes, WATSON, Moriarty, or BLADE.
- Do not require production certification for local development.
- Do not replace EINT v1 before a RED proves the exact missing wire field.
- Do not add application-specific nouns to Echo core.
- Do not import app domain Rust types into Echo core.
- Do not trust caller-supplied runtime footprint claims.
- Do not make deterministic runtime execution read wall clock, randomness,
  environment, filesystem, or host session maps.

## Acceptance Impact

The next implementation cycle should start RED-only.

It should add focused tests or design-level fixtures proving the absent
admission posture before changing production dispatch. The first GREEN should
be the smallest explicit posture model that can represent:

- registered artifact identity;
- artifact trust posture;
- authenticated intent submission identity;
- policy/session admission result;
- no caller-supplied footprint trust.

Full production cryptography remains a later ramp. The current slice is about
preventing the API from confusing "well-formed EINT" with "tick-admissible
certified intent."
