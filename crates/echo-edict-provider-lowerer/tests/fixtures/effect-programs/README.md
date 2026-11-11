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
| `read-allocation-short`                                       | an allocation budget one byte under a read's runtime cost on a 1,048,000-byte atom                 | `read-and-return-control`                                      |
| `read-captured-budget`                                        | the captured read's own 1 MiB step budget                                                          | `read-control`                                                 |

The runtime prepares eight of these and refuses them only when it runs. `the_runtime_refuses_each_executed_entry_when_it_runs` executes each one beside its accepted control and asserts the exact refusal; `ensure_node_needs_exactly_its_worst_case_allocation` does the same for `mutation-allocation-short`.

`reads_cost_a_fixed_linear_law` pins what a read costs the runtime on a root-warp node. The captured read, whose `require` fails, costs 67 + 2n steps and 3,976 + 3n bytes for an n-byte atom. `read_and_return` costs 55 + n steps and 3,142 + 3n bytes. A run is capped at 1,048,576 steps, so the captured read exceeds its declared 1 MiB step budget once the atom is larger than 524,254 bytes.

Every package names its Core as its canonical meaning, as the lowerer publishes it.

These are synthetic, internally repinned packages built from the retained public node-atom-read capture. They are not evidence of a successful public compiler run.
