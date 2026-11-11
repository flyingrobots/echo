<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Authored effect-program verdict corpus

Each `.hex` file holds the canonical bytes of one whole operation package carrying an authored effect program. `verdicts.txt` lists every entry, in order, with the verdict Echo's provider lowerer and verifier must each reach: `accept` or `refuse`.

warp-core generates the corpus from its own effect-package fixture builders and asserts the runtime's verdict on every entry (`crates/warp-core/tests/edict_effect_package_tests/provider_corpus.rs`). The lowerer (`tests/effect_program_judgment.rs`) and the verifier (`crates/echo-edict-provider-verifier/tests/effect_program_judgment.rs`) judge the same bytes independently. Rebuild the files with that module's ignored `regenerate_the_retained_provider_corpus` test after changing an entry.

The accepted entries are a node-atom read with its four failure arms, an `ensure-node` mutation, a runtime prefix slice proven by an input constraint, the same slice proven by an earlier conjunct of one `all` input constraint, and an `ensure-node` mutation whose allocation budget is exactly the runtime's worst case on a 1,024-byte atom. Each refused entry is a well-formed package that reaches one specific check: an unproven slice, a slice guarded on another operand, a slice guarded only by a `require`, an input constraint whose slice only an earlier input constraint would prove, swapped execution order, a mutation input whose node id is not exactly 32 bytes, an obstruction missing from the exports, tag-spelled, unknown and duplicate failure mappings, two step budgets too small for the program's worst case, and an allocation budget one byte short of the mutation's worst case.

The three budget entries are refused by the providers' static cost models. The runtime prepares them and meters the budget only when it executes, so the runtime test checks preparation for those entries rather than refusal. `ensure_node_needs_exactly_its_worst_case_allocation` runs the mutation on its largest input to pin the allocation boundary both mutation entries use.

Every package names its Core as its canonical meaning, as the lowerer publishes it.

These are synthetic, internally repinned packages built from the retained public node-atom-read capture. They are not evidence of a successful public compiler run.
