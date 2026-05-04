<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley-Compiled Optic Bindings For Echo

This companion spec defines the Echo-owned Wesley compiler extension required
by [Echo Optics API Design](./design.md).

The implementation home is `crates/echo-wesley-gen`. Wesley owns authored
contract semantics and IR production. Echo owns the generated Echo-facing
runtime bindings that turn those contracts into typed optic descriptors,
observation requests, and intent-dispatch proposals.

## Decision

Wesley output should compile application contracts into typed Echo Optic
bindings.

It should not compile them into Echo-core subclasses, global graph adapters, or
mutable handles.

The correct shape is:

```text
Wesley contract
  -> Echo-owned generator extension
  -> typed optic family
  -> typed optic binding
  -> ObserveOpticRequest for reads
  -> DispatchOpticIntentRequest for proposals
```

Byte-level EINT packing may be hidden inside the generated binding. Intent
dispatch itself must remain explicit at the Echo API boundary.

The rule:

```text
EINT bytes are a binding implementation detail.
Intent dispatch is not an optic implementation detail.
```

## Why Not Subclasses

Avoid "Optics subclasses" as the mental model.

Subclass language implies that Echo core owns application-specific runtime
types or inheritance slots. Echo should not grow specialized text, debugger,
Graft, editor, or consumer subclasses.

Use these names instead:

- Wesley-generated optic bindings;
- Wesley-generated optic families;
- generated contract optic descriptors;
- typed optic adapters.

The generated Rust type may be a struct with methods, but those methods are
typed request builders or dispatch convenience wrappers over generic Echo
requests.

## Generated Layers

`echo-wesley-gen` should emit four layers.

1. Contract metadata:
    - schema hash;
    - codec id;
    - registry version;
    - contract family id or derivation inputs;
    - operation catalog;
    - projection and reducer versions.
2. DTO and codec layer:
    - generated input/result structs;
    - canonical vars encoders;
    - result decoders where available.
3. Optic descriptor layer:
    - typed focus builders;
    - typed `OpenOpticRequest` builders;
    - typed query/read aperture helpers.
4. Intent proposal layer:
    - typed mutation vars encoders;
    - EINT v1 packing where applicable;
    - typed `DispatchOpticIntentRequest` builders.

The generated module may still expose low-level helpers for tests and advanced
hosts, but the primary app-facing surface should be optic request builders.

## Generated Type Shape

Candidate generated output:

```rust
pub struct GeneratedContractOpticFamily {
    pub family: ContractFamilyRef,
    pub registry: &'static dyn RegistryProvider,
    pub projection_version: ProjectionVersion,
    pub reducer_version: Option<ReducerVersion>,
}

pub struct GeneratedCounterValueOptic {
    pub optic: EchoOptic,
}
```

The family object names static generated metadata. The opened optic binding
names one validated `EchoOptic` descriptor. Neither is a mutable handle.

## Opening A Typed Optic

Generated read/query helpers should create `OpenOpticRequest` values rather
than directly calling `observe`.

Example:

```rust
impl GeneratedContractOpticFamily {
    pub fn counter_value_optic(
        &self,
        worldline_id: WorldlineId,
        coordinate: EchoCoordinate,
        capability: OpticCapability,
        cause: OpticCause,
    ) -> OpenOpticRequest {
        OpenOpticRequest {
            focus: OpticFocus::Worldline { worldline_id },
            coordinate,
            projection_law: ProjectionLawRef::ContractQuery {
                family: self.family,
                op_id: OP_COUNTER_VALUE,
                version: self.projection_version,
            },
            reducer_law: self.reducer_version.map(ReducerLawRef::Contract),
            intent_family: IntentFamilyRef::Contract {
                family: self.family,
                allowed_ops: &'static [OP_INCREMENT],
            },
            capability,
            cause,
        }
    }
}
```

The exact syntax will change with real DTO names. The semantic requirements are
stable:

- the generated request names focus;
- it names coordinate;
- it names projection law and version;
- it names intent family;
- it carries capability and cause.

## Observing Through A Typed Optic

Generated query helpers should build `ObserveOpticRequest`.

Example:

```rust
impl GeneratedCounterValueOptic {
    pub fn observe_counter_value(
        &self,
        vars: &CounterValueVars,
        aperture: OpticAperture,
    ) -> Result<ObserveOpticRequest, CanonError> {
        let vars_bytes = encode_counter_value_vars(vars)?;
        let vars_digest = hash_vars(&vars_bytes);

        Ok(ObserveOpticRequest {
            optic_id: self.optic.optic_id,
            focus: self.optic.focus.clone(),
            coordinate: self.optic.coordinate.clone(),
            aperture: aperture.with_query(OP_COUNTER_VALUE, vars_digest),
            projection_version: self.optic.projection_law.version(),
            reducer_version: self.optic.reducer_law.as_ref().map(ReducerLawRef::version),
            capability: self.optic.capability.clone(),
        })
    }
}
```

The helper may also expose a lower-level raw-vars variant:

```rust
pub fn observe_counter_value_raw_vars(
    &self,
    vars_bytes: &[u8],
    aperture: OpticAperture,
) -> ObserveOpticRequest;
```

This mirrors the existing `*_observation_request_raw_vars` pattern while moving
the output from `ObservationRequest` to `ObserveOpticRequest`.

## Dispatching Through A Typed Optic

Generated mutation helpers should build `DispatchOpticIntentRequest`.

They may provide convenience methods that call an injected `EchoOptics` port,
but the request object must remain visible and testable.

Request-builder form:

```rust
impl GeneratedCounterValueOptic {
    pub fn increment_intent(
        &self,
        base_coordinate: EchoCoordinate,
        vars: &IncrementVars,
        actor: OpticActor,
        cause: OpticCause,
    ) -> Result<DispatchOpticIntentRequest, GeneratedIntentError> {
        let vars_bytes = encode_increment_vars(vars)
            .map_err(GeneratedIntentError::EncodeVars)?;
        let vars_digest = hash_vars(&vars_bytes);
        let eint = pack_intent_v1(OP_INCREMENT, &vars_bytes)
            .map_err(GeneratedIntentError::PackEnvelope)?;

        Ok(DispatchOpticIntentRequest {
            optic_id: self.optic.optic_id,
            base_coordinate,
            intent_family: IntentFamilyRef::Contract {
                family: self.optic.contract_family(),
                op_id: OP_INCREMENT,
            },
            focus: self.optic.focus.clone(),
            actor,
            cause,
            capability: self.optic.capability.clone(),
            admission_law: AdmissionLawRef::ContractDefault {
                family: self.optic.contract_family(),
            },
            intent: OpticIntentPayload::EintV1 {
                bytes: eint,
                op_id: OP_INCREMENT,
                vars_digest,
            },
        })
    }
}
```

Convenience dispatch form:

```rust
impl GeneratedCounterValueOptic {
    pub fn dispatch_increment(
        &self,
        port: &mut dyn EchoOptics,
        base_coordinate: EchoCoordinate,
        vars: &IncrementVars,
        actor: OpticActor,
        cause: OpticCause,
    ) -> Result<IntentDispatchResult, GeneratedIntentError> {
        let request = self.increment_intent(base_coordinate, vars, actor, cause)?;
        Ok(port.dispatch_optic_intent(request))
    }
}
```

This is allowed because the generated method still requires an explicit base
coordinate and still crosses Echo through `dispatch_optic_intent`.

Forbidden generated forms:

```rust
optic.set_counter_value(...)
optic.replace_range(...)
optic.update(...)
```

unless the method name and signature clearly express intent proposal and require
an explicit causal basis. Prefer:

- `build_*_intent`;
- `*_intent`;
- `dispatch_*`;
- `submit_*_intent`;
- `propose_*`.

## Causal Basis Rule

Generated mutation helpers must not default to "current frontier".

They must require one of:

- explicit `base_coordinate`;
- explicit `BasePolicy::UseOpenedCoordinate`;
- explicit `BasePolicy::ResolveFrontierAtDispatch` with a named admission law
  that can obstruct, stage, or preserve plurality.

The default generated API should require `base_coordinate`.

If a convenience method resolves frontier at dispatch time, the method name must
make that visible:

```rust
dispatch_increment_at_resolved_frontier(...)
```

and the result must still be typed as `IntentDispatchResult`.

## Query Helper Migration

Current `echo-wesley-gen` emits:

```text
*_observation_request(...)
*_observation_request_raw_vars(...)
```

The Optics extension should add, not immediately replace:

```text
*_observe_optic_request(...)
*_observe_optic_request_raw_vars(...)
```

The old helpers can remain during migration. They build the lower-level
`ObservationRequest` used by the current ABI. The new helpers build the
first-class optic request.

Migration rule:

```text
ObservationRequest helpers are compatibility helpers.
ObserveOpticRequest helpers are the preferred generated read surface.
```

## Mutation Helper Migration

Current `echo-wesley-gen` emits:

```text
pack_*_intent(...)
pack_*_intent_raw_vars(...)
```

The Optics extension should add:

```text
*_dispatch_optic_intent_request(...)
*_dispatch_optic_intent_request_raw_vars(...)
```

The old helpers remain useful because EINT v1 is still the inner canonical
payload. The new helpers wrap those bytes into an optic dispatch request with
explicit base coordinate, focus, actor/cause, capability, and admission law.

Migration rule:

```text
EINT pack helpers are low-level payload helpers.
DispatchOpticIntentRequest helpers are the preferred generated proposal surface.
```

## Echo-Owned IR Requirements

The current Echo IR has enough for basic op ids and vars:

- op kind;
- op name;
- op id;
- args;
- result type;
- schema hash;
- codec id;
- registry version.

Optic bindings need additional optional metadata. Add only what RED tests prove
missing, but expect these fields:

```json
{
    "contract_family": "toy-counter",
    "projection_version": 1,
    "reducer_version": null,
    "ops": [
        {
            "kind": "QUERY",
            "name": "counterValue",
            "op_id": 1002,
            "optic": {
                "focus": "worldline",
                "aperture": "query_bytes",
                "projection_law": "contract_query"
            }
        },
        {
            "kind": "MUTATION",
            "name": "increment",
            "op_id": 1001,
            "optic": {
                "intent_family": "contract_mutation",
                "admission_law": "contract_default"
            }
        }
    ]
}
```

Do not make GraphQL directives the Echo runtime API. Directives may feed Wesley
IR. The Echo-owned generator consumes IR and emits Rust DTOs.

## Registry And Artifact Identity

Generated optic bindings must include registry/artifact identity in every
request they build.

At minimum:

- schema hash;
- codec id;
- registry version;
- contract family id;
- op id;
- projection version;
- reducer version when present.

Future authenticated admission may add artifact attestation and capability
certificates. The generated binding must leave a slot for those identities
without requiring production crypto in the first slice.

## no_std Requirements

The generated optic bindings must preserve the existing `--no-std` path.

Requirements:

- use `alloc::vec::Vec` and `alloc::string::String` when needed;
- no ambient filesystem, time, randomness, or host IO;
- compile in a no-std consumer smoke crate;
- expose request-builder helpers even when convenience dispatch helpers require
  a std-hosted trait object.

If convenience dispatch helpers cannot be no-std, gate them. Request builders
should remain no-std-capable.

## Tests

Add generator tests before implementation:

1. Generated query op emits `*_observe_optic_request`.
2. Generated mutation op emits `*_dispatch_optic_intent_request`.
3. Mutation helper requires explicit base coordinate.
4. Generated request includes optic id, focus, intent family, capability,
   actor/cause, and admission law.
5. EINT bytes remain inner payload and decode to the original op id/vars.
6. No generated helper is named `set_*`.
7. Generated output compiles under std and no-std consumer crates.
8. Existing `ObservationRequest` and `pack_*_intent` helpers remain available
   during migration.

## Acceptance Criteria

- Application code can interact with typed generated optic bindings.
- Application code does not need to manually pack EINT for the happy path.
- Echo still receives explicit `ObserveOpticRequest` or
  `DispatchOpticIntentRequest`.
- Dispatch requests always name causal basis unless an explicit named base
  policy says otherwise.
- Generated read helpers remain bounded and aperture-aware.
- Echo core remains generic and imports no application nouns.
- Intent admission remains witnessable through Echo receipts.

## Non-Goals

- Do not make generated bindings mutable handles.
- Do not add inheritance/subclass machinery to Echo core.
- Do not make Intent dispatch disappear from Echo's public boundary.
- Do not replace EINT v1 until a RED proves it insufficient.
- Do not make GraphQL the runtime API.
- Do not add jedit-specific generated behavior.
