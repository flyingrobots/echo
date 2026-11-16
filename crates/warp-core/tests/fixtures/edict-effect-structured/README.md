# Structured effect publication fixture

These authored inputs select the current `echo.dpo@2` profile and its exact
`echo.span-ir/v3` reference. The lawpack exports only `ensureNode`, `createNode`,
and their input/result types. The application exercises an initial effect,
a bounded loop, and both statement-conditional arms through the public compiler.

The `effect_step_publication` example accepts this directory with the output name
`structured`. It builds the lawpack, replaces the source import's zero digest
with the actual manifest digest, and builds the application twice. Its retained
outputs must come from that producer, including
`structured-executable-operation-package.hex`; they are not authored Target IR.
The zero digest is an unbuilt input placeholder, not a lawpack identity.

The intended runtime input sets `initial` to node A and supplies two items:
`{create: true, node: B}`, then `{create: false, node: A}`. A successful private
evaluation must buffer and complete exactly `EnsureNode(A)`, `CreateNode(B)`,
`EnsureNode(A)` in that order. Empty and one-item lists retain the corresponding
prefix. The return value is the initial ensure result, an empty record. No
graph commit, causal receipt, or WAL publication is claimed by this fixture.

`items` has maximum length two and each node payload has at most 64 atom bytes.
The configuration retains a 1,024-byte input ceiling, 64-byte result ceiling,
65,536-byte allocation ceiling, and 4,096 each for steps, read bytes, and write
bytes. The shared node shape keeps the two-item input within the input ceiling;
there is no extra unused atom-replacement payload in either branch.

Draft evidence boundary: only the authored inputs exist at preparation time.
Compiler publication, repeat-byte equality, profile substitution refusals, and
native execution remain to be run. The runtime witness owns exact package
policy, list-bound refusal, and effect-order assertions. Frozen `echo.dpo@1`
fixtures are unchanged.
