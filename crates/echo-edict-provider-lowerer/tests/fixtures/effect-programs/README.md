<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Authored effect-program verdict corpus

Each `.hex` file holds the canonical bytes of one whole operation package carrying an authored effect program. `verdicts.txt` lists every entry, in order, with the verdict Echo's provider lowerer and verifier must each reach: `accept` or `refuse`.

warp-core generates the corpus from its own effect-package fixture builders and asserts the runtime's verdict on every entry (`crates/warp-core/tests/edict_effect_package_tests/provider_corpus.rs`). The lowerer (`tests/effect_program_judgment.rs`) and the verifier (`crates/echo-edict-provider-verifier/tests/effect_program_judgment.rs`) judge the same bytes independently. Rebuild the files with that module's ignored `regenerate_the_retained_provider_corpus` test after changing an entry.

Each refused entry is a well-formed package that reaches one specific check. Each has an accepted sibling that differs from it only by the edit it targets, so that edit is what refuses it:

| Refused entry                                                 | What it edits                                                                                      | Accepted sibling                                               |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `slice-unproven`, `slice-other-operand`, `slice-require-only` | the slice's proof: none, another operand's, or only a `require`                                    | `slice-proven`                                                 |
| `constraint-proven-by-constraint`                             | a second constraint that only the first would prove                                                | `constraint-conjunct-proven` (the same proof inside one `all`) |
| `forged-node-order`                                           | the lowered execution order, swapped                                                               | `two-let-control`                                              |
| `wrong-intrinsic-input-type`                                  | a mutation node id that is not exactly 32 bytes                                                    | `ensure-node-control`                                          |
| `undeclared-obstruction`                                      | the exports, without the obstruction the `missing` arm maps to, with every lowering pin re-derived | `read-exports-control`                                         |
| `failure-mapping-duplicate`                                   | the adapter's failure table                                                                        | `read-adapter-control`                                         |
| `failure-mapping-unknown`                                     | one failure renamed in both the adapter's table and the catalog                                    | `read-adapter-control`                                         |
| `failure-mapping-tag-spelled`                                 | the same failures spelled by runtime tag instead of by name, in both the adapter and the catalog   | `read-control`                                                 |
| `over-budget`                                                 | a one-step budget                                                                                  | `slice-proven`                                                 |
| `mutation-allocation-short`                                   | an allocation budget one byte under the runtime's worst case                                       | `mutation-allocation-exact`                                    |
| `require-payload-over-output`                                 | an output ceiling with room for the result but not for the failing `require`'s payload             | `require-payload-fits`                                         |
| `mutation-count-over`                                         | 1,025 mutations, one over the runtime's ceiling                                                    | `mutation-count-control` (two mutations)                       |
| `local-name-empty`                                            | an empty local name                                                                                | `ensure-node-control`                                          |
| `constraint-without-coordinate`                               | an input constraint with no coordinate                                                             | `slice-proven`                                                 |
| `mutation-result-nominal`                                     | the mutation's result type, made a nominal wrapper                                                 | `mutation-input-nominal` (the input made nominal)              |
| `nominal-extra-field`                                         | a nominal type entry with a field beyond kind, contract and representation                         | `mutation-input-nominal`                                       |
| `failure-declared-twice`                                      | a read step that declares one failure twice                                                        | `read-control`                                                 |
| `basis-not-bytes`                                             | an evaluation basis that is a `Bool`                                                               | `basis-control` (a 32-byte basis)                              |
| `type-table-int`                                              | a type-table entry `{ kind: "Int", width: "U64" }`                                                 | `type-table-width` (`{ kind: "U64" }`)                         |
| `read-allocation-short`                                       | an allocation budget one byte under a read's runtime cost on a 1,048,521-byte atom                 | `read-and-return-control`                                      |
| `read-captured-budget`                                        | the captured read's own 1 MiB step budget                                                          | `read-control`                                                 |


## Structured entries (`echo.span-ir/v3` under `echo.dpo@2`, #947)

`structured-loop-control` runs one `ensure-node` site inside `for item in arg.0 bounded 3` over `List<MutationInput,max=3>`; `structured-branch-control` wraps that loop in a statement branch and `structured-nested-control` in an outer loop over `List<List<MutationInput,max=3>,max=2>`. Every refused entry below is that loop, fully repinned after the one edit named:

| Refused entry | What it edits | Accepted sibling |
| --- | --- | --- |
| `structured-echo-dpo-1` | the profile, the frozen `echo.dpo@1` | `structured-loop-control` |
| `structured-bound-below-list` | a literal bound of 2 under a list of at most 3 | `structured-loop-control` |
| `structured-coordinate-bound` | a coordinate bound, which Target IR cannot authenticate | `structured-loop-control` |
| `structured-leaked-local` | a result that reads the loop body's local after the loop | `structured-loop-control` |
| `structured-control-id-reused` | the loop's control id, set to its body's instruction id | `structured-loop-control` |
| `structured-dropped-body` | the loop entry's nested order, emptied | `structured-loop-control` |
| `structured-ordered-with-control` | the declaration, `echo.span-ir/v2` with the same control entry | `structured-loop-control` |
| `structured-substituted-binder` | the Target loop's binder, unlike its Core loop's | `structured-loop-control` |
| `structured-body-with-result` | the loop body's result, a record instead of `null` | `structured-loop-control` |
| `structured-loop-allocation-short` | an allocation one byte under the loop's runtime worst case (three 1,024-byte atoms) | `structured-loop-allocation-exact` |
| `structured-loop-under-budget` | a step budget one under what the runtime spends on that input | `structured-loop-allocation-exact` |
| `structured-mutation-count-over` | a bound of 1,025 on one effect site | `structured-mutation-count-control` (bound 1,024) |

`structured_loop_needs_exactly_its_worst_case_allocation` runs the worst case: both providers charge exactly the runtime's 18,325 bytes, and their step ceiling (173) covers the runtime's 158 steps.

The runtime prepares eight of the flat entries and three of the structured ones and refuses them only when it runs. `the_runtime_refuses_each_executed_entry_when_it_runs` executes each one and asserts the exact refusal, beside an accepted control for every entry but `read-captured-budget`; `ensure_node_needs_exactly_its_worst_case_allocation` does the same for `mutation-allocation-short`. `read-captured-budget` declares 1,048,576 steps, which equals the frame's fixed step ceiling, so on a large enough atom the declared budget and the ceiling refuse together.

`reads_cost_a_fixed_linear_law` pins what a read costs the runtime on a root-warp node. The captured read, whose `require` fails, costs 67 + 2n steps and 3,976 + 3n bytes for an n-byte atom. `read_and_return` costs 55 + n steps and 3,142 + 3n bytes. A run is capped at 1,048,576 steps, so the captured read exceeds its declared 1 MiB step budget once the atom is larger than 524,254 bytes.

Every package names its Core as its canonical meaning, as the lowerer publishes it.

These are synthetic, internally repinned packages built from the retained public node-atom-read capture. They are not evidence of a successful public compiler run.
