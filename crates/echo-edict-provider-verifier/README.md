<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Edict Provider Verifier

This crate owns Echo's pure semantic decision for Edict's frozen verifier
boundary. It compares explicit, digest-bound Edict Core and Target IR artifacts
under the checked Echo provider closure. It performs no discovery or I/O and
grants no Echo runtime authority.

The first native slice is intentionally independent from the provider lowerer.
A supported but semantically false Target IR produces a rejected verifier
report; malformed input in the selected native closure and unsupported source
semantics produce typed provider refusals. Complete structural admission for
every CDDL alternative remains the Edict host's owning-schema check before the
component runs; decoding this native model's result is never admission. Before
classifying a known but unsupported expression as semantic disagreement, the
native verifier checks its complete expression, predicate, input-constraint,
require-failure, and Core-value shape under the fixed recursion bound. The
target profile's diagnostic ABI and every emitted report consume one shared
admitted identity.

The verifier also owns the structurally separate executable-operation route. It
accepts the exact source, Core, lawpack, exports, adapter, target
configuration, Target IR, and compiler-emitted package as digest-bound inputs,
then reconstructs the expected generic package without importing or calling
the lowerer. Exact equality yields an accepted
`echo.operation-package-verifier-report/v1`; a rebound or otherwise
self-consistent package mutation yields a typed rejected report. The verifier
independently resolves source-local effect and obstruction aliases through the
exact lawpack import before comparing the canonical package. It independently
validates the compiler-owned result-projection artifact, reconstructs the
runtime expression and application-input paths, binds their identity into the
expected package, and names that exact projection in the accepted report.
Application vocabulary is treated only as opaque authored data.

**What this independence covers.** The mutation, compare-and-set and bounded
pure packages are rebuilt in `src/executable_operation.rs`. That file does not
import the lowerer, but most of it is a copy of the lowerer's file of the same
name: on 2026-10-14, 91% of the lowerer's distinct code lines appeared
verbatim. The byte comparison therefore rejects any package that differs from
what this copy produces. That covers tampering after lowering and a defect
introduced in the lowerer alone, as the rebound-coordinate and tamper tests in
`tests/executable_operation_package.rs` show. It does not catch a mistake
present in both copies: such a package lowers and then verifies. Commit
`d121758b` is an example, where one missing refusal had to be added to both
files. For these packages, acceptance shows that two copies agree; it is not
an independent semantic check (#809). The bounded-read route below is
different: `src/executable_operation/bounded_read.rs` rebuilds Core from the
Target IR instead of repeating the lowering.

For `compiler-produced-bounded-pure/v1`, the verifier separately enforces
the zero-choice configuration, effect-free Core and Target IR shapes, exact
budget and closure bindings, pure-binding result projection, and embedded
compiler artifact bytes. It reconstructs the distinct generic pure package
without calling the lowerer and rejects canonical package substitution. An
accepted report retains a canonical `echo.executable-subject/v1` manifest and
names its domain-framed identity. That subject binds the exact package, Target
IR, and application result projection; it does not prove runtime evaluation,
admission, or an Echo consequence.

The native `compiler-produced-bounded-read/v1` route reconstructs an ordered
Core body from the Target IR instruction list, reversing failure mappings
through the declared adapter. It independently checks read signatures, scoped
local identities, type bounds, guards, obstruction constructors, projection,
budget agreement, profile identities, and every embedded semantic artifact.
It does not import or call the lowerer in production. Coherently rehashed Core,
Target IR, and package mutations still receive rejected reports when they
violate that relation. Accepted and rejected reports retain the existing exact
executable-subject binding; the byte ceiling counts artifact bytes, role and
domain strings, and diagnostic strings.

This route is included in the checked schema/components. The explicit ordered
contract publication passes unmodified public Edict builds for single and paired
reads, including this verifier's accepted report and repeatable package/report
bytes. Native fixtures retain their diagnostic-capture provenance; the separate
public witness requires no compiler instrumentation. A separate trusted-host
private evaluator consumes pinned packages over real frontier storage; see
[application contract hosting](../../docs/architecture/application-contract-hosting.md)
for its bounded aperture and evidence limits. Compilation and accepted
verification do not authorize a state reading.

The native pure-v1 verifier additionally compares the one selected intent's
ordered let bindings (identity, local, and expression), result, input
constraints, and optional basis against Core. It refuses extra intents or
requirements. This is an identity-preserving lowering relation: independently
digest-binding two artifacts does not prove that one preserves the other.
The retained public-compiler fixture has an exact-byte positive control and
seven separately rebound semantic mutation witnesses. The checked component
below includes this relation check; host admission and runtime execution remain
separate evidence boundaries.

Before selecting either executable-package route, both native providers require
the source artifact reference to match Core's coordinate and its canonical value
to contain UTF-8 bytes. These checks do not parse or recompile the source and do
not prove source-to-Core correspondence. Both checked components include this
validation after independent reproducible promotion.

The native pure route also requires the selected adapter profile to declare an
empty `semanticEffects` array and a nonempty `budgetObligation`. A profile that
declares effects is unsupported even when the supplied program has no effect
steps. This keeps the selected profile consistent with this route's no-effects
package identity; it is not evidence of an effect-execution exploit in earlier
components. Host admission and compiler budget discharge remain separate
checks. Both checked components include this validation.

The native pure projection check independently follows each application-input
or declared-binding source path back to the Core result, including exact record
field membership, output type, and output-byte budget. A declared binding ID or
a digest-bound projection is insufficient to establish that correspondence.
Five additional retained-fixture mutations demonstrate wrong fields, wrong
bindings, missing fields, and changed output contracts or budgets. The checked
component includes the same projection check.

Both native pure routes additionally enforce the compiler-owned structural
projection contract: at most 64 KiB of canonical artifact bytes, 256 expression
nodes, 32 path segments, 1,024 UTF-8 bytes per identifier or field/path string,
and exactly the fields belonging to each expression and source variant. Both
checked components include these structural guards. The artifact byte
limit is distinct from `maxOutputBytes`: pure output remains bounded by the
matching Core budget and consumer execution limits, rather than the anchored
route's fixed 64 KiB output ceiling.

The `wasm32` guest adapter vendors Edict's exact frozen
`edict:target-provider/verifier@1.0.0` WIT world and performs only exhaustive
transport-to-model conversion. Its reproducibly built 494,526-byte checked
component has SHA-256
`2b74403a4ae4b116172401620655263944b636c19e635461815e94d01f18f766`.
Component identity and admitted host replay remain separate propositions: the
pinned Edict host preflights the request artifacts and declared output schema,
invokes the checked component, then admits and manifests each returned accepted
or rejected report. It preserves an unsupported output-role overclaim as a typed
refusal without a response or manifest and replays all three completed outcome
classes identically in independent fresh stores and separate host processes.

For this first one-operation closure, the exact checked target profile, exact
lowerability facts, and exact `echo.dpo@1.replace` intrinsic jointly bind the
`precommit-atomic` guard posture and `echo.dpo.footprint/v1` algebra identity. The
current Target IR has no independent footprint field and its requirements list
is empty, so this crate does not claim a general guard-order or footprint-
expression proof. The crate embeds the exact generated type, intrinsic,
footprint, cost, operation-profile, obstruction, lawpack-adapter, and verifier
resources. Before comparing Core with Target IR, it reproduces their
domain-framed identities, resolves the profile and lawpack references, and
checks the complete reviewed semantic crossing. Raw byte identities remain a
separate pinned proposition, and workspace validation must still prove these
package-local copies equal the CDDL-admitted checked corpus.

A verifier report's proposition is deliberately narrow: the fixed verifier
accepted or rejected the exact Target IR reference named by that report. The
report alone does not identify the Core, target profile, or semantic closure.
For each successful invocation, the pinned Edict host's output manifest binds
the exact Core, target profile, Target IR, ordered semantic inputs, requested
output, and admitted report digest. That per-invocation envelope is distinct
from the digest-locked package assembled by the next campaign goalpost, which
binds the static inputs and verifier component together. Neither proposition
has an Echo causal site or witnessed hologram, and neither grants Echo runtime
installation, execution, or consequence authority.

## Authored effect packages

The native Rust API `effect_package::verify_effect_package` judges an authored effect package before installation, without trusting the lowerer that produced it or the runtime that will execute it. It judges the compiled effect program itself: its binding to its own Core, Target IR and exports; the pinned lawpack, adapter, target profile and intrinsic catalog that authorize each effect; each effect's intrinsic ABI and version-scoped failure contract; one declared obstruction per failure arm; Core and Target IR correspondence; and the body's types, proven ranges and declared budget under the verifier's own cost model. It then requires every identity the package claims, its operation, its output type and its footprint contract to name the program it carries. It refuses with `InvalidEnvelope`, `InvalidBinding` or `UnsupportedProgram`. It does not judge graph-law semantics or grant authority, and the verifier component's interface does not yet call it.
