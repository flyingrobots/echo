<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
<!-- markdownlint-disable -->

# ISSUES

Generated inventory for tracked Echo work. Sources: GitHub issues via `gh issue list --state all --limit 1000`, METHOD task rows from `docs/method/task-matrix.csv`, active METHOD design cycles under `docs/design/*/design.md`, and local METHOD graveyard notes under `docs/method/graveyard/*.md`.

The `M###` identifiers are generated METHOD DAG row ids. The GitHub `GH-###` sections are separate on purpose, even when a METHOD row references the same GitHub issue, so both tracking systems can be pruned deliberately.

## Inventory Summary

| Field                  | Value |
| ---------------------- | ----- |
| GitHub issues          | 179   |
| METHOD DAG rows        | 181   |
| Active design cycles   | 16    |
| METHOD graveyard notes | 7     |

<hr />

# METHOD DAG Rows

## M001 - Man page generation and README examples

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M010 (Bench subcommand -- criterion invocation and reporting), M011 (Inspect subcommand -- metadata and graph stats), M013 (clap subcommand structure and global flags), M015 (Verify subcommand -- hash recomputation)
DAG chain depth: downstream 1; upstream 3
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                                                                                                                   |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                                                                                      |
| METHOD id                | M001                                                                                                                                                                                                                    |
| Native id                | T-6-5-1                                                                                                                                                                                                                 |
| Lane                     | asap                                                                                                                                                                                                                    |
| Status                   | done                                                                                                                                                                                                                    |
| Completed                | yes                                                                                                                                                                                                                     |
| Source path              | docs/method/backlog/asap/DOCS_cli-man-pages.md                                                                                                                                                                          |
| Anchor/link              | docs/method/backlog/asap/DOCS_cli-man-pages.md#t-6-5-1-man-page-generation-and-readme-examples                                                                                                                          |
| Direct blockers          | M010 (Bench subcommand -- criterion invocation and reporting), M011 (Inspect subcommand -- metadata and graph stats), M013 (clap subcommand structure and global flags), M015 (Verify subcommand -- hash recomputation) |
| Direct dependents        | none                                                                                                                                                                                                                    |
| Referenced GitHub issues | none                                                                                                                                                                                                                    |

<hr />

## M002 - Docs cleanup

Execute the five-at-a-time docs inventory recorded in
`docs/audits/docs-inventory-2026-04-26.md`. The old `docs/DOCS_AUDIT.md`
was deleted because it was stale; git history is the archive.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                         |
| ------------------------ | --------------------------------------------- |
| Source                   | METHOD task matrix                            |
| METHOD id                | M002                                          |
| Native id                | none                                          |
| Lane                     | asap                                          |
| Status                   | open                                          |
| Completed                | no                                            |
| Source path              | docs/method/backlog/asap/DOCS_docs-cleanup.md |
| Anchor/link              | docs/method/backlog/asap/DOCS_docs-cleanup.md |
| Direct blockers          | none                                          |
| Direct dependents        | none                                          |
| Referenced GitHub issues | none                                          |

<hr />

## M003 - Implement 1-thread vs N-thread determinism harness

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: none
DAG chain depth: downstream 2; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                             |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                |
| METHOD id                | M003                                                                                                              |
| Native id                | T-9-1-1                                                                                                           |
| Lane                     | asap                                                                                                              |
| Status                   | done                                                                                                              |
| Completed                | yes                                                                                                               |
| Source path              | docs/method/backlog/asap/KERNEL_determinism-torture.md                                                            |
| Anchor/link              | docs/method/backlog/asap/KERNEL_determinism-torture.md#t-9-1-1-implement-1-thread-vs-n-thread-determinism-harness |
| Direct blockers          | none                                                                                                              |
| Direct dependents        | M004 (Implement snapshot/restore fuzz)                                                                            |
| Referenced GitHub issues | none                                                                                                              |

<hr />

## M004 - Implement snapshot/restore fuzz

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M003 (Implement 1-thread vs N-thread determinism harness)
DAG chain depth: downstream 1; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                         |
| ------------------------ | --------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                            |
| METHOD id                | M004                                                                                          |
| Native id                | T-9-1-2                                                                                       |
| Lane                     | asap                                                                                          |
| Status                   | done                                                                                          |
| Completed                | yes                                                                                           |
| Source path              | docs/method/backlog/asap/KERNEL_determinism-torture.md                                        |
| Anchor/link              | docs/method/backlog/asap/KERNEL_determinism-torture.md#t-9-1-2-implement-snapshotrestore-fuzz |
| Direct blockers          | M003 (Implement 1-thread vs N-thread determinism harness)                                     |
| Direct dependents        | none                                                                                          |
| Referenced GitHub issues | none                                                                                          |

<hr />

## M005 - Echo and git-warp compatibility sanity check

A systematic review of where Echo and git-warp align, where they
diverge, and what needs to happen for the two substrates to share
a debugger, a protocol, and a schema compiler.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                       |
| ------------------------ | --------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                          |
| METHOD id                | M005                                                                        |
| Native id                | none                                                                        |
| Lane                     | asap                                                                        |
| Status                   | open                                                                        |
| Completed                | no                                                                          |
| Source path              | docs/method/backlog/asap/KERNEL_echo-git-warp-compatibility-sanity-check.md |
| Anchor/link              | docs/method/backlog/asap/KERNEL_echo-git-warp-compatibility-sanity-check.md |
| Direct blockers          | none                                                                        |
| Direct dependents        | none                                                                        |
| Referenced GitHub issues | none                                                                        |

<hr />

## M006 - Live holographic strands

Status: complete. The first live-basis strand slice is implemented across
strand reports, settlement planning, observation/read artifacts, and the
normative strand invariant.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: none
DAG chain depth: downstream 3; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                             |
| ------------------------ | ------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                |
| METHOD id                | M006                                                                                              |
| Native id                | none                                                                                              |
| Lane                     | asap                                                                                              |
| Status                   | done                                                                                              |
| Completed                | yes                                                                                               |
| Source path              | docs/method/backlog/asap/KERNEL_live-holographic-strands.md                                       |
| Anchor/link              | docs/method/backlog/asap/KERNEL_live-holographic-strands.md                                       |
| Direct blockers          | none                                                                                              |
| Direct dependents        | M046 (Contract Strands And Counterfactuals), M047 (Parent drift and owned-footprint revalidation) |
| Referenced GitHub issues | none                                                                                              |

<hr />

## M007 - Verify and integrate deterministic trig oracle into release gate

**User Story:** As a release engineer, I want a CI gate that verifies the deterministic trig oracle (sin/cos) produces identical results across macOS, Ubuntu, and Alpine so that cross-OS determinism is proven before every release.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                        |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                           |
| METHOD id                | M007                                                                                                                         |
| Native id                | T-9-3-1                                                                                                                      |
| Lane                     | asap                                                                                                                         |
| Status                   | open                                                                                                                         |
| Completed                | no                                                                                                                           |
| Source path              | docs/method/backlog/asap/MATH_deterministic-trig.md                                                                          |
| Anchor/link              | docs/method/backlog/asap/MATH_deterministic-trig.md#t-9-3-1-verify-and-integrate-deterministic-trig-oracle-into-release-gate |
| Direct blockers          | none                                                                                                                         |
| Direct dependents        | none                                                                                                                         |
| Referenced GitHub issues | none                                                                                                                         |

<hr />

## M008 - WESLEY Protocol Consumer Cutover

Coordination: `WESLEY_protocol_surface_cutover`

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: none
DAG chain depth: downstream 8; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                 |
| ------------------------ | --------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                    |
| METHOD id                | M008                                                                  |
| Native id                | none                                                                  |
| Lane                     | asap                                                                  |
| Status                   | done                                                                  |
| Completed                | yes                                                                   |
| Source path              | docs/method/backlog/asap/PLATFORM_WESLEY_protocol-consumer-cutover.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_WESLEY_protocol-consumer-cutover.md |
| Direct blockers          | none                                                                  |
| Direct dependents        | M079 (Wesley To Echo Toy Contract Proof)                              |
| Referenced GitHub issues | none                                                                  |

<hr />

## M009 - CI det-policy hardening

Harden the determinism classification CI pipeline.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #284, #285, #286
GH issue createdAt: #284: 2026-02-15T18:48:49Z, #285: 2026-02-15T18:48:55Z, #286: 2026-02-15T18:49:18Z

| Field                    | Value                                                        |
| ------------------------ | ------------------------------------------------------------ |
| Source                   | METHOD task matrix                                           |
| METHOD id                | M009                                                         |
| Native id                | none                                                         |
| Lane                     | asap                                                         |
| Status                   | open                                                         |
| Completed                | no                                                           |
| Source path              | docs/method/backlog/asap/PLATFORM_ci-det-policy-hardening.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_ci-det-policy-hardening.md |
| Direct blockers          | none                                                         |
| Direct dependents        | none                                                         |
| Referenced GitHub issues | #284, #285, #286                                             |

<hr />

## M010 - Bench subcommand -- criterion invocation and reporting

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M013 (clap subcommand structure and global flags)
DAG chain depth: downstream 2; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                      |
| ------------------------ | ---------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                         |
| METHOD id                | M010                                                                                                       |
| Native id                | T-6-3-1                                                                                                    |
| Lane                     | asap                                                                                                       |
| Status                   | done                                                                                                       |
| Completed                | yes                                                                                                        |
| Source path              | docs/method/backlog/asap/PLATFORM_cli-bench.md                                                             |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_cli-bench.md#t-6-3-1-bench-subcommand-criterion-invocation-and-reporting |
| Direct blockers          | M013 (clap subcommand structure and global flags)                                                          |
| Direct dependents        | M001 (Man page generation and README examples)                                                             |
| Referenced GitHub issues | none                                                                                                       |

<hr />

## M011 - Inspect subcommand -- metadata and graph stats

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M013 (clap subcommand structure and global flags)
DAG chain depth: downstream 2; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                |
| ------------------------ | ---------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                   |
| METHOD id                | M011                                                                                                 |
| Native id                | T-6-4-1                                                                                              |
| Lane                     | asap                                                                                                 |
| Status                   | done                                                                                                 |
| Completed                | yes                                                                                                  |
| Source path              | docs/method/backlog/asap/PLATFORM_cli-inspect.md                                                     |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_cli-inspect.md#t-6-4-1-inspect-subcommand-metadata-and-graph-stats |
| Direct blockers          | M013 (clap subcommand structure and global flags)                                                    |
| Direct dependents        | M001 (Man page generation and README examples), M012 (Inspect -- attachment payload pretty-printing) |
| Referenced GitHub issues | none                                                                                                 |

<hr />

## M012 - Inspect -- attachment payload pretty-printing

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M011 (Inspect subcommand -- metadata and graph stats)
DAG chain depth: downstream 1; upstream 3
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                               |
| ------------------------ | --------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                  |
| METHOD id                | M012                                                                                                |
| Native id                | T-6-4-2                                                                                             |
| Lane                     | asap                                                                                                |
| Status                   | done                                                                                                |
| Completed                | yes                                                                                                 |
| Source path              | docs/method/backlog/asap/PLATFORM_cli-inspect.md                                                    |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_cli-inspect.md#t-6-4-2-inspect-attachment-payload-pretty-printing |
| Direct blockers          | M011 (Inspect subcommand -- metadata and graph stats)                                               |
| Direct dependents        | none                                                                                                |
| Referenced GitHub issues | none                                                                                                |

<hr />

## M013 - clap subcommand structure and global flags

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: none
DAG chain depth: downstream 3; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                                                                                                                                                                  |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                                                                                                                                     |
| METHOD id                | M013                                                                                                                                                                                                                                                                   |
| Native id                | T-6-1-1                                                                                                                                                                                                                                                                |
| Lane                     | asap                                                                                                                                                                                                                                                                   |
| Status                   | done                                                                                                                                                                                                                                                                   |
| Completed                | yes                                                                                                                                                                                                                                                                    |
| Source path              | docs/method/backlog/asap/PLATFORM_cli-scaffold.md                                                                                                                                                                                                                      |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_cli-scaffold.md#t-6-1-1-clap-subcommand-structure-and-global-flags                                                                                                                                                                   |
| Direct blockers          | none                                                                                                                                                                                                                                                                   |
| Direct dependents        | M001 (Man page generation and README examples), M010 (Bench subcommand -- criterion invocation and reporting), M011 (Inspect subcommand -- metadata and graph stats), M014 (Config file support and shell completions), M015 (Verify subcommand -- hash recomputation) |
| Referenced GitHub issues | none                                                                                                                                                                                                                                                                   |

<hr />

## M014 - Config file support and shell completions

**User Story:** As a developer, I want to set default CLI options in a config file and generate shell completions so that the CLI is ergonomic for daily use.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: M013 (clap subcommand structure and global flags)
DAG chain depth: downstream 1; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                               |
| ------------------------ | --------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                  |
| METHOD id                | M014                                                                                                |
| Native id                | T-6-1-2                                                                                             |
| Lane                     | asap                                                                                                |
| Status                   | open                                                                                                |
| Completed                | no                                                                                                  |
| Source path              | docs/method/backlog/asap/PLATFORM_cli-scaffold.md                                                   |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_cli-scaffold.md#t-6-1-2-config-file-support-and-shell-completions |
| Direct blockers          | M013 (clap subcommand structure and global flags)                                                   |
| Direct dependents        | none                                                                                                |
| Referenced GitHub issues | none                                                                                                |

<hr />

## M015 - Verify subcommand -- hash recomputation

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M013 (clap subcommand structure and global flags)
DAG chain depth: downstream 2; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                        |
| ------------------------ | -------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                           |
| METHOD id                | M015                                                                                         |
| Native id                | T-6-2-1                                                                                      |
| Lane                     | asap                                                                                         |
| Status                   | done                                                                                         |
| Completed                | yes                                                                                          |
| Source path              | docs/method/backlog/asap/PLATFORM_cli-verify.md                                              |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_cli-verify.md#t-6-2-1-verify-subcommand-hash-recomputation |
| Direct blockers          | M013 (clap subcommand structure and global flags)                                            |
| Direct dependents        | M001 (Man page generation and README examples)                                               |
| Referenced GitHub issues | none                                                                                         |

<hr />

## M016 - Existing EINT, Registry, And Observation Boundary Inventory

Status: design packet complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M035 (Observer plans and reading artifacts), M039 (Wesley Compiled Contract Hosting Doctrine), M069 (Reading envelope family boundary)
DAG chain depth: downstream 9; upstream 3
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                                  |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                     |
| METHOD id                | M016                                                                                                                                   |
| Native id                | none                                                                                                                                   |
| Lane                     | asap                                                                                                                                   |
| Status                   | done                                                                                                                                   |
| Completed                | yes                                                                                                                                    |
| Source path              | docs/method/backlog/asap/PLATFORM_contract-aware-intent-observation-envelope.md                                                        |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_contract-aware-intent-observation-envelope.md                                                        |
| Direct blockers          | M035 (Observer plans and reading artifacts), M039 (Wesley Compiled Contract Hosting Doctrine), M069 (Reading envelope family boundary) |
| Direct dependents        | M036 (Registry Provider Wiring And Host Boundary Decision)                                                                             |
| Referenced GitHub issues | none                                                                                                                                   |

<hr />

## M017 - Make decoder control coverage auditable

**User Story:** As a security reviewer, I want each decoder rejection control to
point at an explicit negative test so that malformed-input coverage can be
audited without reading the whole decoder by hand.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                          |
| ------------------------ | -------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                             |
| METHOD id                | M017                                                                                                           |
| Native id                | T-279-1                                                                                                        |
| Lane                     | asap                                                                                                           |
| Status                   | open                                                                                                           |
| Completed                | no                                                                                                             |
| Source path              | docs/method/backlog/asap/PLATFORM_decoder-negative-test-map.md                                                 |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_decoder-negative-test-map.md#t-279-1-make-decoder-control-coverage-auditable |
| Direct blockers          | none                                                                                                           |
| Direct dependents        | none                                                                                                           |
| Referenced GitHub issues | none                                                                                                           |

<hr />

## M018 - Echo Contract Hosting Roadmap

Status: active sequencing card.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                              |
| ------------------------ | ------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                 |
| METHOD id                | M018                                                               |
| Native id                | none                                                               |
| Lane                     | asap                                                               |
| Status                   | open                                                               |
| Completed                | no                                                                 |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-contract-hosting-roadmap.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-contract-hosting-roadmap.md |
| Direct blockers          | none                                                               |
| Direct dependents        | none                                                               |
| Referenced GitHub issues | none                                                               |

<hr />

## M019 - Echo Optics ABI DTOs

Status: complete. The ABI exposes the optic DTO surface needed by generated
request builders, round-trips the required observe/dispatch DTO set
deterministically, and has a generated-helper-shaped smoke crate compiling
against `echo-wasm-abi`.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M025 (Echo Optics Dispatch Intent Model), M028 (Echo Optics Observe Model), M029 (Echo Optics Obstruction And Admission Results)
DAG chain depth: downstream 4; upstream 6
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                            |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                               |
| METHOD id                | M019                                                                                                                             |
| Native id                | none                                                                                                                             |
| Lane                     | asap                                                                                                                             |
| Status                   | done                                                                                                                             |
| Completed                | yes                                                                                                                              |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-abi-dtos.md                                                                        |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-abi-dtos.md                                                                        |
| Direct blockers          | M025 (Echo Optics Dispatch Intent Model), M028 (Echo Optics Observe Model), M029 (Echo Optics Obstruction And Admission Results) |
| Direct dependents        | M026 (Echo Optics Example Implementation), M034 (Echo Wesley Gen Optic Request Builders)                                         |
| Referenced GitHub issues | none                                                                                                                             |

<hr />

## M020 - Echo Optics Adapter Notes

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M026 (Echo Optics Example Implementation), M034 (Echo Wesley Gen Optic Request Builders)
DAG chain depth: downstream 1; upstream 9
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                    |
| ------------------------ | ---------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                       |
| METHOD id                | M020                                                                                     |
| Native id                | none                                                                                     |
| Lane                     | asap                                                                                     |
| Status                   | done                                                                                     |
| Completed                | yes                                                                                      |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-adapter-notes.md                           |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-adapter-notes.md                           |
| Direct blockers          | M026 (Echo Optics Example Implementation), M034 (Echo Wesley Gen Optic Request Builders) |
| Direct dependents        | none                                                                                     |
| Referenced GitHub issues | none                                                                                     |

<hr />

## M021 - Echo Optics API Design

Status: design/spec packet complete; executable work split into visible cards.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: none
DAG chain depth: downstream 9; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                       |
| ------------------------ | ----------------------------------------------------------- |
| Source                   | METHOD task matrix                                          |
| METHOD id                | M021                                                        |
| Native id                | none                                                        |
| Lane                     | asap                                                        |
| Status                   | done                                                        |
| Completed                | yes                                                         |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-api-design.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-api-design.md |
| Direct blockers          | none                                                        |
| Direct dependents        | M024 (Echo Optics Core Nouns And IDs)                       |
| Referenced GitHub issues | none                                                        |

<hr />

## M022 - Echo Optics Attachment Boundary Model

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M028 (Echo Optics Observe Model)
DAG chain depth: downstream 1; upstream 6
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                      |
| ------------------------ | -------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                         |
| METHOD id                | M022                                                                       |
| Native id                | none                                                                       |
| Lane                     | asap                                                                       |
| Status                   | done                                                                       |
| Completed                | yes                                                                        |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-attachment-boundary-model.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-attachment-boundary-model.md |
| Direct blockers          | M028 (Echo Optics Observe Model)                                           |
| Direct dependents        | none                                                                       |
| Referenced GitHub issues | none                                                                       |

<hr />

## M023 - Echo Optics Cached-Reading Identity Tests

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M031 (Echo Optics Reading Envelope And Identity), M033 (Echo Optics Witness Basis And Retained Key)
DAG chain depth: downstream 1; upstream 5
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                               |
| ------------------------ | --------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                  |
| METHOD id                | M023                                                                                                |
| Native id                | none                                                                                                |
| Lane                     | asap                                                                                                |
| Status                   | done                                                                                                |
| Completed                | yes                                                                                                 |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-cached-reading-identity-tests.md                      |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-cached-reading-identity-tests.md                      |
| Direct blockers          | M031 (Echo Optics Reading Envelope And Identity), M033 (Echo Optics Witness Basis And Retained Key) |
| Direct dependents        | none                                                                                                |
| Referenced GitHub issues | none                                                                                                |

<hr />

## M024 - Echo Optics Core Nouns And IDs

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M021 (Echo Optics API Design)
DAG chain depth: downstream 8; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                                            |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                                                                                               |
| METHOD id                | M024                                                                                                                                             |
| Native id                | none                                                                                                                                             |
| Lane                     | asap                                                                                                                                             |
| Status                   | done                                                                                                                                             |
| Completed                | yes                                                                                                                                              |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-core-nouns-and-ids.md                                                                              |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-core-nouns-and-ids.md                                                                              |
| Direct blockers          | M021 (Echo Optics API Design)                                                                                                                    |
| Direct dependents        | M029 (Echo Optics Obstruction And Admission Results), M030 (Echo Optics Open And Close Models), M031 (Echo Optics Reading Envelope And Identity) |
| Referenced GitHub issues | none                                                                                                                                             |

<hr />

## M025 - Echo Optics Dispatch Intent Model

Status: complete. Echo now has typed core/ABI optic dispatch request models,
an EINT v1 optic payload, an admission-law id, and a KernelPort route that
validates the optic proposal and carries EINT v1 through the existing
`dispatch_intent` path as a typed staged admission posture.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M029 (Echo Optics Obstruction And Admission Results), M030 (Echo Optics Open And Close Models), M040 (Witnessed suffix admission shells)
DAG chain depth: downstream 5; upstream 5
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                                    |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                       |
| METHOD id                | M025                                                                                                                                     |
| Native id                | none                                                                                                                                     |
| Lane                     | asap                                                                                                                                     |
| Status                   | done                                                                                                                                     |
| Completed                | yes                                                                                                                                      |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-dispatch-intent-model.md                                                                   |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-dispatch-intent-model.md                                                                   |
| Direct blockers          | M029 (Echo Optics Obstruction And Admission Results), M030 (Echo Optics Open And Close Models), M040 (Witnessed suffix admission shells) |
| Direct dependents        | M019 (Echo Optics ABI DTOs), M026 (Echo Optics Example Implementation), M032 (Echo Optics Stale-Basis Obstruction Tests)                 |
| Referenced GitHub issues | none                                                                                                                                     |

<hr />

## M026 - Echo Optics Example Implementation

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M019 (Echo Optics ABI DTOs), M025 (Echo Optics Dispatch Intent Model), M028 (Echo Optics Observe Model), M032 (Echo Optics Stale-Basis Obstruction Tests)
DAG chain depth: downstream 3; upstream 7
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                                                     |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                        |
| METHOD id                | M026                                                                                                                                                      |
| Native id                | none                                                                                                                                                      |
| Lane                     | asap                                                                                                                                                      |
| Status                   | done                                                                                                                                                      |
| Completed                | yes                                                                                                                                                       |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-example-implementation.md                                                                                   |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-example-implementation.md                                                                                   |
| Direct blockers          | M019 (Echo Optics ABI DTOs), M025 (Echo Optics Dispatch Intent Model), M028 (Echo Optics Observe Model), M032 (Echo Optics Stale-Basis Obstruction Tests) |
| Direct dependents        | M020 (Echo Optics Adapter Notes), M034 (Echo Wesley Gen Optic Request Builders)                                                                           |
| Referenced GitHub issues | none                                                                                                                                                      |

<hr />

## M027 - Echo Optics Live-Tail Honesty Tests

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M031 (Echo Optics Reading Envelope And Identity), M033 (Echo Optics Witness Basis And Retained Key)
DAG chain depth: downstream 1; upstream 5
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                               |
| ------------------------ | --------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                  |
| METHOD id                | M027                                                                                                |
| Native id                | none                                                                                                |
| Lane                     | asap                                                                                                |
| Status                   | done                                                                                                |
| Completed                | yes                                                                                                 |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-live-tail-honesty-tests.md                            |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-live-tail-honesty-tests.md                            |
| Direct blockers          | M031 (Echo Optics Reading Envelope And Identity), M033 (Echo Optics Witness Basis And Retained Key) |
| Direct dependents        | none                                                                                                |
| Referenced GitHub issues | none                                                                                                |

<hr />

## M028 - Echo Optics Observe Model

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M030 (Echo Optics Open And Close Models), M031 (Echo Optics Reading Envelope And Identity), M033 (Echo Optics Witness Basis And Retained Key)
DAG chain depth: downstream 5; upstream 5
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                                         |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                            |
| METHOD id                | M028                                                                                                                                          |
| Native id                | none                                                                                                                                          |
| Lane                     | asap                                                                                                                                          |
| Status                   | done                                                                                                                                          |
| Completed                | yes                                                                                                                                           |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-observe-model.md                                                                                |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-observe-model.md                                                                                |
| Direct blockers          | M030 (Echo Optics Open And Close Models), M031 (Echo Optics Reading Envelope And Identity), M033 (Echo Optics Witness Basis And Retained Key) |
| Direct dependents        | M019 (Echo Optics ABI DTOs), M022 (Echo Optics Attachment Boundary Model), M026 (Echo Optics Example Implementation)                          |
| Referenced GitHub issues | none                                                                                                                                          |

<hr />

## M029 - Echo Optics Obstruction And Admission Results

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M024 (Echo Optics Core Nouns And IDs)
DAG chain depth: downstream 7; upstream 3
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                           |
| ------------------------ | --------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                              |
| METHOD id                | M029                                                                                                            |
| Native id                | none                                                                                                            |
| Lane                     | asap                                                                                                            |
| Status                   | done                                                                                                            |
| Completed                | yes                                                                                                             |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-obstruction-admission-results.md                                  |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-obstruction-admission-results.md                                  |
| Direct blockers          | M024 (Echo Optics Core Nouns And IDs)                                                                           |
| Direct dependents        | M019 (Echo Optics ABI DTOs), M025 (Echo Optics Dispatch Intent Model), M030 (Echo Optics Open And Close Models) |
| Referenced GitHub issues | none                                                                                                            |

<hr />

## M030 - Echo Optics Open And Close Models

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M024 (Echo Optics Core Nouns And IDs), M029 (Echo Optics Obstruction And Admission Results)
DAG chain depth: downstream 6; upstream 4
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                       |
| ------------------------ | ------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                          |
| METHOD id                | M030                                                                                        |
| Native id                | none                                                                                        |
| Lane                     | asap                                                                                        |
| Status                   | done                                                                                        |
| Completed                | yes                                                                                         |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-open-close-models.md                          |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-open-close-models.md                          |
| Direct blockers          | M024 (Echo Optics Core Nouns And IDs), M029 (Echo Optics Obstruction And Admission Results) |
| Direct dependents        | M025 (Echo Optics Dispatch Intent Model), M028 (Echo Optics Observe Model)                  |
| Referenced GitHub issues | none                                                                                        |

<hr />

## M031 - Echo Optics Reading Envelope And Identity

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M024 (Echo Optics Core Nouns And IDs)
DAG chain depth: downstream 7; upstream 3
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                                                                             |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                                                |
| METHOD id                | M031                                                                                                                                                                              |
| Native id                | none                                                                                                                                                                              |
| Lane                     | asap                                                                                                                                                                              |
| Status                   | done                                                                                                                                                                              |
| Completed                | yes                                                                                                                                                                               |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-reading-envelope-identity.md                                                                                                        |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-reading-envelope-identity.md                                                                                                        |
| Direct blockers          | M024 (Echo Optics Core Nouns And IDs)                                                                                                                                             |
| Direct dependents        | M023 (Echo Optics Cached-Reading Identity Tests), M027 (Echo Optics Live-Tail Honesty Tests), M028 (Echo Optics Observe Model), M033 (Echo Optics Witness Basis And Retained Key) |
| Referenced GitHub issues | none                                                                                                                                                                              |

<hr />

## M032 - Echo Optics Stale-Basis Obstruction Tests

Status: complete. Core optic dispatch proposals can now be validated against a
known current coordinate, and engine-backed optic dispatch obstructs stale
worldline bases before staging EINT bytes.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M025 (Echo Optics Dispatch Intent Model)
DAG chain depth: downstream 4; upstream 6
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                          |
| ------------------------ | ------------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                             |
| METHOD id                | M032                                                                           |
| Native id                | none                                                                           |
| Lane                     | asap                                                                           |
| Status                   | done                                                                           |
| Completed                | yes                                                                            |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-stale-basis-obstruction-tests.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-stale-basis-obstruction-tests.md |
| Direct blockers          | M025 (Echo Optics Dispatch Intent Model)                                       |
| Direct dependents        | M026 (Echo Optics Example Implementation)                                      |
| Referenced GitHub issues | none                                                                           |

<hr />

## M033 - Echo Optics Witness Basis And Retained Key

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M031 (Echo Optics Reading Envelope And Identity)
DAG chain depth: downstream 6; upstream 4
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                          |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                                                                             |
| METHOD id                | M033                                                                                                                           |
| Native id                | none                                                                                                                           |
| Lane                     | asap                                                                                                                           |
| Status                   | done                                                                                                                           |
| Completed                | yes                                                                                                                            |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-optics-witness-basis-retained-key.md                                                    |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-optics-witness-basis-retained-key.md                                                    |
| Direct blockers          | M031 (Echo Optics Reading Envelope And Identity)                                                                               |
| Direct dependents        | M023 (Echo Optics Cached-Reading Identity Tests), M027 (Echo Optics Live-Tail Honesty Tests), M028 (Echo Optics Observe Model) |
| Referenced GitHub issues | none                                                                                                                           |

<hr />

## M034 - Echo Wesley Gen Optic Request Builders

Status: complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M019 (Echo Optics ABI DTOs), M026 (Echo Optics Example Implementation)
DAG chain depth: downstream 2; upstream 8
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                       |
| ------------------------ | --------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                          |
| METHOD id                | M034                                                                        |
| Native id                | none                                                                        |
| Lane                     | asap                                                                        |
| Status                   | done                                                                        |
| Completed                | yes                                                                         |
| Source path              | docs/method/backlog/asap/PLATFORM_echo-wesley-gen-optic-request-builders.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_echo-wesley-gen-optic-request-builders.md |
| Direct blockers          | M019 (Echo Optics ABI DTOs), M026 (Echo Optics Example Implementation)      |
| Direct dependents        | M020 (Echo Optics Adapter Notes)                                            |
| Referenced GitHub issues | none                                                                        |

<hr />

## M035 - Observer plans and reading artifacts

Status: complete. `ObservationService` and the ABI now emit one-shot built-in
observation artifacts with `ReadingEnvelope` metadata, and observation
requests explicitly name observer plan, optional hosted observer instance,
budget, and rights posture. Authored plans and hosted/stateful observer
instances are typed at the boundary and fail closed until an installed observer
host exists; query-shaped reads still exist only as a placeholder plan and
return unsupported at runtime.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: none
DAG chain depth: downstream 11; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                       |
| ------------------------ | ----------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                          |
| METHOD id                | M035                                                                                                        |
| Native id                | none                                                                                                        |
| Lane                     | asap                                                                                                        |
| Status                   | done                                                                                                        |
| Completed                | yes                                                                                                         |
| Source path              | docs/method/backlog/asap/PLATFORM_observer-plan-reading-artifacts.md                                        |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_observer-plan-reading-artifacts.md                                        |
| Direct blockers          | none                                                                                                        |
| Direct dependents        | M016 (Existing EINT, Registry, And Observation Boundary Inventory), M069 (Reading envelope family boundary) |
| Referenced GitHub issues | none                                                                                                        |

<hr />

## M036 - Registry Provider Wiring And Host Boundary Decision

Status: design packet complete.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: M016 (Existing EINT, Registry, And Observation Boundary Inventory)
DAG chain depth: downstream 8; upstream 4
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                           |
| ------------------------ | ------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                              |
| METHOD id                | M036                                                                            |
| Native id                | none                                                                            |
| Lane                     | asap                                                                            |
| Status                   | done                                                                            |
| Completed                | yes                                                                             |
| Source path              | docs/method/backlog/asap/PLATFORM_static-contract-registry-and-host-boundary.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_static-contract-registry-and-host-boundary.md |
| Direct blockers          | M016 (Existing EINT, Registry, And Observation Boundary Inventory)              |
| Direct dependents        | M079 (Wesley To Echo Toy Contract Proof)                                        |
| Referenced GitHub issues | none                                                                            |

<hr />

## M037 - Commit-ordered rollback playbooks for TTD integration

Ref: #282

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #282
GH issue createdAt: #282: 2026-02-14T16:39:19Z

| Field                    | Value                                                       |
| ------------------------ | ----------------------------------------------------------- |
| Source                   | METHOD task matrix                                          |
| METHOD id                | M037                                                        |
| Native id                | none                                                        |
| Lane                     | asap                                                        |
| Status                   | open                                                        |
| Completed                | no                                                          |
| Source path              | docs/method/backlog/asap/PLATFORM_ttd-rollback-playbooks.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_ttd-rollback-playbooks.md |
| Direct blockers          | none                                                        |
| Direct dependents        | none                                                        |
| Referenced GitHub issues | #282                                                        |

<hr />

## M038 - Reconcile TTD protocol schemas with warp-ttd

Status: active and partially implemented. Echo's generated Rust and TypeScript
protocol consumers are labeled as generated from the canonical `warp-ttd`
protocol, and `cargo xtask wesley sync` now verifies local downstream-consumer
provenance. The remaining gap is the full external handoff from the canonical
schema bundle to checked-in generated artifacts.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                          |
| ------------------------ | -------------------------------------------------------------- |
| Source                   | METHOD task matrix                                             |
| METHOD id                | M038                                                           |
| Native id                | none                                                           |
| Lane                     | asap                                                           |
| Status                   | open                                                           |
| Completed                | no                                                             |
| Source path              | docs/method/backlog/asap/PLATFORM_ttd-schema-reconciliation.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_ttd-schema-reconciliation.md |
| Direct blockers          | none                                                           |
| Direct dependents        | none                                                           |
| Referenced GitHub issues | none                                                           |

<hr />

## M039 - Wesley Compiled Contract Hosting Doctrine

Status: active planned design.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 10; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                          |
| ------------------------ | ------------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                             |
| METHOD id                | M039                                                                           |
| Native id                | none                                                                           |
| Lane                     | asap                                                                           |
| Status                   | open                                                                           |
| Completed                | no                                                                             |
| Source path              | docs/method/backlog/asap/PLATFORM_wesley-compiled-contract-hosting-doctrine.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_wesley-compiled-contract-hosting-doctrine.md |
| Direct blockers          | none                                                                           |
| Direct dependents        | M016 (Existing EINT, Registry, And Observation Boundary Inventory)             |
| Referenced GitHub issues | none                                                                           |

<hr />

## M040 - Witnessed suffix admission shells

Status: complete. Echo has settlement, neighborhood publication, observer
reading envelopes, design 0009 for witnessed causal suffix sync, and the first
Rust/ABI witnessed suffix export/import shell surfaces.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: none
DAG chain depth: downstream 6; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                                    |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                       |
| METHOD id                | M040                                                                                                                                     |
| Native id                | none                                                                                                                                     |
| Lane                     | asap                                                                                                                                     |
| Status                   | done                                                                                                                                     |
| Completed                | yes                                                                                                                                      |
| Source path              | docs/method/backlog/asap/PLATFORM_witnessed-suffix-admission-shells.md                                                                   |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_witnessed-suffix-admission-shells.md                                                                   |
| Direct blockers          | none                                                                                                                                     |
| Direct dependents        | M025 (Echo Optics Dispatch Intent Model), M066 (Import outcome idempotence and loop law), M158 (Continuum Contract Artifact Interchange) |
| Referenced GitHub issues | none                                                                                                                                     |

<hr />

## M041 - xtask method close

Status: complete. `cargo xtask method close [cycle]` now scaffolds
`docs/method/retro/<cycle>/retro.md` plus a `witness/` artifact directory for
an active cycle, defaults to the most recent active cycle, accepts a numeric or
full cycle selector, and refuses to overwrite existing retro material.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                   |
| ------------------------ | ------------------------------------------------------- |
| Source                   | METHOD task matrix                                      |
| METHOD id                | M041                                                    |
| Native id                | none                                                    |
| Lane                     | asap                                                    |
| Status                   | done                                                    |
| Completed                | yes                                                     |
| Source path              | docs/method/backlog/asap/PLATFORM_xtask-method-close.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_xtask-method-close.md |
| Direct blockers          | none                                                    |
| Direct dependents        | none                                                    |
| Referenced GitHub issues | none                                                    |

<hr />

## M042 - xtask method drift

Status: complete. `cargo xtask method drift [cycle]` checks playback questions
from active cycle design docs against committed test files, supports JSON output
for agents, reports matched and missing questions, and exits nonzero when any
playback question lacks visible test coverage.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                   |
| ------------------------ | ------------------------------------------------------- |
| Source                   | METHOD task matrix                                      |
| METHOD id                | M042                                                    |
| Native id                | none                                                    |
| Lane                     | asap                                                    |
| Status                   | done                                                    |
| Completed                | yes                                                     |
| Source path              | docs/method/backlog/asap/PLATFORM_xtask-method-drift.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_xtask-method-drift.md |
| Direct blockers          | none                                                    |
| Direct dependents        | none                                                    |
| Referenced GitHub issues | none                                                    |

<hr />

## M043 - xtask method pull

Status: complete. `cargo xtask method pull <item>` promotes a backlog markdown
file into the next numbered `docs/design/<cycle>/` directory, accepts a source
path, file stem, generated METHOD id, or native task id, strips uppercase legend
prefixes from the design filename, and refuses ambiguous or missing selectors.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail unless the completed METHOD row is intentionally being pruned from history.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                  |
| ------------------------ | ------------------------------------------------------ |
| Source                   | METHOD task matrix                                     |
| METHOD id                | M043                                                   |
| Native id                | none                                                   |
| Lane                     | asap                                                   |
| Status                   | done                                                   |
| Completed                | yes                                                    |
| Source path              | docs/method/backlog/asap/PLATFORM_xtask-method-pull.md |
| Anchor/link              | docs/method/backlog/asap/PLATFORM_xtask-method-pull.md |
| Direct blockers          | none                                                   |
| Direct dependents        | none                                                   |
| Referenced GitHub issues | none                                                   |

<hr />

## M044 - Compliance reporting as a TTD protocol extension

`echo-ttd` produces `Violation` records (policy, footprint,
determinism, hashing) via its `PolicyChecker`. These are valuable
debugging information but have no way to reach warp-ttd's UI.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                              |
| ------------------------ | ------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                 |
| METHOD id                | M044                                                               |
| Native id                | none                                                               |
| Lane                     | up-next                                                            |
| Status                   | open                                                               |
| Completed                | no                                                                 |
| Source path              | docs/method/backlog/up-next/KERNEL_compliance-protocol-envelope.md |
| Anchor/link              | docs/method/backlog/up-next/KERNEL_compliance-protocol-envelope.md |
| Direct blockers          | none                                                               |
| Direct dependents        | none                                                               |
| Referenced GitHub issues | none                                                               |

<hr />

## M045 - Contract-Aware Receipts And Readings

Status: planned kernel hardening.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M069 (Reading envelope family boundary), M079 (Wesley To Echo Toy Contract Proof)
DAG chain depth: downstream 6; upstream 6
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                |
| ------------------------ | ---------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                   |
| METHOD id                | M045                                                                                                 |
| Native id                | none                                                                                                 |
| Lane                     | up-next                                                                                              |
| Status                   | blocked                                                                                              |
| Completed                | no                                                                                                   |
| Source path              | docs/method/backlog/up-next/KERNEL_contract-aware-receipts-and-readings.md                           |
| Anchor/link              | docs/method/backlog/up-next/KERNEL_contract-aware-receipts-and-readings.md                           |
| Direct blockers          | M069 (Reading envelope family boundary), M079 (Wesley To Echo Toy Contract Proof)                    |
| Direct dependents        | M053 (Authenticated Wesley Intent Admission Posture), M058 (Contract Artifact Retention In echo-cas) |
| Referenced GitHub issues | none                                                                                                 |

<hr />

## M046 - Contract Strands And Counterfactuals

Status: planned kernel/runtime implementation.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M006 (Live holographic strands), M065 (Graft Live Frontier Structural Readings)
DAG chain depth: downstream 2; upstream 10
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                           |
| ------------------------ | ------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                              |
| METHOD id                | M046                                                                            |
| Native id                | none                                                                            |
| Lane                     | up-next                                                                         |
| Status                   | blocked                                                                         |
| Completed                | no                                                                              |
| Source path              | docs/method/backlog/up-next/KERNEL_contract-strands-and-counterfactuals.md      |
| Anchor/link              | docs/method/backlog/up-next/KERNEL_contract-strands-and-counterfactuals.md      |
| Direct blockers          | M006 (Live holographic strands), M065 (Graft Live Frontier Structural Readings) |
| Direct dependents        | M158 (Continuum Contract Artifact Interchange)                                  |
| Referenced GitHub issues | none                                                                            |

<hr />

## M047 - Parent drift and owned-footprint revalidation

Depends on:

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: M006 (Live holographic strands)
DAG chain depth: downstream 1; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                           |
| ------------------------ | ------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                              |
| METHOD id                | M047                                                                            |
| Native id                | none                                                                            |
| Lane                     | up-next                                                                         |
| Status                   | open                                                                            |
| Completed                | no                                                                              |
| Source path              | docs/method/backlog/up-next/KERNEL_parent-drift-owned-footprint-revalidation.md |
| Anchor/link              | docs/method/backlog/up-next/KERNEL_parent-drift-owned-footprint-revalidation.md |
| Direct blockers          | M006 (Live holographic strands)                                                 |
| Direct dependents        | none                                                                            |
| Referenced GitHub issues | none                                                                            |

<hr />

## M048 - SHA-256 to BLAKE3 migration spec

**User Story:** As a cross-project architect, I want a written migration plan for switching Wesley from SHA-256 to BLAKE3 so that both repos use the same hash algorithm and the transition is safe.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                        |
| ------------------------ | -------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                           |
| METHOD id                | M048                                                                                         |
| Native id                | T-2-5-1                                                                                      |
| Lane                     | up-next                                                                                      |
| Status                   | open                                                                                         |
| Completed                | no                                                                                           |
| Source path              | docs/method/backlog/up-next/KERNEL_sha256-blake3.md                                          |
| Anchor/link              | docs/method/backlog/up-next/KERNEL_sha256-blake3.md#t-2-5-1-sha-256-to-blake3-migration-spec |
| Direct blockers          | none                                                                                         |
| Direct dependents        | none                                                                                         |
| Referenced GitHub issues | none                                                                                         |

<hr />

## M049 - Spec — HistoryTime vs HostTime field classification (#191)

Status: complete. This stale task was compressed into current invariant doctrine rather than implemented against the obsolete `docs/spec-time-streams-and-wormholes.md` target. The current source is `docs/invariants/FIXED-TIMESTEP.md`: R5 covers tick-denominated TTL/deadline semantics, R6 forbids HostTime from directly affecting admission, commit identity, read identity, replay outcome, or causal ordering, and the Time Field Classification section names the obvious HistoryTime and HostTime surfaces. Static evidence is `scripts/ban-nondeterminism.sh`, which bans wall-clock APIs in determinism-critical crate paths.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as an audit trail. The old full-table task is closed as compressed/superseded, but the doctrine is load-bearing and now lives in the current invariant doc.

DAG blocked by: none
DAG chain depth: downstream 10; upstream 1
GH issue #: #191
GH issue createdAt: #191: 2026-01-02T16:56:17Z

| Field                    | Value                                                                                                                                                                                                                                                                                             |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                                                                                                                                                                |
| METHOD id                | M049                                                                                                                                                                                                                                                                                              |
| Native id                | T-7-1-1                                                                                                                                                                                                                                                                                           |
| Lane                     | up-next                                                                                                                                                                                                                                                                                           |
| Status                   | done                                                                                                                                                                                                                                                                                              |
| Completed                | yes                                                                                                                                                                                                                                                                                               |
| Source path              | docs/method/backlog/up-next/KERNEL_time-model-spec.md                                                                                                                                                                                                                                             |
| Anchor/link              | docs/method/backlog/up-next/KERNEL_time-model-spec.md#t-7-1-1-spec-historytime-vs-hosttime-field-classification-191                                                                                                                                                                               |
| Direct blockers          | none                                                                                                                                                                                                                                                                                              |
| Direct dependents        | M050 (Spec — TTL/deadline semantics are ticks only (#192)), M173 (Spec — dt policy: fixed timestep vs admitted dt stream (#243)), M174 (Spec — TimeStream retention, spool compaction, wormhole density (#244)), M175 (Spec — Merge semantics for admitted stream facts across worldlines (#245)) |
| Referenced GitHub issues | #191                                                                                                                                                                                                                                                                                              |

<hr />

## M050 - Spec — TTL/deadline semantics are ticks only (#192)

**User Story:** As a game designer using Echo, I want certainty that all TTL and deadline semantics use deterministic tick/epoch counts so that my game logic replays identically regardless of host performance.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M049 (Spec — HistoryTime vs HostTime field classification (#191))
DAG chain depth: downstream 9; upstream 2
GH issue #: #192
GH issue createdAt: #192: 2026-01-02T16:56:18Z

| Field                    | Value                                                                                                                                                                                                                                 |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                                                                                                    |
| METHOD id                | M050                                                                                                                                                                                                                                  |
| Native id                | T-7-1-2                                                                                                                                                                                                                               |
| Lane                     | up-next                                                                                                                                                                                                                               |
| Status                   | blocked                                                                                                                                                                                                                               |
| Completed                | no                                                                                                                                                                                                                                    |
| Source path              | docs/method/backlog/up-next/KERNEL_time-model-spec.md                                                                                                                                                                                 |
| Anchor/link              | docs/method/backlog/up-next/KERNEL_time-model-spec.md#t-7-1-2-spec-ttldeadline-semantics-are-ticks-only-192                                                                                                                           |
| Direct blockers          | M049 (Spec — HistoryTime vs HostTime field classification (#191))                                                                                                                                                                     |
| Direct dependents        | M173 (Spec — dt policy: fixed timestep vs admitted dt stream (#243)), M174 (Spec — TimeStream retention, spool compaction, wormhole density (#244)), M175 (Spec — Merge semantics for admitted stream facts across worldlines (#245)) |
| Referenced GitHub issues | #192                                                                                                                                                                                                                                  |

<hr />

## M051 - Security/capabilities for fork/rewind/merge

Ref: #246

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #246
GH issue createdAt: #246: 2026-01-03T01:20:55Z

| Field                    | Value                                                          |
| ------------------------ | -------------------------------------------------------------- |
| Source                   | METHOD task matrix                                             |
| METHOD id                | M051                                                           |
| Native id                | none                                                           |
| Lane                     | up-next                                                        |
| Status                   | open                                                           |
| Completed                | no                                                             |
| Source path              | docs/method/backlog/up-next/KERNEL_time-travel-capabilities.md |
| Anchor/link              | docs/method/backlog/up-next/KERNEL_time-travel-capabilities.md |
| Direct blockers          | none                                                           |
| Direct dependents        | none                                                           |
| Referenced GitHub issues | #246                                                           |

<hr />

## M052 - TimeStream retention + spool compaction + wormhole density

Ref: #244

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #244
GH issue createdAt: #244: 2026-01-03T01:20:24Z

| Field                    | Value                                                      |
| ------------------------ | ---------------------------------------------------------- |
| Source                   | METHOD task matrix                                         |
| METHOD id                | M052                                                       |
| Native id                | none                                                       |
| Lane                     | up-next                                                    |
| Status                   | open                                                       |
| Completed                | no                                                         |
| Source path              | docs/method/backlog/up-next/KERNEL_timestream-retention.md |
| Anchor/link              | docs/method/backlog/up-next/KERNEL_timestream-retention.md |
| Direct blockers          | none                                                       |
| Direct dependents        | none                                                       |
| Referenced GitHub issues | #244                                                       |

<hr />

## M053 - Authenticated Wesley Intent Admission Posture

Status: proposed security hardening.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M045 (Contract-Aware Receipts And Readings), M079 (Wesley To Echo Toy Contract Proof)
DAG chain depth: downstream 1; upstream 7
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                 |
| ------------------------ | ------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                    |
| METHOD id                | M053                                                                                  |
| Native id                | none                                                                                  |
| Lane                     | up-next                                                                               |
| Status                   | blocked                                                                               |
| Completed                | no                                                                                    |
| Source path              | docs/method/backlog/up-next/PLATFORM_authenticated-wesley-intent-admission-posture.md |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_authenticated-wesley-intent-admission-posture.md |
| Direct blockers          | M045 (Contract-Aware Receipts And Readings), M079 (Wesley To Echo Toy Contract Proof) |
| Direct dependents        | none                                                                                  |
| Referenced GitHub issues | none                                                                                  |

<hr />

## M054 - Canvas graph renderer (static materialized reading)

**User Story:** As a user, I want to see the simulation's current graph-shaped reading rendered visually so that I can understand the entity structure at a glance.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M071 (Wire Engine lifecycle behind wasm-bindgen exports), M073 (JS/WASM memory bridge and error protocol)
DAG chain depth: downstream 3; upstream 3
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                   |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                      |
| METHOD id                | M054                                                                                                                    |
| Native id                | T-4-2-1                                                                                                                 |
| Lane                     | up-next                                                                                                                 |
| Status                   | blocked                                                                                                                 |
| Completed                | no                                                                                                                      |
| Source path              | docs/method/backlog/up-next/PLATFORM_browser-visualization.md                                                           |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_browser-visualization.md#t-4-2-1-canvas-graph-renderer-static-materialized-reading |
| Direct blockers          | M071 (Wire Engine lifecycle behind wasm-bindgen exports), M073 (JS/WASM memory bridge and error protocol)               |
| Direct dependents        | M055 (Live tick playback and rewrite animation)                                                                         |
| Referenced GitHub issues | none                                                                                                                    |

<hr />

## M055 - Live tick playback and rewrite animation

**User Story:** As a user, I want to step through ticks and see graph rewrites animate so that I can understand causal relationships between rules.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M054 (Canvas graph renderer (static materialized reading)), M072 (Snapshot and ViewOp drain exports)
DAG chain depth: downstream 2; upstream 4
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                          |
| ------------------------ | -------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                             |
| METHOD id                | M055                                                                                                           |
| Native id                | T-4-2-2                                                                                                        |
| Lane                     | up-next                                                                                                        |
| Status                   | blocked                                                                                                        |
| Completed                | no                                                                                                             |
| Source path              | docs/method/backlog/up-next/PLATFORM_browser-visualization.md                                                  |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_browser-visualization.md#t-4-2-2-live-tick-playback-and-rewrite-animation |
| Direct blockers          | M054 (Canvas graph renderer (static materialized reading)), M072 (Snapshot and ViewOp drain exports)           |
| Direct dependents        | M056 (Node inspection panel)                                                                                   |
| Referenced GitHub issues | none                                                                                                           |

<hr />

## M056 - Node inspection panel

**User Story:** As a user, I want to click a node and see its properties, attachments, and connected edges so that I can debug simulation state.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M055 (Live tick playback and rewrite animation)
DAG chain depth: downstream 1; upstream 5
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                       |
| ------------------------ | ------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                          |
| METHOD id                | M056                                                                                        |
| Native id                | T-4-2-3                                                                                     |
| Lane                     | up-next                                                                                     |
| Status                   | blocked                                                                                     |
| Completed                | no                                                                                          |
| Source path              | docs/method/backlog/up-next/PLATFORM_browser-visualization.md                               |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_browser-visualization.md#t-4-2-3-node-inspection-panel |
| Direct blockers          | M055 (Live tick playback and rewrite animation)                                             |
| Direct dependents        | none                                                                                        |
| Referenced GitHub issues | none                                                                                        |

<hr />

## M057 - Continuum Proof Family Runtime Cutover

- Lane: `up-next`
- Legend: `PLATFORM`
- Rank: `1`

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                          |
| ------------------------ | ------------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                             |
| METHOD id                | M057                                                                           |
| Native id                | none                                                                           |
| Lane                     | up-next                                                                        |
| Status                   | open                                                                           |
| Completed                | no                                                                             |
| Source path              | docs/method/backlog/up-next/PLATFORM_continuum-proof-family-runtime-cutover.md |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_continuum-proof-family-runtime-cutover.md |
| Direct blockers          | none                                                                           |
| Direct dependents        | none                                                                           |
| Referenced GitHub issues | none                                                                           |

<hr />

## M058 - Contract Artifact Retention In echo-cas

Status: planned platform implementation.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M045 (Contract-Aware Receipts And Readings), M060 (MemoryTier WASM compilation gate), M061 (JS bindings for CAS store/retrieve)
DAG chain depth: downstream 5; upstream 7
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                           |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                              |
| METHOD id                | M058                                                                                                                            |
| Native id                | none                                                                                                                            |
| Lane                     | up-next                                                                                                                         |
| Status                   | blocked                                                                                                                         |
| Completed                | no                                                                                                                              |
| Source path              | docs/method/backlog/up-next/PLATFORM_contract-artifact-retention-in-echo-cas.md                                                 |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_contract-artifact-retention-in-echo-cas.md                                                 |
| Direct blockers          | M045 (Contract-Aware Receipts And Readings), M060 (MemoryTier WASM compilation gate), M061 (JS bindings for CAS store/retrieve) |
| Direct dependents        | M067 (jedit Text Contract MVP)                                                                                                  |
| Referenced GitHub issues | none                                                                                                                            |

<hr />

## M059 - Add an explicit Echo CLI and MCP agent surface

Echo is browser-hostable and increasingly Continuum-aligned, but it is still
not agent-native in the METHOD sense.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                  |
| ------------------------ | ---------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                     |
| METHOD id                | M059                                                                   |
| Native id                | none                                                                   |
| Lane                     | up-next                                                                |
| Status                   | open                                                                   |
| Completed                | no                                                                     |
| Source path              | docs/method/backlog/up-next/PLATFORM_echo-agent-surface-cli-and-mcp.md |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_echo-agent-surface-cli-and-mcp.md |
| Direct blockers          | none                                                                   |
| Direct dependents        | none                                                                   |
| Referenced GitHub issues | none                                                                   |

<hr />

## M060 - MemoryTier WASM compilation gate

**User Story:** As a developer, I want echo-cas to compile to `wasm32-unknown-unknown` so that the browser demo can use content-addressed storage.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 7; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                             |
| ------------------------ | ------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                |
| METHOD id                | M060                                                                                              |
| Native id                | T-4-3-1                                                                                           |
| Lane                     | up-next                                                                                           |
| Status                   | open                                                                                              |
| Completed                | no                                                                                                |
| Source path              | docs/method/backlog/up-next/PLATFORM_echo-cas-browser.md                                          |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_echo-cas-browser.md#t-4-3-1-memorytier-wasm-compilation-gate |
| Direct blockers          | none                                                                                              |
| Direct dependents        | M058 (Contract Artifact Retention In echo-cas), M061 (JS bindings for CAS store/retrieve)         |
| Referenced GitHub issues | none                                                                                              |

<hr />

## M061 - JS bindings for CAS store/retrieve

**User Story:** As a web developer, I want to store and retrieve blobs from JavaScript so that the demo can persist simulation snapshots in content-addressed storage.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M060 (MemoryTier WASM compilation gate)
DAG chain depth: downstream 6; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                              |
| ------------------------ | -------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                 |
| METHOD id                | M061                                                                                               |
| Native id                | T-4-3-2                                                                                            |
| Lane                     | up-next                                                                                            |
| Status                   | blocked                                                                                            |
| Completed                | no                                                                                                 |
| Source path              | docs/method/backlog/up-next/PLATFORM_echo-cas-browser.md                                           |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_echo-cas-browser.md#t-4-3-2-js-bindings-for-cas-storeretrieve |
| Direct blockers          | M060 (MemoryTier WASM compilation gate)                                                            |
| Direct dependents        | M058 (Contract Artifact Retention In echo-cas)                                                     |
| Referenced GitHub issues | none                                                                                               |

<hr />

## M062 - Echo / git-warp witnessed suffix sync

- Lane: `up-next`
- Legend: `PLATFORM`
- Rank: `1`

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                       |
| ------------------------ | --------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                          |
| METHOD id                | M062                                                                        |
| Native id                | none                                                                        |
| Lane                     | up-next                                                                     |
| Status                   | open                                                                        |
| Completed                | no                                                                          |
| Source path              | docs/method/backlog/up-next/PLATFORM_echo-git-warp-witnessed-suffix-sync.md |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_echo-git-warp-witnessed-suffix-sync.md |
| Direct blockers          | none                                                                        |
| Direct dependents        | none                                                                        |
| Referenced GitHub issues | none                                                                        |

<hr />

## M063 - Split echo-session-proto into retained bridge contracts vs legacy transport residue

`echo-session-proto` still mixes two different things:

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                            |
| ------------------------ | ---------------------------------------------------------------- |
| Source                   | METHOD task matrix                                               |
| METHOD id                | M063                                                             |
| Native id                | none                                                             |
| Lane                     | up-next                                                          |
| Status                   | open                                                             |
| Completed                | no                                                               |
| Source path              | docs/method/backlog/up-next/PLATFORM_echo-session-proto-split.md |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_echo-session-proto-split.md |
| Direct blockers          | none                                                             |
| Direct dependents        | none                                                             |
| Referenced GitHub issues | none                                                             |

<hr />

## M064 - Update echo-wesley-gen IR deserializer for v2 format

**User Story:** As an Echo developer, I want echo-wesley-gen to consume the v2 IR format so that new Wesley features (QIR operations, migration metadata) are available in generated Rust types.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 8; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                   |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                      |
| METHOD id                | M064                                                                                                                    |
| Native id                | T-2-4-1                                                                                                                 |
| Lane                     | up-next                                                                                                                 |
| Status                   | open                                                                                                                    |
| Completed                | no                                                                                                                      |
| Source path              | docs/method/backlog/up-next/PLATFORM_echo-wesley-gen-v2.md                                                              |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_echo-wesley-gen-v2.md#t-2-4-1-update-echo-wesley-gen-ir-deserializer-for-v2-format |
| Direct blockers          | none                                                                                                                    |
| Direct dependents        | M079 (Wesley To Echo Toy Contract Proof)                                                                                |
| Referenced GitHub issues | none                                                                                                                    |

<hr />

## M065 - Graft Live Frontier Structural Readings

Status: planned consumer integration.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M067 (jedit Text Contract MVP)
DAG chain depth: downstream 3; upstream 9
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                           |
| ------------------------ | ------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                              |
| METHOD id                | M065                                                                            |
| Native id                | none                                                                            |
| Lane                     | up-next                                                                         |
| Status                   | blocked                                                                         |
| Completed                | no                                                                              |
| Source path              | docs/method/backlog/up-next/PLATFORM_graft-live-frontier-structural-readings.md |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_graft-live-frontier-structural-readings.md |
| Direct blockers          | M067 (jedit Text Contract MVP)                                                  |
| Direct dependents        | M046 (Contract Strands And Counterfactuals)                                     |
| Referenced GitHub issues | none                                                                            |

<hr />

## M066 - Import outcome idempotence and loop law

Depends on:

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: M040 (Witnessed suffix admission shells)
DAG chain depth: downstream 1; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                           |
| ------------------------ | ------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                              |
| METHOD id                | M066                                                                            |
| Native id                | none                                                                            |
| Lane                     | up-next                                                                         |
| Status                   | open                                                                            |
| Completed                | no                                                                              |
| Source path              | docs/method/backlog/up-next/PLATFORM_import-outcome-idempotence-and-loop-law.md |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_import-outcome-idempotence-and-loop-law.md |
| Direct blockers          | M040 (Witnessed suffix admission shells)                                        |
| Direct dependents        | none                                                                            |
| Referenced GitHub issues | none                                                                            |

<hr />

## M067 - jedit Text Contract MVP

Status: planned consumer proof.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M058 (Contract Artifact Retention In echo-cas)
DAG chain depth: downstream 4; upstream 8
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                           |
| ------------------------ | --------------------------------------------------------------- |
| Source                   | METHOD task matrix                                              |
| METHOD id                | M067                                                            |
| Native id                | none                                                            |
| Lane                     | up-next                                                         |
| Status                   | blocked                                                         |
| Completed                | no                                                              |
| Source path              | docs/method/backlog/up-next/PLATFORM_jedit-text-contract-mvp.md |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_jedit-text-contract-mvp.md |
| Direct blockers          | M058 (Contract Artifact Retention In echo-cas)                  |
| Direct dependents        | M065 (Graft Live Frontier Structural Readings)                  |
| Referenced GitHub issues | none                                                            |

<hr />

## M068 - Triage METHOD drift against ~/git/method

Echo already has METHOD scaffolding and active cycle/backlog structure, so this
should not become an open-ended "refresh everything" cleanup pass.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                 |
| ------------------------ | --------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                    |
| METHOD id                | M068                                                                  |
| Native id                | none                                                                  |
| Lane                     | up-next                                                               |
| Status                   | open                                                                  |
| Completed                | no                                                                    |
| Source path              | docs/method/backlog/up-next/PLATFORM_method-sync-and-doctor-triage.md |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_method-sync-and-doctor-triage.md |
| Direct blockers          | none                                                                  |
| Direct dependents        | none                                                                  |
| Referenced GitHub issues | none                                                                  |

<hr />

## M069 - Reading envelope family boundary

Depends on:

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: M035 (Observer plans and reading artifacts)
DAG chain depth: downstream 10; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                           |
| ------------------------ | --------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                              |
| METHOD id                | M069                                                                                                            |
| Native id                | none                                                                                                            |
| Lane                     | up-next                                                                                                         |
| Status                   | open                                                                                                            |
| Completed                | no                                                                                                              |
| Source path              | docs/method/backlog/up-next/PLATFORM_reading-envelope-family-boundary.md                                        |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_reading-envelope-family-boundary.md                                        |
| Direct blockers          | M035 (Observer plans and reading artifacts)                                                                     |
| Direct dependents        | M016 (Existing EINT, Registry, And Observation Boundary Inventory), M045 (Contract-Aware Receipts And Readings) |
| Referenced GitHub issues | none                                                                                                            |

<hr />

## M070 - Narrow ttd-browser into an Echo browser host bridge

`ttd-browser` proved useful browser/WASM ideas before `warp-ttd` existed as
its own debugger product. That history is valuable, but the ownership split is
different now:

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                           |
| ------------------------ | --------------------------------------------------------------- |
| Source                   | METHOD task matrix                                              |
| METHOD id                | M070                                                            |
| Native id                | none                                                            |
| Lane                     | up-next                                                         |
| Status                   | open                                                            |
| Completed                | no                                                              |
| Source path              | docs/method/backlog/up-next/PLATFORM_ttd-browser-host-bridge.md |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_ttd-browser-host-bridge.md |
| Direct blockers          | none                                                            |
| Direct dependents        | none                                                            |
| Referenced GitHub issues | none                                                            |

<hr />

## M071 - Wire Engine lifecycle behind wasm-bindgen exports

**User Story:** As a web developer, I want the WASM module to expose a real Engine instance so that I can drive tick execution from JavaScript.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 5; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                                                 |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                    |
| METHOD id                | M071                                                                                                                                                  |
| Native id                | T-4-1-1                                                                                                                                               |
| Lane                     | up-next                                                                                                                                               |
| Status                   | open                                                                                                                                                  |
| Completed                | no                                                                                                                                                    |
| Source path              | docs/method/backlog/up-next/PLATFORM_wasm-runtime.md                                                                                                  |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_wasm-runtime.md#t-4-1-1-wire-engine-lifecycle-behind-wasm-bindgen-exports                                        |
| Direct blockers          | none                                                                                                                                                  |
| Direct dependents        | M054 (Canvas graph renderer (static materialized reading)), M072 (Snapshot and ViewOp drain exports), M073 (JS/WASM memory bridge and error protocol) |
| Referenced GitHub issues | none                                                                                                                                                  |

<hr />

## M072 - Snapshot and ViewOp drain exports

**User Story:** As a web developer, I want to drain ViewOps and request snapshots at specific ticks so that I can render simulation state in the browser.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M071 (Wire Engine lifecycle behind wasm-bindgen exports)
DAG chain depth: downstream 3; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                          |
| ------------------------ | ---------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                             |
| METHOD id                | M072                                                                                           |
| Native id                | T-4-1-2                                                                                        |
| Lane                     | up-next                                                                                        |
| Status                   | blocked                                                                                        |
| Completed                | no                                                                                             |
| Source path              | docs/method/backlog/up-next/PLATFORM_wasm-runtime.md                                           |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_wasm-runtime.md#t-4-1-2-snapshot-and-viewop-drain-exports |
| Direct blockers          | M071 (Wire Engine lifecycle behind wasm-bindgen exports)                                       |
| Direct dependents        | M055 (Live tick playback and rewrite animation)                                                |
| Referenced GitHub issues | none                                                                                           |

<hr />

## M073 - JS/WASM memory bridge and error protocol

**User Story:** As a web developer, I want a clean TypeScript API wrapper around the raw WASM exports so that I do not deal with raw Uint8Array encoding/decoding.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M071 (Wire Engine lifecycle behind wasm-bindgen exports)
DAG chain depth: downstream 4; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                   |
| METHOD id                | M073                                                                                                                 |
| Native id                | T-4-1-3                                                                                                              |
| Lane                     | up-next                                                                                                              |
| Status                   | blocked                                                                                                              |
| Completed                | no                                                                                                                   |
| Source path              | docs/method/backlog/up-next/PLATFORM_wasm-runtime.md                                                                 |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_wasm-runtime.md#t-4-1-3-jswasm-memory-bridge-and-error-protocol                 |
| Direct blockers          | M071 (Wire Engine lifecycle behind wasm-bindgen exports)                                                             |
| Direct dependents        | M054 (Canvas graph renderer (static materialized reading)), M082 (CBOR serialization bridge (TS types to WASM Rust)) |
| Referenced GitHub issues | none                                                                                                                 |

<hr />

## M074 - README, contributor guide, and CI hardening

**User Story:** As a potential Wesley contributor, I want clear onboarding documentation and a reliable CI pipeline so that I can understand the project and submit quality PRs.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                      |
| ------------------------ | ---------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                         |
| METHOD id                | M074                                                                                                       |
| Native id                | T-2-3-1                                                                                                    |
| Lane                     | up-next                                                                                                    |
| Status                   | open                                                                                                       |
| Completed                | no                                                                                                         |
| Source path              | docs/method/backlog/up-next/PLATFORM_wesley-go-public.md                                                   |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_wesley-go-public.md#t-2-3-1-readme-contributor-guide-and-ci-hardening |
| Direct blockers          | none                                                                                                       |
| Direct dependents        | none                                                                                                       |
| Referenced GitHub issues | none                                                                                                       |

<hr />

## M075 - Backfill script generation for schema migrations

**User Story:** As a Wesley user, I want automatic backfill script generation when a schema migration adds or transforms fields so that I can safely evolve my schema without data loss.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 2; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                             |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                |
| METHOD id                | M075                                                                                                              |
| Native id                | T-2-2-1                                                                                                           |
| Lane                     | up-next                                                                                                           |
| Status                   | open                                                                                                              |
| Completed                | no                                                                                                                |
| Source path              | docs/method/backlog/up-next/PLATFORM_wesley-migration.md                                                          |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_wesley-migration.md#t-2-2-1-backfill-script-generation-for-schema-migrations |
| Direct blockers          | none                                                                                                              |
| Direct dependents        | M076 (Switch-over plan and contract validation)                                                                   |
| Referenced GitHub issues | none                                                                                                              |

<hr />

## M076 - Switch-over plan and contract validation

**User Story:** As a Wesley user, I want a switch-over plan that coordinates the migration sequence (backfill, schema swap, validation) so that I can execute migrations with confidence.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M075 (Backfill script generation for schema migrations)
DAG chain depth: downstream 1; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                     |
| ------------------------ | --------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                        |
| METHOD id                | M076                                                                                                      |
| Native id                | T-2-2-2                                                                                                   |
| Lane                     | up-next                                                                                                   |
| Status                   | blocked                                                                                                   |
| Completed                | no                                                                                                        |
| Source path              | docs/method/backlog/up-next/PLATFORM_wesley-migration.md                                                  |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_wesley-migration.md#t-2-2-2-switch-over-plan-and-contract-validation |
| Direct blockers          | M075 (Backfill script generation for schema migrations)                                                   |
| Direct dependents        | none                                                                                                      |
| Referenced GitHub issues | none                                                                                                      |

<hr />

## M077 - GraphQL operation parser for QIR

**User Story:** As a Wesley user, I want to write GraphQL operations against my schema and have Wesley parse them into a typed QIR AST so that I can generate SQL query plans automatically.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 2; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                               |
| ------------------------ | --------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                  |
| METHOD id                | M077                                                                                                |
| Native id                | T-2-1-1                                                                                             |
| Lane                     | up-next                                                                                             |
| Status                   | open                                                                                                |
| Completed                | no                                                                                                  |
| Source path              | docs/method/backlog/up-next/PLATFORM_wesley-qir-phase-c.md                                          |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_wesley-qir-phase-c.md#t-2-1-1-graphql-operation-parser-for-qir |
| Direct blockers          | none                                                                                                |
| Direct dependents        | M078 (SQL query plan generation from QIR)                                                           |
| Referenced GitHub issues | none                                                                                                |

<hr />

## M078 - SQL query plan generation from QIR

**User Story:** As a Wesley user, I want QIR ASTs compiled into SQL query plan ASTs so that I can generate efficient database queries from my GraphQL schema.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M077 (GraphQL operation parser for QIR)
DAG chain depth: downstream 1; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                 |
| ------------------------ | ----------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                    |
| METHOD id                | M078                                                                                                  |
| Native id                | T-2-1-2                                                                                               |
| Lane                     | up-next                                                                                               |
| Status                   | blocked                                                                                               |
| Completed                | no                                                                                                    |
| Source path              | docs/method/backlog/up-next/PLATFORM_wesley-qir-phase-c.md                                            |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_wesley-qir-phase-c.md#t-2-1-2-sql-query-plan-generation-from-qir |
| Direct blockers          | M077 (GraphQL operation parser for QIR)                                                               |
| Direct dependents        | none                                                                                                  |
| Referenced GitHub issues | none                                                                                                  |

<hr />

## M079 - Wesley To Echo Toy Contract Proof

Status: GREEN 4.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M008 (WESLEY Protocol Consumer Cutover), M036 (Registry Provider Wiring And Host Boundary Decision), M064 (Update echo-wesley-gen IR deserializer for v2 format)
DAG chain depth: downstream 7; upstream 5
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                                                            |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                               |
| METHOD id                | M079                                                                                                                                                             |
| Native id                | none                                                                                                                                                             |
| Lane                     | up-next                                                                                                                                                          |
| Status                   | blocked                                                                                                                                                          |
| Completed                | no                                                                                                                                                               |
| Source path              | docs/method/backlog/up-next/PLATFORM_wesley-to-echo-toy-contract-proof.md                                                                                        |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_wesley-to-echo-toy-contract-proof.md                                                                                        |
| Direct blockers          | M008 (WESLEY Protocol Consumer Cutover), M036 (Registry Provider Wiring And Host Boundary Decision), M064 (Update echo-wesley-gen IR deserializer for v2 format) |
| Direct dependents        | M045 (Contract-Aware Receipts And Readings), M053 (Authenticated Wesley Intent Admission Posture)                                                                |
| Referenced GitHub issues | none                                                                                                                                                             |

<hr />

## M080 - TypeScript type generation from Wesley IR

**User Story:** As a web developer, I want TypeScript interfaces generated from the Wesley schema so that my browser code is type-safe against the simulation's data model.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 3; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                  |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                     |
| METHOD id                | M080                                                                                                                   |
| Native id                | T-4-4-1                                                                                                                |
| Lane                     | up-next                                                                                                                |
| Status                   | open                                                                                                                   |
| Completed                | no                                                                                                                     |
| Source path              | docs/method/backlog/up-next/PLATFORM_wesley-type-pipeline-browser.md                                                   |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_wesley-type-pipeline-browser.md#t-4-4-1-typescript-type-generation-from-wesley-ir |
| Direct blockers          | none                                                                                                                   |
| Direct dependents        | M081 (Zod runtime validators from Wesley IR)                                                                           |
| Referenced GitHub issues | none                                                                                                                   |

<hr />

## M081 - Zod runtime validators from Wesley IR

**User Story:** As a web developer, I want Zod schemas generated from the Wesley schema so that I can validate data at the browser boundary before sending it to WASM.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M080 (TypeScript type generation from Wesley IR)
DAG chain depth: downstream 2; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                              |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                                                                 |
| METHOD id                | M081                                                                                                               |
| Native id                | T-4-4-2                                                                                                            |
| Lane                     | up-next                                                                                                            |
| Status                   | blocked                                                                                                            |
| Completed                | no                                                                                                                 |
| Source path              | docs/method/backlog/up-next/PLATFORM_wesley-type-pipeline-browser.md                                               |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_wesley-type-pipeline-browser.md#t-4-4-2-zod-runtime-validators-from-wesley-ir |
| Direct blockers          | M080 (TypeScript type generation from Wesley IR)                                                                   |
| Direct dependents        | M082 (CBOR serialization bridge (TS types to WASM Rust))                                                           |
| Referenced GitHub issues | none                                                                                                               |

<hr />

## M082 - CBOR serialization bridge (TS types to WASM Rust)

**User Story:** As a web developer, I want to encode validated TypeScript objects as CBOR and send them to the WASM engine so that intent payloads are correctly deserialized on the Rust side.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. It is still part of the dependency graph and has explicit unresolved blockers.

DAG blocked by: M073 (JS/WASM memory bridge and error protocol), M081 (Zod runtime validators from Wesley IR)
DAG chain depth: downstream 1; upstream 3
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                        |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                           |
| METHOD id                | M082                                                                                                                         |
| Native id                | T-4-4-3                                                                                                                      |
| Lane                     | up-next                                                                                                                      |
| Status                   | blocked                                                                                                                      |
| Completed                | no                                                                                                                           |
| Source path              | docs/method/backlog/up-next/PLATFORM_wesley-type-pipeline-browser.md                                                         |
| Anchor/link              | docs/method/backlog/up-next/PLATFORM_wesley-type-pipeline-browser.md#t-4-4-3-cbor-serialization-bridge-ts-types-to-wasm-rust |
| Direct blockers          | M073 (JS/WASM memory bridge and error protocol), M081 (Zod runtime validators from Wesley IR)                                |
| Direct dependents        | none                                                                                                                         |
| Referenced GitHub issues | none                                                                                                                         |

<hr />

## M083 - Information Architecture Consolidation

**User Story:** As a Wesley contributor, I want a consolidated documentation structure so that information is discoverable and not duplicated across scattered files.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 2; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                          |
| ------------------------ | ---------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                             |
| METHOD id                | M083                                                                                           |
| Native id                | T-10-10-1                                                                                      |
| Lane                     | inbox                                                                                          |
| Status                   | open                                                                                           |
| Completed                | no                                                                                             |
| Source path              | docs/method/backlog/inbox/DOCS_wesley-docs.md                                                  |
| Anchor/link              | docs/method/backlog/inbox/DOCS_wesley-docs.md#t-10-10-1-information-architecture-consolidation |
| Direct blockers          | none                                                                                           |
| Direct dependents        | M084 (Tutorial Series + API Reference)                                                         |
| Referenced GitHub issues | none                                                                                           |

<hr />

## M084 - Tutorial Series + API Reference

**User Story:** As a new Wesley user, I want tutorials and API reference so that I can learn the tool without reading source code.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M083 (Information Architecture Consolidation)
DAG chain depth: downstream 1; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                 |
| ------------------------ | ------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                    |
| METHOD id                | M084                                                                                  |
| Native id                | T-10-10-2                                                                             |
| Lane                     | inbox                                                                                 |
| Status                   | blocked                                                                               |
| Completed                | no                                                                                    |
| Source path              | docs/method/backlog/inbox/DOCS_wesley-docs.md                                         |
| Anchor/link              | docs/method/backlog/inbox/DOCS_wesley-docs.md#t-10-10-2-tutorial-series-api-reference |
| Direct blockers          | M083 (Information Architecture Consolidation)                                         |
| Direct dependents        | none                                                                                  |
| Referenced GitHub issues | none                                                                                  |

<hr />

## M085 - Rhai Sandbox Configuration (#173, part a)

**User Story:** As a simulation author, I want a Rhai sandbox that disallows non-deterministic operations so that scripts cannot accidentally break replay determinism.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 2; upstream 1
GH issue #: #173
GH issue createdAt: #173: 2026-01-01T19:24:43Z

| Field                    | Value                                                                                                  |
| ------------------------ | ------------------------------------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                                                     |
| METHOD id                | M085                                                                                                   |
| Native id                | T-10-6-1a                                                                                              |
| Lane                     | inbox                                                                                                  |
| Status                   | open                                                                                                   |
| Completed                | no                                                                                                     |
| Source path              | docs/method/backlog/inbox/KERNEL_deterministic-rhai.md                                                 |
| Anchor/link              | docs/method/backlog/inbox/KERNEL_deterministic-rhai.md#t-10-6-1a-rhai-sandbox-configuration-173-part-a |
| Direct blockers          | none                                                                                                   |
| Direct dependents        | M086 (ViewClaim / EffectClaim Receipts (#173, part b))                                                 |
| Referenced GitHub issues | #173                                                                                                   |

<hr />

## M086 - ViewClaim / EffectClaim Receipts (#173, part b)

**User Story:** As the Echo runtime, I want Rhai scripts to declare their state reads and writes via ViewClaim and EffectClaim receipts so that the scheduler can track data dependencies and verify determinism.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M085 (Rhai Sandbox Configuration (#173, part a))
DAG chain depth: downstream 1; upstream 2
GH issue #: #173
GH issue createdAt: #173: 2026-01-01T19:24:43Z

| Field                    | Value                                                                                                      |
| ------------------------ | ---------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                         |
| METHOD id                | M086                                                                                                       |
| Native id                | T-10-6-1b                                                                                                  |
| Lane                     | inbox                                                                                                      |
| Status                   | blocked                                                                                                    |
| Completed                | no                                                                                                         |
| Source path              | docs/method/backlog/inbox/KERNEL_deterministic-rhai.md                                                     |
| Anchor/link              | docs/method/backlog/inbox/KERNEL_deterministic-rhai.md#t-10-6-1b-viewclaim-effectclaim-receipts-173-part-b |
| Direct blockers          | M085 (Rhai Sandbox Configuration (#173, part a))                                                           |
| Direct dependents        | none                                                                                                       |
| Referenced GitHub issues | #173                                                                                                       |

<hr />

## M087 - First-class invariant documents

bijou has `docs/invariants/` with named invariants that legends and
design docs link to ("Layout Owns Interaction Geometry", "Commands
Change State, Effects Do Not"). Echo's invariants are scattered
across ADR prose and spec docs.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                  |
| ------------------------ | ------------------------------------------------------ |
| Source                   | METHOD task matrix                                     |
| METHOD id                | M087                                                   |
| Native id                | none                                                   |
| Lane                     | inbox                                                  |
| Status                   | open                                                   |
| Completed                | no                                                     |
| Source path              | docs/method/backlog/inbox/KERNEL_invariants-as-docs.md |
| Anchor/link              | docs/method/backlog/inbox/KERNEL_invariants-as-docs.md |
| Direct blockers          | none                                                   |
| Direct dependents        | none                                                   |
| Referenced GitHub issues | none                                                   |

<hr />

## M088 - Draft C ABI Spec (#85)

**User Story:** As a plugin author, I want a clear C ABI specification so that I can write plugins in any language that targets C calling conventions.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 5; upstream 1
GH issue #: #85
GH issue createdAt: #85: 2025-10-30T08:03:19Z

| Field                    | Value                                                                       |
| ------------------------ | --------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                          |
| METHOD id                | M088                                                                        |
| Native id                | T-10-1-1                                                                    |
| Lane                     | inbox                                                                       |
| Status                   | open                                                                        |
| Completed                | no                                                                          |
| Source path              | docs/method/backlog/inbox/KERNEL_plugin-abi.md                              |
| Anchor/link              | docs/method/backlog/inbox/KERNEL_plugin-abi.md#t-10-1-1-draft-c-abi-spec-85 |
| Direct blockers          | none                                                                        |
| Direct dependents        | M089 (C Header + Host Loader (#86))                                         |
| Referenced GitHub issues | #85                                                                         |

<hr />

## M089 - C Header + Host Loader (#86)

**User Story:** As the Echo runtime, I want to dynamically load plugin shared libraries via a C ABI so that plugins can be developed and deployed independently.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M088 (Draft C ABI Spec (#85))
DAG chain depth: downstream 4; upstream 2
GH issue #: #86
GH issue createdAt: #86: 2025-10-30T08:03:23Z

| Field                    | Value                                                                           |
| ------------------------ | ------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                              |
| METHOD id                | M089                                                                            |
| Native id                | T-10-1-2                                                                        |
| Lane                     | inbox                                                                           |
| Status                   | blocked                                                                         |
| Completed                | no                                                                              |
| Source path              | docs/method/backlog/inbox/KERNEL_plugin-abi.md                                  |
| Anchor/link              | docs/method/backlog/inbox/KERNEL_plugin-abi.md#t-10-1-2-c-header-host-loader-86 |
| Direct blockers          | M088 (Draft C ABI Spec (#85))                                                   |
| Direct dependents        | M090 (Version Negotiation (#87))                                                |
| Referenced GitHub issues | #86                                                                             |

<hr />

## M090 - Version Negotiation (#87)

**User Story:** As the Echo runtime, I want to negotiate ABI versions with plugins at load time so that incompatible plugins are rejected gracefully instead of causing undefined behavior.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M089 (C Header + Host Loader (#86))
DAG chain depth: downstream 3; upstream 3
GH issue #: #87
GH issue createdAt: #87: 2025-10-30T08:03:28Z

| Field                    | Value                                                                          |
| ------------------------ | ------------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                             |
| METHOD id                | M090                                                                           |
| Native id                | T-10-1-3                                                                       |
| Lane                     | inbox                                                                          |
| Status                   | blocked                                                                        |
| Completed                | no                                                                             |
| Source path              | docs/method/backlog/inbox/KERNEL_plugin-abi.md                                 |
| Anchor/link              | docs/method/backlog/inbox/KERNEL_plugin-abi.md#t-10-1-3-version-negotiation-87 |
| Direct blockers          | M089 (C Header + Host Loader (#86))                                            |
| Direct dependents        | M091 (Capability Tokens (#88))                                                 |
| Referenced GitHub issues | #87                                                                            |

<hr />

## M091 - Capability Tokens (#88)

**User Story:** As a host operator, I want to grant plugins fine-grained capability tokens so that untrusted plugins cannot access resources they were not explicitly authorized for.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M090 (Version Negotiation (#87))
DAG chain depth: downstream 2; upstream 4
GH issue #: #88
GH issue createdAt: #88: 2025-10-30T08:03:32Z

| Field                    | Value                                                                        |
| ------------------------ | ---------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                           |
| METHOD id                | M091                                                                         |
| Native id                | T-10-1-4                                                                     |
| Lane                     | inbox                                                                        |
| Status                   | blocked                                                                      |
| Completed                | no                                                                           |
| Source path              | docs/method/backlog/inbox/KERNEL_plugin-abi.md                               |
| Anchor/link              | docs/method/backlog/inbox/KERNEL_plugin-abi.md#t-10-1-4-capability-tokens-88 |
| Direct blockers          | M090 (Version Negotiation (#87))                                             |
| Direct dependents        | M092 (Example Plugin + Tests (#89))                                          |
| Referenced GitHub issues | #88                                                                          |

<hr />

## M092 - Example Plugin + Tests (#89)

**User Story:** As a plugin author, I want a reference plugin implementation with integration tests so that I have a concrete starting point for building my own plugins.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M091 (Capability Tokens (#88))
DAG chain depth: downstream 1; upstream 5
GH issue #: #89
GH issue createdAt: #89: 2025-10-30T08:03:36Z

| Field                    | Value                                                                           |
| ------------------------ | ------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                              |
| METHOD id                | M092                                                                            |
| Native id                | T-10-1-5                                                                        |
| Lane                     | inbox                                                                           |
| Status                   | blocked                                                                         |
| Completed                | no                                                                              |
| Source path              | docs/method/backlog/inbox/KERNEL_plugin-abi.md                                  |
| Anchor/link              | docs/method/backlog/inbox/KERNEL_plugin-abi.md#t-10-1-5-example-plugin-tests-89 |
| Direct blockers          | M091 (Capability Tokens (#88))                                                  |
| Direct dependents        | none                                                                            |
| Referenced GitHub issues | #89                                                                             |

<hr />

## M093 - Spec — Commit/Manifest Signing (#20)

**User Story:** As a deployment operator, I want a specification for signing commits and manifests so that I can verify the integrity and authorship of simulation artifacts.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 5; upstream 1
GH issue #: #20
GH issue createdAt: #20: 2025-10-30T07:54:57Z

| Field                    | Value                                                                                |
| ------------------------ | ------------------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                                   |
| METHOD id                | M093                                                                                 |
| Native id                | T-10-2-1                                                                             |
| Lane                     | inbox                                                                                |
| Status                   | open                                                                                 |
| Completed                | no                                                                                   |
| Source path              | docs/method/backlog/inbox/KERNEL_security.md                                         |
| Anchor/link              | docs/method/backlog/inbox/KERNEL_security.md#t-10-2-1-spec-commitmanifest-signing-20 |
| Direct blockers          | none                                                                                 |
| Direct dependents        | M106 (Key Management Doc (#35))                                                      |
| Referenced GitHub issues | #20                                                                                  |

<hr />

## M094 - Spec — Security Contexts (#21)

**User Story:** As a runtime integrator, I want clearly defined security contexts for FFI, WASM, and CLI boundaries so that I understand what each boundary permits and denies.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 2; upstream 1
GH issue #: #21
GH issue createdAt: #21: 2025-10-30T07:54:58Z

| Field                    | Value                                                                           |
| ------------------------ | ------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                              |
| METHOD id                | M094                                                                            |
| Native id                | T-10-2-2                                                                        |
| Lane                     | inbox                                                                           |
| Status                   | open                                                                            |
| Completed                | no                                                                              |
| Source path              | docs/method/backlog/inbox/KERNEL_security.md                                    |
| Anchor/link              | docs/method/backlog/inbox/KERNEL_security.md#t-10-2-2-spec-security-contexts-21 |
| Direct blockers          | none                                                                            |
| Direct dependents        | M095 (FFI Limits and Validation (#38))                                          |
| Referenced GitHub issues | #21                                                                             |

<hr />

## M095 - FFI Limits and Validation (#38)

**User Story:** As the Echo runtime, I want input validation at every FFI boundary so that malformed or malicious inputs cannot cause undefined behavior or panics.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M094 (Spec — Security Contexts (#21))
DAG chain depth: downstream 1; upstream 2
GH issue #: #38
GH issue createdAt: #38: 2025-10-30T07:58:39Z

| Field                    | Value                                                                              |
| ------------------------ | ---------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                 |
| METHOD id                | M095                                                                               |
| Native id                | T-10-2-3                                                                           |
| Lane                     | inbox                                                                              |
| Status                   | blocked                                                                            |
| Completed                | no                                                                                 |
| Source path              | docs/method/backlog/inbox/KERNEL_security.md                                       |
| Anchor/link              | docs/method/backlog/inbox/KERNEL_security.md#t-10-2-3-ffi-limits-and-validation-38 |
| Direct blockers          | M094 (Spec — Security Contexts (#21))                                              |
| Direct dependents        | none                                                                               |
| Referenced GitHub issues | #38                                                                                |

<hr />

## M096 - JS-ABI Packet Checksum v2 (#195)

**User Story:** As a JS-ABI consumer, I want packet checksums to use domain-separated hashing so that checksum collisions across different packet types are cryptographically impossible.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #195
GH issue createdAt: #195: 2026-01-02T16:56:24Z

| Field                    | Value                                                                               |
| ------------------------ | ----------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                  |
| METHOD id                | M096                                                                                |
| Native id                | T-10-2-4                                                                            |
| Lane                     | inbox                                                                               |
| Status                   | open                                                                                |
| Completed                | no                                                                                  |
| Source path              | docs/method/backlog/inbox/KERNEL_security.md                                        |
| Anchor/link              | docs/method/backlog/inbox/KERNEL_security.md#t-10-2-4-js-abi-packet-checksum-v2-195 |
| Direct blockers          | none                                                                                |
| Direct dependents        | none                                                                                |
| Referenced GitHub issues | #195                                                                                |

<hr />

## M097 - Spec — Provenance Payload v1 (#202)

**User Story:** As an auditor, I want a canonical envelope format for artifact provenance so that I can trace the full lineage and verify signatures of any simulation artifact.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #202
GH issue createdAt: #202: 2026-01-02T17:10:55Z

| Field                    | Value                                                                                |
| ------------------------ | ------------------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                                   |
| METHOD id                | M097                                                                                 |
| Native id                | T-10-2-5                                                                             |
| Lane                     | inbox                                                                                |
| Status                   | open                                                                                 |
| Completed                | no                                                                                   |
| Source path              | docs/method/backlog/inbox/KERNEL_security.md                                         |
| Anchor/link              | docs/method/backlog/inbox/KERNEL_security.md#t-10-2-5-spec-provenance-payload-v1-202 |
| Direct blockers          | none                                                                                 |
| Direct dependents        | none                                                                                 |
| Referenced GitHub issues | #202                                                                                 |

<hr />

## M098 - ABI nested evidence strictness

Status: inbox.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #322
GH issue createdAt: #322: unknown

| Field                    | Value                                                                |
| ------------------------ | -------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                   |
| METHOD id                | M098                                                                 |
| Native id                | none                                                                 |
| Lane                     | inbox                                                                |
| Status                   | open                                                                 |
| Completed                | no                                                                   |
| Source path              | docs/method/backlog/inbox/PLATFORM_abi-nested-evidence-strictness.md |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_abi-nested-evidence-strictness.md |
| Direct blockers          | none                                                                 |
| Direct dependents        | none                                                                 |
| Referenced GitHub issues | #322                                                                 |

<hr />

## M099 - Draft Hot-Reload Spec (#75)

**User Story:** As a simulation developer, I want a hot-reload specification so that the reload behavior is well-defined and predictable (what reloads, what resets, what persists).

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 3; upstream 1
GH issue #: #75
GH issue createdAt: #75: 2025-10-30T08:02:29Z

| Field                    | Value                                                                                     |
| ------------------------ | ----------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                        |
| METHOD id                | M099                                                                                      |
| Native id                | T-10-4-1                                                                                  |
| Lane                     | inbox                                                                                     |
| Status                   | open                                                                                      |
| Completed                | no                                                                                        |
| Source path              | docs/method/backlog/inbox/PLATFORM_editor-hot-reload.md                                   |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_editor-hot-reload.md#t-10-4-1-draft-hot-reload-spec-75 |
| Direct blockers          | none                                                                                      |
| Direct dependents        | M100 (File Watcher / Debounce (#76))                                                      |
| Referenced GitHub issues | #75                                                                                       |

<hr />

## M100 - File Watcher / Debounce (#76)

**User Story:** As a simulation developer, I want a file watcher with debounce logic so that rapid saves don't trigger redundant reloads.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M099 (Draft Hot-Reload Spec (#75))
DAG chain depth: downstream 2; upstream 2
GH issue #: #76
GH issue createdAt: #76: 2025-10-30T08:02:33Z

| Field                    | Value                                                                                     |
| ------------------------ | ----------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                        |
| METHOD id                | M100                                                                                      |
| Native id                | T-10-4-2                                                                                  |
| Lane                     | inbox                                                                                     |
| Status                   | blocked                                                                                   |
| Completed                | no                                                                                        |
| Source path              | docs/method/backlog/inbox/PLATFORM_editor-hot-reload.md                                   |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_editor-hot-reload.md#t-10-4-2-file-watcher-debounce-76 |
| Direct blockers          | M099 (Draft Hot-Reload Spec (#75))                                                        |
| Direct dependents        | M101 (Hot-Reload Implementation (#24))                                                    |
| Referenced GitHub issues | #76                                                                                       |

<hr />

## M101 - Hot-Reload Implementation (#24)

**User Story:** As a simulation developer, I want the editor to automatically reload when I save a file so that I see changes reflected immediately without manual restart.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M100 (File Watcher / Debounce (#76))
DAG chain depth: downstream 1; upstream 3
GH issue #: #24
GH issue createdAt: #24: 2025-10-30T07:55:00Z

| Field                    | Value                                                                                         |
| ------------------------ | --------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                            |
| METHOD id                | M101                                                                                          |
| Native id                | T-10-4-3                                                                                      |
| Lane                     | inbox                                                                                         |
| Status                   | blocked                                                                                       |
| Completed                | no                                                                                            |
| Source path              | docs/method/backlog/inbox/PLATFORM_editor-hot-reload.md                                       |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_editor-hot-reload.md#t-10-4-3-hot-reload-implementation-24 |
| Direct blockers          | M100 (File Watcher / Debounce (#76))                                                          |
| Direct dependents        | none                                                                                          |
| Referenced GitHub issues | #24                                                                                           |

<hr />

## M102 - git-mind NEXUS

> **Milestone:** Backlog | **Priority:** Unscheduled
> **Formerly:** MS-3 (demoted — independent of Echo critical path)

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                |
| ------------------------ | ---------------------------------------------------- |
| Source                   | METHOD task matrix                                   |
| METHOD id                | M102                                                 |
| Native id                | none                                                 |
| Lane                     | inbox                                                |
| Status                   | open                                                 |
| Completed                | no                                                   |
| Source path              | docs/method/backlog/inbox/PLATFORM_git-mind-nexus.md |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_git-mind-nexus.md |
| Direct blockers          | none                                                 |
| Direct dependents        | none                                                 |
| Referenced GitHub issues | none                                                 |

<hr />

## M103 - Importer Umbrella Audit + Close (#25)

**User Story:** As a project maintainer, I want to audit the importer umbrella issue so that it can be closed if all work is complete, or remaining gaps are identified.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #25, #80, #81, #82, #83, #84
GH issue createdAt: #25: 2025-10-30T07:55:01Z, #80: 2025-10-30T08:02:49Z, #81: 2025-10-30T08:02:55Z, #82: 2025-10-30T08:03:01Z, #83: 2025-10-30T08:03:07Z, #84: 2025-10-30T08:03:13Z

| Field                    | Value                                                                                    |
| ------------------------ | ---------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                       |
| METHOD id                | M103                                                                                     |
| Native id                | T-10-5-1                                                                                 |
| Lane                     | inbox                                                                                    |
| Status                   | open                                                                                     |
| Completed                | no                                                                                       |
| Source path              | docs/method/backlog/inbox/PLATFORM_importer.md                                           |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_importer.md#t-10-5-1-importer-umbrella-audit-close-25 |
| Direct blockers          | none                                                                                     |
| Direct dependents        | none                                                                                     |
| Referenced GitHub issues | #25, #80, #81, #82, #83, #84                                                             |

<hr />

## M104 - Legend progress in method status

Currently `method status` only counts backlog items per legend. It
would be useful to also count completed cycles per legend (from retro
dirs) and show a progress ratio — e.g., "KERNEL: 3 done / 19 backlog."

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                               |
| ------------------------ | ------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                  |
| METHOD id                | M104                                                                |
| Native id                | none                                                                |
| Lane                     | inbox                                                               |
| Status                   | open                                                                |
| Completed                | no                                                                  |
| Source path              | docs/method/backlog/inbox/PLATFORM_method-status-legend-progress.md |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_method-status-legend-progress.md |
| Direct blockers          | none                                                                |
| Direct dependents        | none                                                                |
| Referenced GitHub issues | none                                                                |

<hr />

## M105 - Reconcile Relocated Wesley Echo Schemas

Status: inbox.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                         |
| ------------------------ | ----------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                            |
| METHOD id                | M105                                                                          |
| Native id                | none                                                                          |
| Lane                     | inbox                                                                         |
| Status                   | open                                                                          |
| Completed                | no                                                                            |
| Source path              | docs/method/backlog/inbox/PLATFORM_reconcile-relocated-wesley-echo-schemas.md |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_reconcile-relocated-wesley-echo-schemas.md |
| Direct blockers          | none                                                                          |
| Direct dependents        | none                                                                          |
| Referenced GitHub issues | none                                                                          |

<hr />

## M106 - Key Management Doc (#35)

**User Story:** As a release engineer, I want key management documentation so that I know how to generate, store, rotate, and revoke signing keys.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M093 (Spec — Commit/Manifest Signing (#20))
DAG chain depth: downstream 4; upstream 2
GH issue #: #35
GH issue createdAt: #35: 2025-10-30T07:58:20Z

| Field                    | Value                                                                                 |
| ------------------------ | ------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                    |
| METHOD id                | M106                                                                                  |
| Native id                | T-10-3-1                                                                              |
| Lane                     | inbox                                                                                 |
| Status                   | blocked                                                                               |
| Completed                | no                                                                                    |
| Source path              | docs/method/backlog/inbox/PLATFORM_signing-pipeline.md                                |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_signing-pipeline.md#t-10-3-1-key-management-doc-35 |
| Direct blockers          | M093 (Spec — Commit/Manifest Signing (#20))                                           |
| Direct dependents        | M107 (CI — Sign Release Artifacts (Dry Run) (#33))                                    |
| Referenced GitHub issues | #35                                                                                   |

<hr />

## M107 - CI — Sign Release Artifacts (Dry Run) (#33)

**User Story:** As a release engineer, I want CI to sign release artifacts automatically so that every release includes verifiable signatures without manual intervention.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M106 (Key Management Doc (#35))
DAG chain depth: downstream 3; upstream 3
GH issue #: #33
GH issue createdAt: #33: 2025-10-30T07:58:06Z

| Field                    | Value                                                                                                |
| ------------------------ | ---------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                   |
| METHOD id                | M107                                                                                                 |
| Native id                | T-10-3-2                                                                                             |
| Lane                     | inbox                                                                                                |
| Status                   | blocked                                                                                              |
| Completed                | no                                                                                                   |
| Source path              | docs/method/backlog/inbox/PLATFORM_signing-pipeline.md                                               |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_signing-pipeline.md#t-10-3-2-ci-sign-release-artifacts-dry-run-33 |
| Direct blockers          | M106 (Key Management Doc (#35))                                                                      |
| Direct dependents        | M108 (CLI Verify Path (#34))                                                                         |
| Referenced GitHub issues | #33                                                                                                  |

<hr />

## M108 - CLI Verify Path (#34)

**User Story:** As a user, I want a CLI command to verify artifact signatures so that I can confirm artifacts are authentic before using them.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M107 (CI — Sign Release Artifacts (Dry Run) (#33))
DAG chain depth: downstream 2; upstream 4
GH issue #: #34
GH issue createdAt: #34: 2025-10-30T07:58:13Z

| Field                    | Value                                                                              |
| ------------------------ | ---------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                 |
| METHOD id                | M108                                                                               |
| Native id                | T-10-3-3                                                                           |
| Lane                     | inbox                                                                              |
| Status                   | blocked                                                                            |
| Completed                | no                                                                                 |
| Source path              | docs/method/backlog/inbox/PLATFORM_signing-pipeline.md                             |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_signing-pipeline.md#t-10-3-3-cli-verify-path-34 |
| Direct blockers          | M107 (CI — Sign Release Artifacts (Dry Run) (#33))                                 |
| Direct dependents        | M109 (CI — Verify Signatures (#36))                                                |
| Referenced GitHub issues | #34                                                                                |

<hr />

## M109 - CI — Verify Signatures (#36)

**User Story:** As a release engineer, I want CI to verify signatures of published artifacts so that any signing regression is caught automatically.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M108 (CLI Verify Path (#34))
DAG chain depth: downstream 1; upstream 5
GH issue #: #36
GH issue createdAt: #36: 2025-10-30T07:58:28Z

| Field                    | Value                                                                                   |
| ------------------------ | --------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                      |
| METHOD id                | M109                                                                                    |
| Native id                | T-10-3-4                                                                                |
| Lane                     | inbox                                                                                   |
| Status                   | blocked                                                                                 |
| Completed                | no                                                                                      |
| Source path              | docs/method/backlog/inbox/PLATFORM_signing-pipeline.md                                  |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_signing-pipeline.md#t-10-3-4-ci-verify-signatures-36 |
| Direct blockers          | M108 (CLI Verify Path (#34))                                                            |
| Direct dependents        | none                                                                                    |
| Referenced GitHub issues | #36                                                                                     |

<hr />

## M110 - Docs / Logging Improvements (#79)

**User Story:** As a contributor, I want improved documentation and structured logging so that onboarding is faster and runtime behavior is observable.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #79
GH issue createdAt: #79: 2025-10-30T08:02:45Z

| Field                    | Value                                                                                    |
| ------------------------ | ---------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                       |
| METHOD id                | M110                                                                                     |
| Native id                | T-10-8-1                                                                                 |
| Lane                     | inbox                                                                                    |
| Status                   | open                                                                                     |
| Completed                | no                                                                                       |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                       |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-1-docs-logging-improvements-79 |
| Direct blockers          | none                                                                                     |
| Direct dependents        | none                                                                                     |
| Referenced GitHub issues | #79                                                                                      |

<hr />

## M111 - Naming Consistency Audit (#207)

**User Story:** As a user, I want consistent naming across Echo, WARP, Wesley, and Engram so that there is no confusion about product names in code, docs, and CLI output.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #207
GH issue createdAt: #207: 2026-01-02T17:19:18Z

| Field                    | Value                                                                                    |
| ------------------------ | ---------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                       |
| METHOD id                | M111                                                                                     |
| Native id                | T-10-8-2                                                                                 |
| Lane                     | inbox                                                                                    |
| Status                   | open                                                                                     |
| Completed                | no                                                                                       |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                       |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-2-naming-consistency-audit-207 |
| Direct blockers          | none                                                                                     |
| Direct dependents        | none                                                                                     |
| Referenced GitHub issues | #207                                                                                     |

<hr />

## M112 - Reliving Debugger UX Design (#239)

**User Story:** As a simulation developer, I want a UX design for the reliving debugger so that the Constraint Lens and Provenance Heatmap features are well-specified before implementation begins.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #239
GH issue createdAt: #239: 2026-01-02T22:43:10Z

| Field                    | Value                                                                                       |
| ------------------------ | ------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                          |
| METHOD id                | M112                                                                                        |
| Native id                | T-10-8-3                                                                                    |
| Lane                     | inbox                                                                                       |
| Status                   | open                                                                                        |
| Completed                | no                                                                                          |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                          |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-3-reliving-debugger-ux-design-239 |
| Direct blockers          | none                                                                                        |
| Direct dependents        | none                                                                                        |
| Referenced GitHub issues | #239                                                                                        |

<hr />

## M113 - Local Rustdoc Warning Gate

**User Story:** As a contributor, I want the Rustdoc warnings gate available locally so that private intra-doc link failures and other doc regressions are caught before CI.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                  |
| ------------------------ | -------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                     |
| METHOD id                | M113                                                                                   |
| Native id                | T-10-8-4                                                                               |
| Lane                     | inbox                                                                                  |
| Status                   | open                                                                                   |
| Completed                | no                                                                                     |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                     |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-4-local-rustdoc-warning-gate |
| Direct blockers          | none                                                                                   |
| Direct dependents        | none                                                                                   |
| Referenced GitHub issues | none                                                                                   |

<hr />

## M114 - Deterministic Test Engine Helper

**User Story:** As a test author, I want one shared deterministic engine-builder helper so that golden/property tests do not silently inherit ambient worker-count entropy.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                        |
| ------------------------ | -------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                           |
| METHOD id                | M114                                                                                         |
| Native id                | T-10-8-5                                                                                     |
| Lane                     | inbox                                                                                        |
| Status                   | open                                                                                         |
| Completed                | no                                                                                           |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                           |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-5-deterministic-test-engine-helper |
| Direct blockers          | none                                                                                         |
| Direct dependents        | none                                                                                         |
| Referenced GitHub issues | none                                                                                         |

<hr />

## M115 - Current-Head PR Review / Merge Summary Tool

**User Story:** As a reviewer, I want a lightweight current-head PR summary
so that unresolved threads, failing checks, historical noise, and
merge-readiness state are visible before push/merge decisions.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                 |
| ------------------------ | ----------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                    |
| METHOD id                | M115                                                                                                  |
| Native id                | T-10-8-6                                                                                              |
| Lane                     | inbox                                                                                                 |
| Status                   | open                                                                                                  |
| Completed                | no                                                                                                    |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                                    |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-6-current-head-pr-review-merge-summary-tool |
| Direct blockers          | none                                                                                                  |
| Direct dependents        | none                                                                                                  |
| Referenced GitHub issues | none                                                                                                  |

<hr />

## M116 - CI Trigger Rationalization

**User Story:** As a contributor, I want less duplicated CI noise so that I can interpret check state quickly without sifting through redundant push/pull_request runs.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                  |
| ------------------------ | -------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                     |
| METHOD id                | M116                                                                                   |
| Native id                | T-10-8-7                                                                               |
| Lane                     | inbox                                                                                  |
| Status                   | open                                                                                   |
| Completed                | no                                                                                     |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                     |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-7-ci-trigger-rationalization |
| Direct blockers          | none                                                                                   |
| Direct dependents        | none                                                                                   |
| Referenced GitHub issues | none                                                                                   |

<hr />

## M117 - Background Cargo Lock Isolation

**User Story:** As a contributor, I want background Cargo activity isolated from manual verification so that ad hoc review fixes and hook-driven checks do not waste time waiting on unrelated workspace builds.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                       |
| ------------------------ | ------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                          |
| METHOD id                | M117                                                                                        |
| Native id                | T-10-8-8                                                                                    |
| Lane                     | inbox                                                                                       |
| Status                   | open                                                                                        |
| Completed                | no                                                                                          |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                          |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-8-background-cargo-lock-isolation |
| Direct blockers          | none                                                                                        |
| Direct dependents        | none                                                                                        |
| Referenced GitHub issues | none                                                                                        |

<hr />

## M118 - Small-Commit Pre-Commit Latency Reduction

**User Story:** As a contributor, I want tiny review-fix commits to complete quickly so that one-line test/doc/tooling follow-ups do not trigger disproportionately expensive staged verification.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                 |
| ------------------------ | ----------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                    |
| METHOD id                | M118                                                                                                  |
| Native id                | T-10-8-9                                                                                              |
| Lane                     | inbox                                                                                                 |
| Status                   | open                                                                                                  |
| Completed                | no                                                                                                    |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                                    |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-9-small-commit-pre-commit-latency-reduction |
| Direct blockers          | none                                                                                                  |
| Direct dependents        | none                                                                                                  |
| Referenced GitHub issues | none                                                                                                  |

<hr />

## M119 - Feature-Gate Contract Verification

**User Story:** As a contributor, I want explicit feature-contract checks for
no-std / alloc-only crates so that feature-gating regressions are caught before
PR review or CI.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                           |
| ------------------------ | ----------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                              |
| METHOD id                | M119                                                                                            |
| Native id                | T-10-8-10                                                                                       |
| Lane                     | inbox                                                                                           |
| Status                   | open                                                                                            |
| Completed                | no                                                                                              |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                              |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-10-feature-gate-contract-verification |
| Direct blockers          | none                                                                                            |
| Direct dependents        | none                                                                                            |
| Referenced GitHub issues | none                                                                                            |

<hr />

## M120 - PR Review Thread Reply / Resolution Helper

**User Story:** As a reviewer, I want a safe helper for replying to and
resolving PR review threads so that GitHub thread state does not lag behind the
branch state after review-fix pushes.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                 |
| ------------------------ | ----------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                    |
| METHOD id                | M120                                                                                                  |
| Native id                | T-10-8-11                                                                                             |
| Lane                     | inbox                                                                                                 |
| Status                   | open                                                                                                  |
| Completed                | no                                                                                                    |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                                    |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-11-pr-review-thread-reply-resolution-helper |
| Direct blockers          | none                                                                                                  |
| Direct dependents        | none                                                                                                  |
| Referenced GitHub issues | none                                                                                                  |

<hr />

## M121 - Shell Script Style / Format Lane

**User Story:** As a maintainer, I want a dedicated shell-style lane for
maintained hook scripts so that shell regressions are caught before PR review or
merge.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                       |
| ------------------------ | ------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                          |
| METHOD id                | M121                                                                                        |
| Native id                | T-10-8-12                                                                                   |
| Lane                     | inbox                                                                                       |
| Status                   | open                                                                                        |
| Completed                | no                                                                                          |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                          |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-12-shell-script-style-format-lane |
| Direct blockers          | none                                                                                        |
| Direct dependents        | none                                                                                        |
| Referenced GitHub issues | none                                                                                        |

<hr />

## M122 - Review-Fix Fast Path for Staged Verification

**User Story:** As a contributor, I want small review-fix commits to verify
quickly so that post-review iteration does not spend minutes rerunning unrelated
lanes.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                     |
| ------------------------ | --------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                        |
| METHOD id                | M122                                                                                                      |
| Native id                | T-10-8-13                                                                                                 |
| Lane                     | inbox                                                                                                     |
| Status                   | open                                                                                                      |
| Completed                | no                                                                                                        |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                                        |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-13-review-fix-fast-path-for-staged-verification |
| Direct blockers          | none                                                                                                      |
| Direct dependents        | none                                                                                                      |
| Referenced GitHub issues | none                                                                                                      |

<hr />

## M123 - Pre-PR Preflight Gate

**User Story:** As a contributor, I want one high-signal preflight command
before opening a PR so that obvious CI/review churn is caught locally first.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 2; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                              |
| ------------------------ | ---------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                 |
| METHOD id                | M123                                                                               |
| Native id                | T-10-8-14                                                                          |
| Lane                     | inbox                                                                              |
| Status                   | open                                                                               |
| Completed                | no                                                                                 |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                 |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-14-pre-pr-preflight-gate |
| Direct blockers          | none                                                                               |
| Direct dependents        | M125 (Pre-PR Checklist and Boundary-Change Policy)                                 |
| Referenced GitHub issues | none                                                                               |

<hr />

## M124 - Self-Review Command

**User Story:** As an author, I want a harsh local self-review against the
merge target before opening a PR so that contract drift, missing negative tests,
and stale docs are found before Rabbit or humans spend cycles on them.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 2; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                            |
| ------------------------ | -------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                               |
| METHOD id                | M124                                                                             |
| Native id                | T-10-8-15                                                                        |
| Lane                     | inbox                                                                            |
| Status                   | open                                                                             |
| Completed                | no                                                                               |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                               |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-15-self-review-command |
| Direct blockers          | none                                                                             |
| Direct dependents        | M125 (Pre-PR Checklist and Boundary-Change Policy)                               |
| Referenced GitHub issues | none                                                                             |

<hr />

## M125 - Pre-PR Checklist and Boundary-Change Policy

**User Story:** As an author or reviewer, I want a written pre-PR checklist for
boundary and tooling work so that the repo has a shared definition of “ready to
open a PR.”

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M123 (Pre-PR Preflight Gate), M124 (Self-Review Command)
DAG chain depth: downstream 1; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                    |
| ------------------------ | -------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                       |
| METHOD id                | M125                                                                                                     |
| Native id                | T-10-8-16                                                                                                |
| Lane                     | inbox                                                                                                    |
| Status                   | blocked                                                                                                  |
| Completed                | no                                                                                                       |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                                       |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-16-pre-pr-checklist-and-boundary-change-policy |
| Direct blockers          | M123 (Pre-PR Preflight Gate), M124 (Self-Review Command)                                                 |
| Direct dependents        | none                                                                                                     |
| Referenced GitHub issues | none                                                                                                     |

<hr />

## M126 - Docs Validation Beyond Markdown

**User Story:** As a contributor, I want docs validation to cover the real docs
surface, not just Markdown, so that broken static-HTML links and other live-doc
regressions are caught before PR review.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                        |
| ------------------------ | -------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                           |
| METHOD id                | M126                                                                                         |
| Native id                | T-10-8-17                                                                                    |
| Lane                     | inbox                                                                                        |
| Status                   | open                                                                                         |
| Completed                | no                                                                                           |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                           |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-17-docs-validation-beyond-markdown |
| Direct blockers          | none                                                                                         |
| Direct dependents        | none                                                                                         |
| Referenced GitHub issues | none                                                                                         |

<hr />

## M127 - Implementation-Backed Docs Claims Policy

**User Story:** As a maintainer, I want contributor guidance and lightweight
checks around strong claims like `bit-exact`, `canonical`, and `deterministic`
so that docs do not overstate what the code actually guarantees.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                 |
| ------------------------ | ----------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                    |
| METHOD id                | M127                                                                                                  |
| Native id                | T-10-8-18                                                                                             |
| Lane                     | inbox                                                                                                 |
| Status                   | open                                                                                                  |
| Completed                | no                                                                                                    |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                                    |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-18-implementation-backed-docs-claims-policy |
| Direct blockers          | none                                                                                                  |
| Direct dependents        | none                                                                                                  |
| Referenced GitHub issues | none                                                                                                  |

<hr />

## M128 - Remove Committed Generated DAG Artifacts

**User Story:** As a maintainer, I want generated DAG outputs out of the main
docs tree so that the repo keeps source-of-truth inputs, not churn-heavy baked
artifacts.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                 |
| ------------------------ | ----------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                    |
| METHOD id                | M128                                                                                                  |
| Native id                | T-10-8-19                                                                                             |
| Lane                     | inbox                                                                                                 |
| Status                   | open                                                                                                  |
| Completed                | no                                                                                                    |
| Source path              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md                                                    |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_tooling-misc.md#t-10-8-19-remove-committed-generated-dag-artifacts |
| Direct blockers          | none                                                                                                  |
| Direct dependents        | none                                                                                                  |
| Referenced GitHub issues | none                                                                                                  |

<hr />

## M129 - Fuzzing the Port

**User Story:** As a maintainer, I want to fuzz the ScenePort boundary so that I can guarantee the MockAdapter (and future production adapters) never panic on malformed CBOR or invalid operation sequences.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                         |
| ------------------------ | ----------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                            |
| METHOD id                | M129                                                                          |
| Native id                | T-10-9-1                                                                      |
| Lane                     | inbox                                                                         |
| Status                   | open                                                                          |
| Completed                | no                                                                            |
| Source path              | docs/method/backlog/inbox/PLATFORM_ttd-hardening.md                           |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_ttd-hardening.md#t-10-9-1-fuzzing-the-port |
| Direct blockers          | none                                                                          |
| Direct dependents        | none                                                                          |
| Referenced GitHub issues | none                                                                          |

<hr />

## M130 - SIMD Canonicalization

**User Story:** As a performance-conscious developer, I want `canonicalize_position` to use SIMD intrinsics so that scene graph updates remain cheap even as the number of entities grows by orders of magnitude.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                              |
| ------------------------ | ---------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                 |
| METHOD id                | M130                                                                               |
| Native id                | T-10-9-2                                                                           |
| Lane                     | inbox                                                                              |
| Status                   | open                                                                               |
| Completed                | no                                                                                 |
| Source path              | docs/method/backlog/inbox/PLATFORM_ttd-hardening.md                                |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_ttd-hardening.md#t-10-9-2-simd-canonicalization |
| Direct blockers          | none                                                                               |
| Direct dependents        | none                                                                               |
| Referenced GitHub issues | none                                                                               |

<hr />

## M131 - Causal Visualizer

**User Story:** As a simulation developer debugging complex forks, I want a tool that generates Graphviz DOT files from the `MockAdapter` state so that I can visually inspect the scene graph and causal provenance.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                          |
| ------------------------ | ------------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                             |
| METHOD id                | M131                                                                           |
| Native id                | T-10-9-3                                                                       |
| Lane                     | inbox                                                                          |
| Status                   | open                                                                           |
| Completed                | no                                                                             |
| Source path              | docs/method/backlog/inbox/PLATFORM_ttd-hardening.md                            |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_ttd-hardening.md#t-10-9-3-causal-visualizer |
| Direct blockers          | none                                                                           |
| Direct dependents        | none                                                                           |
| Referenced GitHub issues | none                                                                           |

<hr />

## M132 - Hashable View Artifacts (#174)

**User Story:** As the Wesley pipeline, I want canonical AST hashing for view artifacts so that any change to a schema produces a detectable and verifiable hash change.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 3; upstream 1
GH issue #: #174
GH issue createdAt: #174: 2026-01-01T19:24:45Z

| Field                    | Value                                                                                              |
| ------------------------ | -------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                 |
| METHOD id                | M132                                                                                               |
| Native id                | T-10-7-1                                                                                           |
| Lane                     | inbox                                                                                              |
| Status                   | open                                                                                               |
| Completed                | no                                                                                                 |
| Source path              | docs/method/backlog/inbox/PLATFORM_wesley-boundary-grammar.md                                      |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_wesley-boundary-grammar.md#t-10-7-1-hashable-view-artifacts-174 |
| Direct blockers          | none                                                                                               |
| Direct dependents        | M133 (Schema Hash Chain Pinning (#193))                                                            |
| Referenced GitHub issues | #174                                                                                               |

<hr />

## M133 - Schema Hash Chain Pinning (#193)

**User Story:** As an operator replaying a simulation, I want schema hashes pinned in receipts so that I can verify the exact schema version used at each tick.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M132 (Hashable View Artifacts (#174))
DAG chain depth: downstream 2; upstream 2
GH issue #: #193
GH issue createdAt: #193: 2026-01-02T16:56:20Z

| Field                    | Value                                                                                                |
| ------------------------ | ---------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                   |
| METHOD id                | M133                                                                                                 |
| Native id                | T-10-7-2                                                                                             |
| Lane                     | inbox                                                                                                |
| Status                   | blocked                                                                                              |
| Completed                | no                                                                                                   |
| Source path              | docs/method/backlog/inbox/PLATFORM_wesley-boundary-grammar.md                                        |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_wesley-boundary-grammar.md#t-10-7-2-schema-hash-chain-pinning-193 |
| Direct blockers          | M132 (Hashable View Artifacts (#174))                                                                |
| Direct dependents        | M135 (Provenance as Query Semantics (#198))                                                          |
| Referenced GitHub issues | #193                                                                                                 |

<hr />

## M134 - SchemaDelta Vocabulary (#194)

**User Story:** As a schema author, I want a read-only schema delta vocabulary so that I can preview what would change before applying a schema migration.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #194
GH issue createdAt: #194: 2026-01-02T16:56:22Z

| Field                    | Value                                                                                             |
| ------------------------ | ------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                |
| METHOD id                | M134                                                                                              |
| Native id                | T-10-7-3                                                                                          |
| Lane                     | inbox                                                                                             |
| Status                   | open                                                                                              |
| Completed                | no                                                                                                |
| Source path              | docs/method/backlog/inbox/PLATFORM_wesley-boundary-grammar.md                                     |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_wesley-boundary-grammar.md#t-10-7-3-schemadelta-vocabulary-194 |
| Direct blockers          | none                                                                                              |
| Direct dependents        | none                                                                                              |
| Referenced GitHub issues | #194                                                                                              |

<hr />

## M135 - Provenance as Query Semantics (#198)

**User Story:** As a simulation analyst, I want provenance tracking built into query semantics so that every query result carries proof of its derivation.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is unprocessed input, so deletion should require a positive decision that it is no longer useful.

DAG blocked by: M133 (Schema Hash Chain Pinning (#193))
DAG chain depth: downstream 1; upstream 3
GH issue #: #198
GH issue createdAt: #198: 2026-01-02T17:08:03Z

| Field                    | Value                                                                                                    |
| ------------------------ | -------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                       |
| METHOD id                | M135                                                                                                     |
| Native id                | T-10-7-4                                                                                                 |
| Lane                     | inbox                                                                                                    |
| Status                   | blocked                                                                                                  |
| Completed                | no                                                                                                       |
| Source path              | docs/method/backlog/inbox/PLATFORM_wesley-boundary-grammar.md                                            |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_wesley-boundary-grammar.md#t-10-7-4-provenance-as-query-semantics-198 |
| Direct blockers          | M133 (Schema Hash Chain Pinning (#193))                                                                  |
| Direct dependents        | none                                                                                                     |
| Referenced GitHub issues | #198                                                                                                     |

<hr />

## M136 - Shadow REALM Investigation

**User Story:** As the Wesley runtime, I want a restricted execution and linear memory (REALM) sandbox for generated code so that user-defined validators run safely in resource-constrained environments.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                   |
| ------------------------ | --------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                      |
| METHOD id                | M136                                                                                    |
| Native id                | T-10-9-1                                                                                |
| Lane                     | inbox                                                                                   |
| Status                   | open                                                                                    |
| Completed                | no                                                                                      |
| Source path              | docs/method/backlog/inbox/PLATFORM_wesley-future.md                                     |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_wesley-future.md#t-10-9-1-shadow-realm-investigation |
| Direct blockers          | none                                                                                    |
| Direct dependents        | none                                                                                    |
| Referenced GitHub issues | none                                                                                    |

<hr />

## M137 - Multi-Language Generator Survey

**User Story:** As a Wesley user, I want code generation targets beyond TypeScript and Rust so that I can use Wesley schemas in Go, Python, and Swift projects.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                        |
| ------------------------ | -------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                           |
| METHOD id                | M137                                                                                         |
| Native id                | T-10-9-2                                                                                     |
| Lane                     | inbox                                                                                        |
| Status                   | open                                                                                         |
| Completed                | no                                                                                           |
| Source path              | docs/method/backlog/inbox/PLATFORM_wesley-future.md                                          |
| Anchor/link              | docs/method/backlog/inbox/PLATFORM_wesley-future.md#t-10-9-2-multi-language-generator-survey |
| Direct blockers          | none                                                                                         |
| Direct dependents        | none                                                                                         |
| Referenced GitHub issues | none                                                                                         |

<hr />

## M138 - Enforce Echo design vocabulary

Status: active cool idea. Echo has `docs/guide/course/glossary.md` and docs
linting, but no glossary/terminology enforcement gate. Keep this as a bounded
docs-tooling task, not as a new vocabulary source of truth.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                       |
| ------------------------ | ----------------------------------------------------------- |
| Source                   | METHOD task matrix                                          |
| METHOD id                | M138                                                        |
| Native id                | none                                                        |
| Lane                     | cool-ideas                                                  |
| Status                   | open                                                        |
| Completed                | no                                                          |
| Source path              | docs/method/backlog/cool-ideas/DOCS_glossary-enforcement.md |
| Anchor/link              | docs/method/backlog/cool-ideas/DOCS_glossary-enforcement.md |
| Direct blockers          | none                                                        |
| Direct dependents        | none                                                        |
| Referenced GitHub issues | none                                                        |

<hr />

## M139 - Course Material

> **Milestone:** Splash Guy | **Priority:** P2
>
> Status: active cool idea, blocked by the open Splash Guy implementation and
> visualization tasks. `docs/guide/course/` already has the course shell plus
> modules `00-orientation` and `01-lockstep`; this card now tracks the remaining
> networking-first course modules, not course creation from zero.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #226
GH issue createdAt: #226: 2026-01-02T22:11:50Z

| Field                    | Value                                                             |
| ------------------------ | ----------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                |
| METHOD id                | M139                                                              |
| Native id                | none                                                              |
| Lane                     | cool-ideas                                                        |
| Status                   | open                                                              |
| Completed                | no                                                                |
| Source path              | docs/method/backlog/cool-ideas/DOCS_splash-guy-course-material.md |
| Anchor/link              | docs/method/backlog/cool-ideas/DOCS_splash-guy-course-material.md |
| Direct blockers          | none                                                              |
| Direct dependents        | none                                                              |
| Referenced GitHub issues | #226                                                              |

<hr />

## M140 - Course Material

> **Milestone:** Tumble Tower | **Priority:** P2
>
> Status: active cool idea, blocked by the open Tumble Tower physics,
> lockstep, desync-breaker, and visualization tasks. `docs/guide/tumble-tower.md`
> defines the staged ladder, but no physics-ladder course modules exist yet.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #238
GH issue createdAt: #238: 2026-01-02T22:38:31Z

| Field                    | Value                                                               |
| ------------------------ | ------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                  |
| METHOD id                | M140                                                                |
| Native id                | none                                                                |
| Lane                     | cool-ideas                                                          |
| Status                   | open                                                                |
| Completed                | no                                                                  |
| Source path              | docs/method/backlog/cool-ideas/DOCS_tumble-tower-course-material.md |
| Anchor/link              | docs/method/backlog/cool-ideas/DOCS_tumble-tower-course-material.md |
| Direct blockers          | none                                                                |
| Direct dependents        | none                                                                |
| Referenced GitHub issues | #238                                                                |

<hr />

## M141 - Expose parallel execution counterfactuals

Status: active cool idea. `warp-core` has shard-based parallel execution,
per-worker/per-shard `TickDelta`s, canonical merge, poisoned-delta handling,
and tick receipts for accepted/rejected candidates, but no public artifact that
preserves shard-level intermediate deltas as debugger-inspectable
counterfactuals.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                       |
| ------------------------ | --------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                          |
| METHOD id                | M141                                                                        |
| Native id                | none                                                                        |
| Lane                     | cool-ideas                                                                  |
| Status                   | open                                                                        |
| Completed                | no                                                                          |
| Source path              | docs/method/backlog/cool-ideas/KERNEL_parallel-execution-counterfactuals.md |
| Anchor/link              | docs/method/backlog/cool-ideas/KERNEL_parallel-execution-counterfactuals.md |
| Direct blockers          | none                                                                        |
| Direct dependents        | none                                                                        |
| Referenced GitHub issues | none                                                                        |

<hr />

## M142 - Implement rulial diff / worldline compare MVP (#172)

**User Story:** As a developer comparing two simulation runs, I want to see the first tick where they diverge and a per-tick diff of state changes so that I can pinpoint the cause of nondeterminism or design differences.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M148 (Implement time travel core — pause/rewind/buffer/catch-up (#171)), M149 (Implement Reliving debugger MVP — scrub timeline + causal slice + fork branch (#205))
DAG chain depth: downstream 3; upstream 8
GH issue #: #172
GH issue createdAt: #172: 2026-01-01T19:24:41Z

| Field                    | Value                                                                                                                                                                     |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                                        |
| METHOD id                | M142                                                                                                                                                                      |
| Native id                | T-7-4-1                                                                                                                                                                   |
| Lane                     | cool-ideas                                                                                                                                                                |
| Status                   | blocked                                                                                                                                                                   |
| Completed                | no                                                                                                                                                                        |
| Source path              | docs/method/backlog/cool-ideas/KERNEL_rulial-diff.md                                                                                                                      |
| Anchor/link              | docs/method/backlog/cool-ideas/KERNEL_rulial-diff.md#t-7-4-1-implement-rulial-diff-worldline-compare-mvp-172                                                              |
| Direct blockers          | M148 (Implement time travel core — pause/rewind/buffer/catch-up (#171)), M149 (Implement Reliving debugger MVP — scrub timeline + causal slice + fork branch (#205))      |
| Direct dependents        | M143 (Implement Wesley worldline diff — compare query outputs/proofs across ticks (#199)), M144 (Implement provenance heatmap — blast radius / cohesion over time (#204)) |
| Referenced GitHub issues | #172                                                                                                                                                                      |

<hr />

## M143 - Implement Wesley worldline diff — compare query outputs/proofs across ticks (#199)

**User Story:** As a schema author using Wesley, I want to compare query results and proofs across two worldlines so that I can see how schema-level semantics differ, not just raw graph diffs.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M142 (Implement rulial diff / worldline compare MVP (#172))
DAG chain depth: downstream 2; upstream 9
GH issue #: #199
GH issue createdAt: #199: 2026-01-02T17:08:29Z

| Field                    | Value                                                                                                                                     |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                        |
| METHOD id                | M143                                                                                                                                      |
| Native id                | T-7-4-2                                                                                                                                   |
| Lane                     | cool-ideas                                                                                                                                |
| Status                   | blocked                                                                                                                                   |
| Completed                | no                                                                                                                                        |
| Source path              | docs/method/backlog/cool-ideas/KERNEL_rulial-diff.md                                                                                      |
| Anchor/link              | docs/method/backlog/cool-ideas/KERNEL_rulial-diff.md#t-7-4-2-implement-wesley-worldline-diff-compare-query-outputsproofs-across-ticks-199 |
| Direct blockers          | M142 (Implement rulial diff / worldline compare MVP (#172))                                                                               |
| Direct dependents        | M144 (Implement provenance heatmap — blast radius / cohesion over time (#204))                                                            |
| Referenced GitHub issues | #199                                                                                                                                      |

<hr />

## M144 - Implement provenance heatmap — blast radius / cohesion over time (#204)

**User Story:** As a developer analyzing simulation behavior, I want a visualization of how a single change propagates through the simulation graph over subsequent ticks so that I can understand blast radius and identify tightly coupled subsystems.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M142 (Implement rulial diff / worldline compare MVP (#172)), M143 (Implement Wesley worldline diff — compare query outputs/proofs across ticks (#199))
DAG chain depth: downstream 1; upstream 10
GH issue #: #204
GH issue createdAt: #204: 2026-01-02T17:12:26Z

| Field                    | Value                                                                                                                                                  |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                                                                                                     |
| METHOD id                | M144                                                                                                                                                   |
| Native id                | T-7-4-3                                                                                                                                                |
| Lane                     | cool-ideas                                                                                                                                             |
| Status                   | blocked                                                                                                                                                |
| Completed                | no                                                                                                                                                     |
| Source path              | docs/method/backlog/cool-ideas/KERNEL_rulial-diff.md                                                                                                   |
| Anchor/link              | docs/method/backlog/cool-ideas/KERNEL_rulial-diff.md#t-7-4-3-implement-provenance-heatmap-blast-radius-cohesion-over-time-204                          |
| Direct blockers          | M142 (Implement rulial diff / worldline compare MVP (#172)), M143 (Implement Wesley worldline diff — compare query outputs/proofs across ticks (#199)) |
| Direct dependents        | none                                                                                                                                                   |
| Referenced GitHub issues | #204                                                                                                                                                   |

<hr />

## M145 - Controlled Desync

> **Milestone:** Splash Guy | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #224 is still live and blocks the
> Splash Guy course track (#226). `docs/guide/splash-guy.md` and the course
> shell define the lesson; no controlled-desync harness exists yet.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #224, #226
GH issue createdAt: #224: 2026-01-02T22:11:20Z, #226: 2026-01-02T22:11:50Z

| Field                    | Value                                                                 |
| ------------------------ | --------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                    |
| METHOD id                | M145                                                                  |
| Native id                | none                                                                  |
| Lane                     | cool-ideas                                                            |
| Status                   | open                                                                  |
| Completed                | no                                                                    |
| Source path              | docs/method/backlog/cool-ideas/KERNEL_splash-guy-controlled-desync.md |
| Anchor/link              | docs/method/backlog/cool-ideas/KERNEL_splash-guy-controlled-desync.md |
| Direct blockers          | none                                                                  |
| Direct dependents        | none                                                                  |
| Referenced GitHub issues | #224, #226                                                            |

<hr />

## M146 - Lockstep Protocol

> **Milestone:** Splash Guy | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #223 is still live.
> `docs/guide/course/01-lockstep.md` teaches the contract, but no Splash Guy
> two-peer harness exists yet.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #223
GH issue createdAt: #223: 2026-01-02T22:11:02Z

| Field                    | Value                                                                 |
| ------------------------ | --------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                    |
| METHOD id                | M146                                                                  |
| Native id                | none                                                                  |
| Lane                     | cool-ideas                                                            |
| Status                   | open                                                                  |
| Completed                | no                                                                    |
| Source path              | docs/method/backlog/cool-ideas/KERNEL_splash-guy-lockstep-protocol.md |
| Anchor/link              | docs/method/backlog/cool-ideas/KERNEL_splash-guy-lockstep-protocol.md |
| Direct blockers          | none                                                                  |
| Direct dependents        | none                                                                  |
| Referenced GitHub issues | #223                                                                  |

<hr />

## M147 - Rules & State Model

> **Milestone:** Splash Guy | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #222 is still live and feeds the
> Splash Guy course track (#226). `docs/guide/splash-guy.md` defines the
> current scenario, but no Splash Guy simulation crate/harness exists yet.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #222, #226
GH issue createdAt: #222: 2026-01-02T22:10:47Z, #226: 2026-01-02T22:11:50Z

| Field                    | Value                                                               |
| ------------------------ | ------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                  |
| METHOD id                | M147                                                                |
| Native id                | none                                                                |
| Lane                     | cool-ideas                                                          |
| Status                   | open                                                                |
| Completed                | no                                                                  |
| Source path              | docs/method/backlog/cool-ideas/KERNEL_splash-guy-rules-and-state.md |
| Anchor/link              | docs/method/backlog/cool-ideas/KERNEL_splash-guy-rules-and-state.md |
| Direct blockers          | none                                                                |
| Direct dependents        | none                                                                |
| Referenced GitHub issues | #222, #226                                                          |

<hr />

## M148 - Implement time travel core — pause/rewind/buffer/catch-up (#171)

**User Story:** As a developer, I want to pause the simulation (while inspector/tools remain live), rewind to an earlier tick, fork a new worldline, and catch up via checkpoints so that I can debug temporal bugs without restarting the session.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M175 (Spec — Merge semantics for admitted stream facts across worldlines (#245)), M176 (Spec — Security/capabilities for fork/rewind/merge in multiplayer (#246)), M177 (Implement StreamsFrame inspector support (#170))
DAG chain depth: downstream 5; upstream 6
GH issue #: #171
GH issue createdAt: #171: 2026-01-01T19:24:40Z

| Field                    | Value                                                                                                                                                                                                                     |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                                                                                        |
| METHOD id                | M148                                                                                                                                                                                                                      |
| Native id                | T-7-3-1                                                                                                                                                                                                                   |
| Lane                     | cool-ideas                                                                                                                                                                                                                |
| Status                   | blocked                                                                                                                                                                                                                   |
| Completed                | no                                                                                                                                                                                                                        |
| Source path              | docs/method/backlog/cool-ideas/KERNEL_time-travel-mvp.md                                                                                                                                                                  |
| Anchor/link              | docs/method/backlog/cool-ideas/KERNEL_time-travel-mvp.md#t-7-3-1-implement-time-travel-core-pauserewindbuffercatch-up-171                                                                                                 |
| Direct blockers          | M175 (Spec — Merge semantics for admitted stream facts across worldlines (#245)), M176 (Spec — Security/capabilities for fork/rewind/merge in multiplayer (#246)), M177 (Implement StreamsFrame inspector support (#170)) |
| Direct dependents        | M142 (Implement rulial diff / worldline compare MVP (#172)), M149 (Implement Reliving debugger MVP — scrub timeline + causal slice + fork branch (#205))                                                                  |
| Referenced GitHub issues | #171                                                                                                                                                                                                                      |

<hr />

## M149 - Implement Reliving debugger MVP — scrub timeline + causal slice + fork branch (#205)

**User Story:** As a developer debugging a simulation, I want a timeline scrubber that lets me move to any tick, view the causal slice (which events caused the current state), and fork a new branch from any point so that I can explore "what if" scenarios interactively.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M148 (Implement time travel core — pause/rewind/buffer/catch-up (#171)), M178 (Implement Constraint Lens panel — admission explain-why + counterfactual sliders (#203))
DAG chain depth: downstream 4; upstream 7
GH issue #: #205
GH issue createdAt: #205: 2026-01-02T17:13:36Z

| Field                    | Value                                                                                                                                                                   |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                                      |
| METHOD id                | M149                                                                                                                                                                    |
| Native id                | T-7-3-2                                                                                                                                                                 |
| Lane                     | cool-ideas                                                                                                                                                              |
| Status                   | blocked                                                                                                                                                                 |
| Completed                | no                                                                                                                                                                      |
| Source path              | docs/method/backlog/cool-ideas/KERNEL_time-travel-mvp.md                                                                                                                |
| Anchor/link              | docs/method/backlog/cool-ideas/KERNEL_time-travel-mvp.md#t-7-3-2-implement-reliving-debugger-mvp-scrub-timeline-causal-slice-fork-branch-205                            |
| Direct blockers          | M148 (Implement time travel core — pause/rewind/buffer/catch-up (#171)), M178 (Implement Constraint Lens panel — admission explain-why + counterfactual sliders (#203)) |
| Direct dependents        | M142 (Implement rulial diff / worldline compare MVP (#172))                                                                                                             |
| Referenced GitHub issues | #205                                                                                                                                                                    |

<hr />

## M150 - Desync Breakers

> **Milestone:** Tumble Tower | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #236 is still open and blocks the
> Tumble Tower course track (#238). `docs/guide/tumble-tower.md` defines the
> breaker lesson, and `F32Scalar` has a deterministic LUT-backed trig path, but
> no Tumble Tower physics simulation, lockstep harness, or desync-breaker
> toggles exist yet.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #236, #238
GH issue createdAt: #236: 2026-01-02T22:38:01Z, #238: 2026-01-02T22:38:31Z

| Field                    | Value                                                                 |
| ------------------------ | --------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                    |
| METHOD id                | M150                                                                  |
| Native id                | none                                                                  |
| Lane                     | cool-ideas                                                            |
| Status                   | open                                                                  |
| Completed                | no                                                                    |
| Source path              | docs/method/backlog/cool-ideas/KERNEL_tumble-tower-desync-breakers.md |
| Anchor/link              | docs/method/backlog/cool-ideas/KERNEL_tumble-tower-desync-breakers.md |
| Direct blockers          | none                                                                  |
| Direct dependents        | none                                                                  |
| Referenced GitHub issues | #236, #238                                                            |

<hr />

## M151 - Lockstep Harness

> **Milestone:** Tumble Tower | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #235 is still open and blocks the
> Tumble Tower course track (#238). `docs/guide/tumble-tower.md` defines the
> inputs-only lockstep proof, but no Tumble Tower physics simulation or
> two-peer harness exists yet.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #235, #238
GH issue createdAt: #235: 2026-01-02T22:37:45Z, #238: 2026-01-02T22:38:31Z

| Field                    | Value                                                                  |
| ------------------------ | ---------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                     |
| METHOD id                | M151                                                                   |
| Native id                | none                                                                   |
| Lane                     | cool-ideas                                                             |
| Status                   | open                                                                   |
| Completed                | no                                                                     |
| Source path              | docs/method/backlog/cool-ideas/KERNEL_tumble-tower-lockstep-harness.md |
| Anchor/link              | docs/method/backlog/cool-ideas/KERNEL_tumble-tower-lockstep-harness.md |
| Direct blockers          | none                                                                   |
| Direct dependents        | none                                                                   |
| Referenced GitHub issues | #235, #238                                                             |

<hr />

## M152 - Implement replay-from-checkpoint convergence tests

**User Story:** As a release engineer, I want property tests that prove replaying a simulation from any checkpoint produces identical state to the original run so that I can guarantee worldline convergence for time travel and debugging.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 2; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                     |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                        |
| METHOD id                | M152                                                                                                                      |
| Native id                | T-9-2-1                                                                                                                   |
| Lane                     | cool-ideas                                                                                                                |
| Status                   | open                                                                                                                      |
| Completed                | no                                                                                                                        |
| Source path              | docs/method/backlog/cool-ideas/KERNEL_worldline-convergence.md                                                            |
| Anchor/link              | docs/method/backlog/cool-ideas/KERNEL_worldline-convergence.md#t-9-2-1-implement-replay-from-checkpoint-convergence-tests |
| Direct blockers          | none                                                                                                                      |
| Direct dependents        | M153 (Implement replay-from-patches convergence property tests)                                                           |
| Referenced GitHub issues | none                                                                                                                      |

<hr />

## M153 - Implement replay-from-patches convergence property tests

**User Story:** As a release engineer, I want property-based tests that replay from raw tick patches (not checkpoints) and prove convergence so that I can validate the tick patch format itself is sufficient for deterministic replay.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M152 (Implement replay-from-checkpoint convergence tests)
DAG chain depth: downstream 1; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                           |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                              |
| METHOD id                | M153                                                                                                                            |
| Native id                | T-9-2-2                                                                                                                         |
| Lane                     | cool-ideas                                                                                                                      |
| Status                   | blocked                                                                                                                         |
| Completed                | no                                                                                                                              |
| Source path              | docs/method/backlog/cool-ideas/KERNEL_worldline-convergence.md                                                                  |
| Anchor/link              | docs/method/backlog/cool-ideas/KERNEL_worldline-convergence.md#t-9-2-2-implement-replay-from-patches-convergence-property-tests |
| Direct blockers          | M152 (Implement replay-from-checkpoint convergence tests)                                                                       |
| Direct dependents        | none                                                                                                                            |
| Referenced GitHub issues | none                                                                                                                            |

<hr />

## M154 - Stage 0: AABB

> **Milestone:** Tumble Tower | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #231 is still open and blocks the
> Tumble Tower physics ladder/course (#232, #235, #238). `crates/warp-geom`
> already provides AABB geometry and deterministic broad-phase scaffolding, but
> no Tumble Tower gravity/contact-resolution simulation or physics fingerprint
> exists yet.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #231, #232, #235, #238
GH issue createdAt: #231: 2026-01-02T22:36:44Z, #232: 2026-01-02T22:37:01Z, #235: 2026-01-02T22:37:45Z, #238: 2026-01-02T22:38:31Z

| Field                    | Value                                                            |
| ------------------------ | ---------------------------------------------------------------- |
| Source                   | METHOD task matrix                                               |
| METHOD id                | M154                                                             |
| Native id                | none                                                             |
| Lane                     | cool-ideas                                                       |
| Status                   | open                                                             |
| Completed                | no                                                               |
| Source path              | docs/method/backlog/cool-ideas/MATH_tumble-tower-stage-0-aabb.md |
| Anchor/link              | docs/method/backlog/cool-ideas/MATH_tumble-tower-stage-0-aabb.md |
| Direct blockers          | none                                                             |
| Direct dependents        | none                                                             |
| Referenced GitHub issues | #231, #232, #235, #238                                           |

<hr />

## M155 - Stage 1: Rotation

> **Milestone:** Tumble Tower | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #232 is still open and blocked by
> Stage 0 (#231). `F32Scalar::sin_cos` and trig golden-vector tests exist, but
> no Tumble Tower OBB, angular dynamics, contact manifold, or torque solver
> exists yet.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #231, #232
GH issue createdAt: #231: 2026-01-02T22:36:44Z, #232: 2026-01-02T22:37:01Z

| Field                    | Value                                                                |
| ------------------------ | -------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                   |
| METHOD id                | M155                                                                 |
| Native id                | none                                                                 |
| Lane                     | cool-ideas                                                           |
| Status                   | open                                                                 |
| Completed                | no                                                                   |
| Source path              | docs/method/backlog/cool-ideas/MATH_tumble-tower-stage-1-rotation.md |
| Anchor/link              | docs/method/backlog/cool-ideas/MATH_tumble-tower-stage-1-rotation.md |
| Direct blockers          | none                                                                 |
| Direct dependents        | none                                                                 |
| Referenced GitHub issues | #231, #232                                                           |

<hr />

## M156 - Stage 2: Friction

> **Milestone:** Tumble Tower | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #233 is still open and blocked by
> Stage 1 (#232). `docs/guide/tumble-tower.md` defines friction/restitution as
> Stage 2, but no Tumble Tower contact solver, material model, or physics
> fingerprint exists yet.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #232, #233
GH issue createdAt: #232: 2026-01-02T22:37:01Z, #233: 2026-01-02T22:37:16Z

| Field                    | Value                                                                |
| ------------------------ | -------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                   |
| METHOD id                | M156                                                                 |
| Native id                | none                                                                 |
| Lane                     | cool-ideas                                                           |
| Status                   | open                                                                 |
| Completed                | no                                                                   |
| Source path              | docs/method/backlog/cool-ideas/MATH_tumble-tower-stage-2-friction.md |
| Anchor/link              | docs/method/backlog/cool-ideas/MATH_tumble-tower-stage-2-friction.md |
| Direct blockers          | none                                                                 |
| Direct dependents        | none                                                                 |
| Referenced GitHub issues | #232, #233                                                           |

<hr />

## M157 - Stage 3: Sleeping

> **Milestone:** Tumble Tower | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #234 is still open and blocked by
> Stage 2 (#233). `docs/guide/tumble-tower.md` defines sleeping/islands as
> Stage 3, but no Tumble Tower solver, sleep-state model, island builder, or
> performance benchmark exists yet.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #233, #234
GH issue createdAt: #233: 2026-01-02T22:37:16Z, #234: 2026-01-02T22:37:31Z

| Field                    | Value                                                                |
| ------------------------ | -------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                   |
| METHOD id                | M157                                                                 |
| Native id                | none                                                                 |
| Lane                     | cool-ideas                                                           |
| Status                   | open                                                                 |
| Completed                | no                                                                   |
| Source path              | docs/method/backlog/cool-ideas/MATH_tumble-tower-stage-3-sleeping.md |
| Anchor/link              | docs/method/backlog/cool-ideas/MATH_tumble-tower-stage-3-sleeping.md |
| Direct blockers          | none                                                                 |
| Direct dependents        | none                                                                 |
| Referenced GitHub issues | #233, #234                                                           |

<hr />

## M158 - Continuum Contract Artifact Interchange

Status: cool idea, future protocol lane.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M040 (Witnessed suffix admission shells), M046 (Contract Strands And Counterfactuals)
DAG chain depth: downstream 1; upstream 11
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                 |
| ------------------------ | ------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                    |
| METHOD id                | M158                                                                                  |
| Native id                | none                                                                                  |
| Lane                     | cool-ideas                                                                            |
| Status                   | blocked                                                                               |
| Completed                | no                                                                                    |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_continuum-contract-artifact-interchange.md    |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_continuum-contract-artifact-interchange.md    |
| Direct blockers          | M040 (Witnessed suffix admission shells), M046 (Contract Strands And Counterfactuals) |
| Direct dependents        | none                                                                                  |
| Referenced GitHub issues | none                                                                                  |

<hr />

## M159 - Cross-repo METHOD dashboard

Status: active cool idea. Echo has `cargo xtask method status --json` and the
`method` crate reports backlog lanes, active cycles, and legend load. Local
sibling repos currently include `echo`, `warp-ttd`, `bijou`, and `method`;
`git-warp` is referenced as part of the Continuum constellation but is not
present in this checkout. No cross-repo aggregation tool exists yet.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                  |
| ------------------------ | ---------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                     |
| METHOD id                | M159                                                                   |
| Native id                | none                                                                   |
| Lane                     | cool-ideas                                                             |
| Status                   | open                                                                   |
| Completed                | no                                                                     |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_cross-repo-method-dashboard.md |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_cross-repo-method-dashboard.md |
| Direct blockers          | none                                                                   |
| Direct dependents        | none                                                                   |
| Referenced GitHub issues | none                                                                   |

<hr />

## M160 - Arc<[u8]> to bytes::Bytes migration

**User Story:** As a developer, I want the BlobStore API to use `bytes::Bytes` instead of `Arc<[u8]>` so that zero-copy slicing and network buffer integration are possible.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: none
DAG chain depth: downstream 3; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                       |
| ------------------------ | ----------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                          |
| METHOD id                | M160                                                                                                        |
| Native id                | T-5-4-1                                                                                                     |
| Lane                     | cool-ideas                                                                                                  |
| Status                   | open                                                                                                        |
| Completed                | no                                                                                                          |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-api-evolution.md                                       |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-api-evolution.md#t-5-4-1-arcu8-to-bytesbytes-migration |
| Direct blockers          | none                                                                                                        |
| Direct dependents        | M161 (AsyncBlobStore trait)                                                                                 |
| Referenced GitHub issues | none                                                                                                        |

<hr />

## M161 - AsyncBlobStore trait

**User Story:** As a developer, I want an async variant of BlobStore so that disk and network tiers can perform non-blocking I/O.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M160, M164 (Tiered promotion/demotion (Memory <-> Disk))
DAG chain depth: downstream 2; upstream 3
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                              |
| ------------------------ | -------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                 |
| METHOD id                | M161                                                                                               |
| Native id                | T-5-4-2                                                                                            |
| Lane                     | cool-ideas                                                                                         |
| Status                   | blocked                                                                                            |
| Completed                | no                                                                                                 |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-api-evolution.md                              |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-api-evolution.md#t-5-4-2-asyncblobstore-trait |
| Direct blockers          | M160, M164 (Tiered promotion/demotion (Memory <-> Disk))                                           |
| Direct dependents        | M162 (Enumeration and metadata API)                                                                |
| Referenced GitHub issues | none                                                                                               |

<hr />

## M162 - Enumeration and metadata API

**User Story:** As a developer, I want to list stored blobs and query metadata so that tooling (CLI inspect, GC) can report storage state.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M161 (AsyncBlobStore trait)
DAG chain depth: downstream 1; upstream 4
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                      |
| ------------------------ | ---------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                         |
| METHOD id                | M162                                                                                                       |
| Native id                | T-5-4-3                                                                                                    |
| Lane                     | cool-ideas                                                                                                 |
| Status                   | blocked                                                                                                    |
| Completed                | no                                                                                                         |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-api-evolution.md                                      |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-api-evolution.md#t-5-4-3-enumeration-and-metadata-api |
| Direct blockers          | M161 (AsyncBlobStore trait)                                                                                |
| Direct dependents        | none                                                                                                       |
| Referenced GitHub issues | none                                                                                                       |

<hr />

## M163 - File-per-blob DiskTier implementation

**User Story:** As a developer, I want blobs persisted to disk so that simulation state survives process restarts.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 4; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                           |
| ------------------------ | --------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                              |
| METHOD id                | M163                                                                                                            |
| Native id                | T-5-1-1                                                                                                         |
| Lane                     | cool-ideas                                                                                                      |
| Status                   | open                                                                                                            |
| Completed                | no                                                                                                              |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-disk-tier.md                                               |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-disk-tier.md#t-5-1-1-file-per-blob-disktier-implementation |
| Direct blockers          | none                                                                                                            |
| Direct dependents        | M164 (Tiered promotion/demotion (Memory <-> Disk)), M165 (Mark-sweep reachability analysis)                     |
| Referenced GitHub issues | none                                                                                                            |

<hr />

## M164 - Tiered promotion/demotion (Memory <-> Disk)

**User Story:** As a developer, I want hot blobs cached in memory and cold blobs on disk so that the system balances speed and memory usage.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M163 (File-per-blob DiskTier implementation)
DAG chain depth: downstream 3; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                          |
| ------------------------ | -------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                             |
| METHOD id                | M164                                                                                                           |
| Native id                | T-5-1-2                                                                                                        |
| Lane                     | cool-ideas                                                                                                     |
| Status                   | blocked                                                                                                        |
| Completed                | no                                                                                                             |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-disk-tier.md                                              |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-disk-tier.md#t-5-1-2-tiered-promotiondemotion-memory-disk |
| Direct blockers          | M163 (File-per-blob DiskTier implementation)                                                                   |
| Direct dependents        | M161 (AsyncBlobStore trait), M166 (Eviction policy and background sweep task)                                  |
| Referenced GitHub issues | none                                                                                                           |

<hr />

## M165 - Mark-sweep reachability analysis

**User Story:** As a developer, I want the CAS to identify unreachable blobs so that disk space can be reclaimed.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M163 (File-per-blob DiskTier implementation)
DAG chain depth: downstream 2; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                              |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                                                                 |
| METHOD id                | M165                                                                                                               |
| Native id                | T-5-2-1                                                                                                            |
| Lane                     | cool-ideas                                                                                                         |
| Status                   | blocked                                                                                                            |
| Completed                | no                                                                                                                 |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-gc-sweep-eviction.md                                          |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-gc-sweep-eviction.md#t-5-2-1-mark-sweep-reachability-analysis |
| Direct blockers          | M163 (File-per-blob DiskTier implementation)                                                                       |
| Direct dependents        | M166 (Eviction policy and background sweep task)                                                                   |
| Referenced GitHub issues | none                                                                                                               |

<hr />

## M166 - Eviction policy and background sweep task

**User Story:** As an operator, I want the CAS to automatically evict cold unpinned blobs so that memory and disk usage stay bounded.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M164 (Tiered promotion/demotion (Memory <-> Disk)), M165 (Mark-sweep reachability analysis)
DAG chain depth: downstream 1; upstream 3
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                       |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                          |
| METHOD id                | M166                                                                                                                        |
| Native id                | T-5-2-2                                                                                                                     |
| Lane                     | cool-ideas                                                                                                                  |
| Status                   | blocked                                                                                                                     |
| Completed                | no                                                                                                                          |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-gc-sweep-eviction.md                                                   |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-gc-sweep-eviction.md#t-5-2-2-eviction-policy-and-background-sweep-task |
| Direct blockers          | M164 (Tiered promotion/demotion (Memory <-> Disk)), M165 (Mark-sweep reachability analysis)                                 |
| Direct dependents        | none                                                                                                                        |
| Referenced GitHub issues | none                                                                                                                        |

<hr />

## M167 - Message type definitions and binary encoding

**User Story:** As a developer, I want a compact binary wire format for blob exchange so that peers can request and transfer blobs efficiently.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 2; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                      |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                         |
| METHOD id                | M167                                                                                                                       |
| Native id                | T-5-3-1                                                                                                                    |
| Lane                     | cool-ideas                                                                                                                 |
| Status                   | open                                                                                                                       |
| Completed                | no                                                                                                                         |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-wire-protocol.md                                                      |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-wire-protocol.md#t-5-3-1-message-type-definitions-and-binary-encoding |
| Direct blockers          | none                                                                                                                       |
| Direct dependents        | M168 (Request/response protocol and backpressure)                                                                          |
| Referenced GitHub issues | none                                                                                                                       |

<hr />

## M168 - Request/response protocol and backpressure

**User Story:** As a developer, I want a protocol state machine that handles blob exchange with flow control so that network transfers do not overwhelm memory.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M167 (Message type definitions and binary encoding)
DAG chain depth: downstream 1; upstream 2
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                                                                   |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                      |
| METHOD id                | M168                                                                                                                    |
| Native id                | T-5-3-2                                                                                                                 |
| Lane                     | cool-ideas                                                                                                              |
| Status                   | blocked                                                                                                                 |
| Completed                | no                                                                                                                      |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-wire-protocol.md                                                   |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_deep-storage-wire-protocol.md#t-5-3-2-requestresponse-protocol-and-backpressure |
| Direct blockers          | M167 (Message type definitions and binary encoding)                                                                     |
| Direct dependents        | none                                                                                                                    |
| Referenced GitHub issues | none                                                                                                                    |

<hr />

## M169 - Extract method crate to its own repo

Status: active cool idea. Echo has a Rust `crates/method` library with no Echo
dependencies and a working `cargo xtask method status --json` command. The
external `/Users/james/git/method` repo already contains the TypeScript METHOD
CLI/library, drift detector, MCP server, and tests. What remains is deciding
whether the Rust crate should be extracted into that repo, published separately,
or kept as Echo-local compatibility glue.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                           |
| ------------------------ | --------------------------------------------------------------- |
| Source                   | METHOD task matrix                                              |
| METHOD id                | M169                                                            |
| Native id                | none                                                            |
| Lane                     | cool-ideas                                                      |
| Status                   | open                                                            |
| Completed                | no                                                              |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_method-crate-extract.md |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_method-crate-extract.md |
| Direct blockers          | none                                                            |
| Direct dependents        | none                                                            |
| Referenced GitHub issues | none                                                            |

<hr />

## M170 - Method drift check as pre-push hook

Status: active cool idea. Echo documents `cargo xtask method drift [cycle]` as
planned, but `cargo xtask method --help` exposes only `status` today. The
canonical pre-push hook delegates through `scripts/hooks/pre-push` to
`.githooks/pre-push`; no Method drift gate is wired there yet. The external
`/Users/james/git/method` repo already has drift detection, so the remaining
work is choosing whether Echo calls the external Method CLI or implements the
Rust xtask command first.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                    |
| ------------------------ | ------------------------------------------------------------------------ |
| Source                   | METHOD task matrix                                                       |
| METHOD id                | M170                                                                     |
| Native id                | none                                                                     |
| Lane                     | cool-ideas                                                               |
| Status                   | open                                                                     |
| Completed                | no                                                                       |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_method-drift-as-pre-push-hook.md |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_method-drift-as-pre-push-hook.md |
| Direct blockers          | none                                                                     |
| Direct dependents        | none                                                                     |
| Referenced GitHub issues | none                                                                     |

<hr />

## M171 - Reading envelope inspector

Status: active cool idea. `ReadingEnvelope` and related observer-plan,
budget, rights, witness, and residual posture fields exist in
`echo-wasm-abi`, and `warp-wasm` emits reading envelopes in observation
artifacts. No local inspector/debug view renders that structure yet.
This card remains operational because it turns the active
reading-envelope boundary into a maintainer-facing inspection surface.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                 |
| ------------------------ | --------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                    |
| METHOD id                | M171                                                                  |
| Native id                | none                                                                  |
| Lane                     | cool-ideas                                                            |
| Status                   | open                                                                  |
| Completed                | no                                                                    |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_reading-envelope-inspector.md |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_reading-envelope-inspector.md |
| Direct blockers          | none                                                                  |
| Direct dependents        | none                                                                  |
| Referenced GitHub issues | none                                                                  |

<hr />

## M172 - Visualization

> **Milestone:** Splash Guy | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #225 is still open and blocks
> the Splash Guy course track (#226). `docs/guide/splash-guy.md` defines
> the scenario, but no Splash Guy simulation state, browser renderer, or
> visualization harness exists yet.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #225, #226
GH issue createdAt: #225: 2026-01-02T22:11:36Z, #226: 2026-01-02T22:11:50Z

| Field                    | Value                                                               |
| ------------------------ | ------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                  |
| METHOD id                | M172                                                                |
| Native id                | none                                                                |
| Lane                     | cool-ideas                                                          |
| Status                   | open                                                                |
| Completed                | no                                                                  |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_splash-guy-visualization.md |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_splash-guy-visualization.md |
| Direct blockers          | none                                                                |
| Direct dependents        | none                                                                |
| Referenced GitHub issues | #225, #226                                                          |

<hr />

## M173 - Spec — dt policy: fixed timestep vs admitted dt stream (#243)

**User Story:** As an engine architect, I want a locked design decision on whether Echo uses a fixed timestep or variable dt admitted as a stream so that all downstream code (physics, animation, admission budgets) can commit to one model.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M049 (Spec — HistoryTime vs HostTime field classification (#191)), M050 (Spec — TTL/deadline semantics are ticks only (#192))
DAG chain depth: downstream 8; upstream 3
GH issue #: #243
GH issue createdAt: #243: 2026-01-03T01:20:09Z

| Field                    | Value                                                                                                                                   |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                      |
| METHOD id                | M173                                                                                                                                    |
| Native id                | T-7-2-1                                                                                                                                 |
| Lane                     | cool-ideas                                                                                                                              |
| Status                   | blocked                                                                                                                                 |
| Completed                | no                                                                                                                                      |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_streams-inspector.md                                                                            |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_streams-inspector.md#t-7-2-1-spec-dt-policy-fixed-timestep-vs-admitted-dt-stream-243            |
| Direct blockers          | M049 (Spec — HistoryTime vs HostTime field classification (#191)), M050 (Spec — TTL/deadline semantics are ticks only (#192))           |
| Direct dependents        | M176 (Spec — Security/capabilities for fork/rewind/merge in multiplayer (#246)), M177 (Implement StreamsFrame inspector support (#170)) |
| Referenced GitHub issues | #243                                                                                                                                    |

<hr />

## M174 - Spec — TimeStream retention, spool compaction, wormhole density (#244)

**User Story:** As an operator deploying Echo sessions, I want documented policies for how long TimeStream spools are retained, when compaction occurs, and how wormhole density is managed so that I can size storage and predict seek latency.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M049 (Spec — HistoryTime vs HostTime field classification (#191)), M050 (Spec — TTL/deadline semantics are ticks only (#192))
DAG chain depth: downstream 8; upstream 3
GH issue #: #244
GH issue createdAt: #244: 2026-01-03T01:20:24Z

| Field                    | Value                                                                                                                                   |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                      |
| METHOD id                | M174                                                                                                                                    |
| Native id                | T-7-2-2                                                                                                                                 |
| Lane                     | cool-ideas                                                                                                                              |
| Status                   | blocked                                                                                                                                 |
| Completed                | no                                                                                                                                      |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_streams-inspector.md                                                                            |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_streams-inspector.md#t-7-2-2-spec-timestream-retention-spool-compaction-wormhole-density-244    |
| Direct blockers          | M049 (Spec — HistoryTime vs HostTime field classification (#191)), M050 (Spec — TTL/deadline semantics are ticks only (#192))           |
| Direct dependents        | M176 (Spec — Security/capabilities for fork/rewind/merge in multiplayer (#246)), M177 (Implement StreamsFrame inspector support (#170)) |
| Referenced GitHub issues | #244                                                                                                                                    |

<hr />

## M175 - Spec — Merge semantics for admitted stream facts across worldlines (#245)

**User Story:** As a multiplayer game developer, I want clear merge semantics for when worldlines rejoin so that buffered "future" events are handled deterministically and I can reason about conflict resolution.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M049 (Spec — HistoryTime vs HostTime field classification (#191)), M050 (Spec — TTL/deadline semantics are ticks only (#192))
DAG chain depth: downstream 7; upstream 3
GH issue #: #245
GH issue createdAt: #245: 2026-01-03T01:20:40Z

| Field                    | Value                                                                                                                                     |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                        |
| METHOD id                | M175                                                                                                                                      |
| Native id                | T-7-2-3                                                                                                                                   |
| Lane                     | cool-ideas                                                                                                                                |
| Status                   | blocked                                                                                                                                   |
| Completed                | no                                                                                                                                        |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_streams-inspector.md                                                                              |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_streams-inspector.md#t-7-2-3-spec-merge-semantics-for-admitted-stream-facts-across-worldlines-245 |
| Direct blockers          | M049 (Spec — HistoryTime vs HostTime field classification (#191)), M050 (Spec — TTL/deadline semantics are ticks only (#192))             |
| Direct dependents        | M148 (Implement time travel core — pause/rewind/buffer/catch-up (#171)), M177 (Implement StreamsFrame inspector support (#170))           |
| Referenced GitHub issues | #245                                                                                                                                      |

<hr />

## M176 - Spec — Security/capabilities for fork/rewind/merge in multiplayer (#246)

**User Story:** As a session host, I want a capability model that controls who can fork, rewind, and merge worldlines so that time-travel operations cannot be abused in multiplayer.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M173 (Spec — dt policy: fixed timestep vs admitted dt stream (#243)), M174 (Spec — TimeStream retention, spool compaction, wormhole density (#244))
DAG chain depth: downstream 7; upstream 4
GH issue #: #246
GH issue createdAt: #246: 2026-01-03T01:20:55Z

| Field                    | Value                                                                                                                                               |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                  |
| METHOD id                | M176                                                                                                                                                |
| Native id                | T-7-2-4                                                                                                                                             |
| Lane                     | cool-ideas                                                                                                                                          |
| Status                   | blocked                                                                                                                                             |
| Completed                | no                                                                                                                                                  |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_streams-inspector.md                                                                                        |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_streams-inspector.md#t-7-2-4-spec-securitycapabilities-for-forkrewindmerge-in-multiplayer-246               |
| Direct blockers          | M173 (Spec — dt policy: fixed timestep vs admitted dt stream (#243)), M174 (Spec — TimeStream retention, spool compaction, wormhole density (#244)) |
| Direct dependents        | M148 (Implement time travel core — pause/rewind/buffer/catch-up (#171)), M177 (Implement StreamsFrame inspector support (#170))                     |
| Referenced GitHub issues | #246                                                                                                                                                |

<hr />

## M177 - Implement StreamsFrame inspector support (#170)

**User Story:** As a developer debugging a live Echo session, I want an inspector frame that shows per-stream backlog, per-view cursor positions, and recent admission decisions so that I can understand why events are or are not entering the simulation.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M173 (Spec — dt policy: fixed timestep vs admitted dt stream (#243)), M174 (Spec — TimeStream retention, spool compaction, wormhole density (#244)), M175 (Spec — Merge semantics for admitted stream facts across worldlines (#245)), M176 (Spec — Security/capabilities for fork/rewind/merge in multiplayer (#246))
DAG chain depth: downstream 6; upstream 5
GH issue #: #170
GH issue createdAt: #170: 2026-01-01T19:24:38Z

| Field                    | Value                                                                                                                                                                                                                                                                                                                  |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                                                                                                                                                                                     |
| METHOD id                | M177                                                                                                                                                                                                                                                                                                                   |
| Native id                | T-7-2-5                                                                                                                                                                                                                                                                                                                |
| Lane                     | cool-ideas                                                                                                                                                                                                                                                                                                             |
| Status                   | blocked                                                                                                                                                                                                                                                                                                                |
| Completed                | no                                                                                                                                                                                                                                                                                                                     |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_streams-inspector.md                                                                                                                                                                                                                                                           |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_streams-inspector.md#t-7-2-5-implement-streamsframe-inspector-support-170                                                                                                                                                                                                      |
| Direct blockers          | M173 (Spec — dt policy: fixed timestep vs admitted dt stream (#243)), M174 (Spec — TimeStream retention, spool compaction, wormhole density (#244)), M175 (Spec — Merge semantics for admitted stream facts across worldlines (#245)), M176 (Spec — Security/capabilities for fork/rewind/merge in multiplayer (#246)) |
| Direct dependents        | M148 (Implement time travel core — pause/rewind/buffer/catch-up (#171)), M178 (Implement Constraint Lens panel — admission explain-why + counterfactual sliders (#203))                                                                                                                                                |
| Referenced GitHub issues | #170                                                                                                                                                                                                                                                                                                                   |

<hr />

## M178 - Implement Constraint Lens panel — admission explain-why + counterfactual sliders (#203)

**User Story:** As a designer tuning admission policies, I want a UI panel that explains why each event was admitted or rejected and lets me adjust policy parameters with counterfactual sliders so that I can iterate on admission budgets without modifying code.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: triage before keeping. This is tracked as a lower-commitment idea and should survive only if it still points at a real future hill.

DAG blocked by: M177 (Implement StreamsFrame inspector support (#170))
DAG chain depth: downstream 5; upstream 6
GH issue #: #203
GH issue createdAt: #203: 2026-01-02T17:12:19Z

| Field                    | Value                                                                                                                                                 |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                                                                                                    |
| METHOD id                | M178                                                                                                                                                  |
| Native id                | T-7-2-6                                                                                                                                               |
| Lane                     | cool-ideas                                                                                                                                            |
| Status                   | blocked                                                                                                                                               |
| Completed                | no                                                                                                                                                    |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_streams-inspector.md                                                                                          |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_streams-inspector.md#t-7-2-6-implement-constraint-lens-panel-admission-explain-why-counterfactual-sliders-203 |
| Direct blockers          | M177 (Implement StreamsFrame inspector support (#170))                                                                                                |
| Direct dependents        | M149 (Implement Reliving debugger MVP — scrub timeline + causal slice + fork branch (#205))                                                           |
| Referenced GitHub issues | #203                                                                                                                                                  |

<hr />

## M179 - Visualization

> **Milestone:** Tumble Tower | **Priority:** P2
>
> Status: active cool idea. Task DAG issue #237 is still open and blocks
> the Tumble Tower course track (#238). `docs/guide/tumble-tower.md`
> defines the visualization/debug-overlay need, but no Tumble Tower physics
> simulation, browser renderer, or visualization harness exists yet.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: #237, #238
GH issue createdAt: #237: 2026-01-02T22:38:15Z, #238: 2026-01-02T22:38:31Z

| Field                    | Value                                                                 |
| ------------------------ | --------------------------------------------------------------------- |
| Source                   | METHOD task matrix                                                    |
| METHOD id                | M179                                                                  |
| Native id                | none                                                                  |
| Lane                     | cool-ideas                                                            |
| Status                   | open                                                                  |
| Completed                | no                                                                    |
| Source path              | docs/method/backlog/cool-ideas/PLATFORM_tumble-tower-visualization.md |
| Anchor/link              | docs/method/backlog/cool-ideas/PLATFORM_tumble-tower-visualization.md |
| Direct blockers          | none                                                                  |
| Direct dependents        | none                                                                  |
| Referenced GitHub issues | #237, #238                                                            |

<hr />

## M180 - RED/GREEN can't be separate commits

Status: active bad-code note. `scripts/verify-local.sh` runs clippy with
`-D warnings -D missing_docs`, so production `todo!()` and `unimplemented!()`
stubs still fail local gates. The repo already uses explicit test-only
allowances for ignored future-contract tests, so the remaining problem is
documenting the approved RED pattern rather than weakening production linting.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                   |
| ------------------------ | ------------------------------------------------------- |
| Source                   | METHOD task matrix                                      |
| METHOD id                | M180                                                    |
| Native id                | none                                                    |
| Lane                     | bad-code                                                |
| Status                   | open                                                    |
| Completed                | no                                                      |
| Source path              | docs/method/backlog/bad-code/red-green-lint-friction.md |
| Anchor/link              | docs/method/backlog/bad-code/red-green-lint-friction.md |
| Direct blockers          | none                                                    |
| Direct dependents        | none                                                    |
| Referenced GitHub issues | none                                                    |

<hr />

## M181 - xtask main.rs is a god file

Status: active bad-code note. `xtask/src/main.rs` is still the only source file
under `xtask/src/` and is roughly 7.8k lines; argument definitions, command
dispatch, GitHub/PR helpers, benchmark tooling, DIND tooling, docs linting, and
Method formatting are still mixed together.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. This is currently available on the METHOD frontier because all direct blockers are complete or absent.

DAG blocked by: none
DAG chain depth: downstream 1; upstream 1
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                          |
| ------------------------ | ---------------------------------------------- |
| Source                   | METHOD task matrix                             |
| METHOD id                | M181                                           |
| Native id                | none                                           |
| Lane                     | bad-code                                       |
| Status                   | open                                           |
| Completed                | no                                             |
| Source path              | docs/method/backlog/bad-code/xtask-god-file.md |
| Anchor/link              | docs/method/backlog/bad-code/xtask-god-file.md |
| Direct blockers          | none                                           |
| Direct dependents        | none                                           |
| Referenced GitHub issues | none                                           |

<hr />

# GitHub Issues

## GH-19 - Spec: Persistent Store (on-disk)

Define on-disk persistent store that does not affect snapshot hashing.
Scope: 64-byte header (counts/offsets/flags), ULEB128 indices, property table (KV + string pool), bit flags, optional edge-as-graph (persistence-only). Include canonicalization notes and security considerations.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #19
GH issue createdAt: 2025-10-30T07:54:56Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #19                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:54:56Z                           |
| Updated at          | 2026-01-17T16:47:04Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/19 |
| Labels              | feature, spec, tooling, backlog                |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-20 - Spec: Commit/Manifest Signing

Specify Ed25519 signing for exported commits and plugin manifests. Define canonicalization rules and verification flow; wire CI for signing/verification.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 5; upstream max 1
GH issue #: #20
GH issue createdAt: 2025-10-30T07:54:57Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #20                                            |
| State               | OPEN                                           |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:54:57Z                           |
| Updated at          | 2025-11-02T01:21:51Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/20 |
| Labels              | feature, spec, tooling, security, backlog      |
| Mapped METHOD tasks | M093                                           |

<hr />

## GH-21 - Spec: Security Contexts (FFI/WASM/CLI)

Define security contexts and deterministic resource limits for FFI/WASM/CLI: memory/time/recursion caps; UTF-8/path checks; overflow-safe math.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 2; upstream max 1
GH issue #: #21
GH issue createdAt: 2025-10-30T07:54:58Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #21                                            |
| State               | OPEN                                           |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:54:58Z                           |
| Updated at          | 2025-11-02T01:21:52Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/21 |
| Labels              | feature, spec, security, backlog               |
| Mapped METHOD tasks | M094                                           |

<hr />

## GH-22 - Benchmarks & CI Regression Gates

Criterion-based benches with JSON output, P50/P95/P99 metrics, and CI thresholds; optional perf counters on Linux.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #22
GH issue createdAt: 2025-10-30T07:54:59Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #22                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:54:59Z                           |
| Updated at          | 2026-02-13T05:24:04Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/22 |
| Labels              | feature, tooling                               |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-23 - CLI: verify/bench/inspect

Scaffold rmg-cli: `verify` (snapshot integrity), `bench` (run criterion + collect JSON), `inspect` (snapshot summary).

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #23
GH issue createdAt: 2025-10-30T07:55:00Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #23                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:55:00Z                           |
| Updated at          | 2026-03-08T08:22:14Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/23 |
| Labels              | feature, tooling                               |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-24 - Editor Hot-Reload (spec + impl)

Editor-only hot-reload: watch → debounce → stage → atomic snapshot swap; version counter and deferred cleanup; determinism fences.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M100 (File Watcher / Debounce (#76))
DAG chain depth: downstream max 1; upstream max 3
GH issue #: #24
GH issue createdAt: 2025-10-30T07:55:00Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #24                                            |
| State               | OPEN                                           |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:55:00Z                           |
| Updated at          | 2025-11-02T01:21:54Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/24 |
| Labels              | feature, tooling, backlog                      |
| Mapped METHOD tasks | M101                                           |

<hr />

## GH-25 - Importer: TurtlGraph → Echo store

Importer to ingest TurtlGraph bundles into Echo store: read string pool/properties, map to node/edge payload schemas, verify BLAKE3 digests.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #25
GH issue createdAt: 2025-10-30T07:55:01Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #25                                            |
| State               | OPEN                                           |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:55:01Z                           |
| Updated at          | 2025-11-02T01:21:56Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/25 |
| Labels              | feature, tooling, runtime, backlog             |
| Mapped METHOD tasks | M103                                           |

<hr />

## GH-26 - Plugin ABI (C) v0

C ABI v0 for plugins: lifecycle, capability tokens, version negotiation, error codes; minimal example plugin + loader.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #26
GH issue createdAt: 2025-10-30T07:55:02Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #26                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:55:02Z                           |
| Updated at          | 2026-03-04T01:45:23Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/26 |
| Labels              | feature, runtime, backlog                      |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-27 - Add golden test vectors (encoder/decoder)

JSON+hex fixtures covering header and minimal tables.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #27
GH issue createdAt: 2025-10-30T07:57:03Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #27                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:57:03Z                           |
| Updated at          | 2026-01-17T16:46:31Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/27 |
| Labels              | task, spec, backlog                            |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-28 - Draft spec document (header/ULEB128/property/string-pool)

Create docs/spec-persistent-store.md with diagrams and encoding details.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #28
GH issue createdAt: 2025-10-30T07:57:41Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #28                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:57:41Z                           |
| Updated at          | 2026-01-17T16:46:26Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/28 |
| Labels              | task, spec, backlog                            |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-29 - Prototype header+string-pool encoder

Small Rust module writing header + string pool.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #29
GH issue createdAt: 2025-10-30T07:57:45Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #29                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:57:45Z                           |
| Updated at          | 2026-01-17T16:46:36Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/29 |
| Labels              | task, spec, backlog                            |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-30 - Prototype header+string-pool decoder

Small Rust module reading header + string pool.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #30
GH issue createdAt: 2025-10-30T07:57:51Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #30                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:57:51Z                           |
| Updated at          | 2026-01-17T16:46:20Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/30 |
| Labels              | task, spec, backlog                            |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-31 - Docs sync + decision log

Link spec from snapshot docs; add entry to decision-log.md.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #31
GH issue createdAt: 2025-10-30T07:57:55Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #31                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:57:55Z                           |
| Updated at          | 2025-11-02T01:00:26Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/31 |
| Labels              | task, spec                                     |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-32 - Draft signing spec

Create docs/spec-commit-signing.md with canonicalization rules.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #32
GH issue createdAt: 2025-10-30T07:57:59Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #32                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:57:59Z                           |
| Updated at          | 2026-01-17T14:25:23Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/32 |
| Labels              | task, spec, tooling, security, backlog         |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-33 - CI: sign release artifacts (dry run)

Add job to produce detached signatures for artifacts.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M106 (Key Management Doc (#35))
DAG chain depth: downstream max 3; upstream max 3
GH issue #: #33
GH issue createdAt: 2025-10-30T07:58:06Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #33                                            |
| State               | OPEN                                           |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:58:06Z                           |
| Updated at          | 2025-11-02T01:22:06Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/33 |
| Labels              | task, spec, tooling, security, backlog         |
| Mapped METHOD tasks | M107                                           |

<hr />

## GH-34 - CLI verify path

Add 'echo verify --sig' to check signatures.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M107 (CI — Sign Release Artifacts (Dry Run) (#33))
DAG chain depth: downstream max 2; upstream max 4
GH issue #: #34
GH issue createdAt: 2025-10-30T07:58:13Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #34                                            |
| State               | OPEN                                           |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:58:13Z                           |
| Updated at          | 2025-11-02T01:22:08Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/34 |
| Labels              | task, spec, tooling, security, backlog         |
| Mapped METHOD tasks | M108                                           |

<hr />

## GH-35 - Key management doc

Describe key storage/rotation and env secrets.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M093 (Spec — Commit/Manifest Signing (#20))
DAG chain depth: downstream max 4; upstream max 2
GH issue #: #35
GH issue createdAt: 2025-10-30T07:58:20Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #35                                            |
| State               | OPEN                                           |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:58:20Z                           |
| Updated at          | 2025-11-02T01:22:09Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/35 |
| Labels              | task, spec, tooling, security, backlog         |
| Mapped METHOD tasks | M106                                           |

<hr />

## GH-36 - CI: verify signatures

Add verification gate for uploaded artifacts.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M108 (CLI Verify Path (#34))
DAG chain depth: downstream max 1; upstream max 5
GH issue #: #36
GH issue createdAt: 2025-10-30T07:58:28Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #36                                            |
| State               | OPEN                                           |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:58:28Z                           |
| Updated at          | 2025-11-02T01:22:11Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/36 |
| Labels              | task, spec, tooling, security, backlog         |
| Mapped METHOD tasks | M109                                           |

<hr />

## GH-37 - Draft security contexts spec

Create docs/spec-security-contexts.md with defaults and error codes.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #37
GH issue createdAt: 2025-10-30T07:58:34Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #37                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:58:34Z                           |
| Updated at          | 2026-01-17T14:25:21Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/37 |
| Labels              | task, security, backlog                        |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-38 - FFI limits and validation

Enforce memory/time/recursion caps; UTF-8, path checks, safe math.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as historical source until the mapped METHOD rows are also retired.

DAG blocked by: M094 (Spec — Security Contexts (#21))
DAG chain depth: downstream max 1; upstream max 2
GH issue #: #38
GH issue createdAt: 2025-10-30T07:58:39Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #38                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:58:39Z                           |
| Updated at          | 2026-03-04T01:45:33Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/38 |
| Labels              | task, security, backlog                        |
| Mapped METHOD tasks | M095                                           |

<hr />

## GH-39 - WASM input validation

Buffer length checks and guard rails in rmg-wasm.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #39
GH issue createdAt: 2025-10-30T07:58:44Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #39                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:58:44Z                           |
| Updated at          | 2026-01-17T16:42:50Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/39 |
| Labels              | task, security, backlog                        |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-40 - Unit tests for denials

Cover overflow/invalid inputs across FFI/WASM.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #40
GH issue createdAt: 2025-10-30T07:58:48Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #40                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:58:48Z                           |
| Updated at          | 2026-01-17T16:45:07Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/40 |
| Labels              | task, security, backlog                        |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-41 - README+docs

Document defaults and toggles.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #41
GH issue createdAt: 2025-10-30T07:58:52Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #41                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:58:52Z                           |
| Updated at          | 2026-01-17T14:25:18Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/41 |
| Labels              | task, security                                 |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-42 - Create benches crate

Set up Criterion workspace crate.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #42
GH issue createdAt: 2025-10-30T07:58:56Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #42                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:58:56Z                           |
| Updated at          | 2025-11-02T12:47:59Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/42 |
| Labels              | task, tooling                                  |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-43 - Snapshot hash microbench

Benchmark compute_state_root over sample graphs.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #43
GH issue createdAt: 2025-10-30T07:59:00Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #43                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:59:00Z                           |
| Updated at          | 2025-11-02T12:47:58Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/43 |
| Labels              | task, tooling                                  |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-44 - Scheduler drain microbench

Benchmark DeterministicScheduler::drain_for_tx.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #44
GH issue createdAt: 2025-10-30T07:59:04Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #44                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:59:04Z                           |
| Updated at          | 2025-11-02T12:47:59Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/44 |
| Labels              | task, tooling                                  |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-45 - JSON report + CI upload

Write report and upload as artifact.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #45
GH issue createdAt: 2025-10-30T07:59:08Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #45                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:59:08Z                           |
| Updated at          | 2025-11-02T12:47:59Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/45 |
| Labels              | task, tooling                                  |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-46 - Regression thresholds gate

Fail CI on >X% regression; script in scripts/.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #46
GH issue createdAt: 2025-10-30T07:59:12Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #46                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:59:12Z                           |
| Updated at          | 2025-11-02T12:48:00Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/46 |
| Labels              | task, tooling                                  |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-47 - Scaffold CLI subcommands

Add verify/bench/inspect to rmg-cli.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #47
GH issue createdAt: 2025-10-30T07:59:16Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #47                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:59:16Z                           |
| Updated at          | 2026-03-08T08:22:16Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/47 |
| Labels              | task, tooling                                  |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-48 - Implement 'verify'

Snapshot hash and signature checks (when enabled).

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #48
GH issue createdAt: 2025-10-30T07:59:21Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #48                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:59:21Z                           |
| Updated at          | 2026-03-08T08:22:17Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/48 |
| Labels              | task, tooling                                  |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-49 - Implement 'bench'

Invoke benches; print summary and path to JSON.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #49
GH issue createdAt: 2025-10-30T07:59:25Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #49                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:59:25Z                           |
| Updated at          | 2026-03-08T08:22:19Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/49 |
| Labels              | task, tooling                                  |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-50 - Implement 'inspect'

Print snapshot summary and basic graph stats.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #50
GH issue createdAt: 2025-10-30T07:59:29Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #50                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:59:29Z                           |
| Updated at          | 2026-03-08T08:22:20Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/50 |
| Labels              | task, tooling                                  |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-51 - Docs/man pages

Update README and help text.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #51
GH issue createdAt: 2025-10-30T07:59:33Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #51                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T07:59:33Z                           |
| Updated at          | 2026-03-08T08:22:22Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/51 |
| Labels              | task                                           |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-52 - Prototype header+string-pool encoder

Small Rust module writing header + string pool.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #52
GH issue createdAt: 2025-10-30T08:00:00Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #52                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:00:00Z                           |
| Updated at          | 2025-10-30T09:25:12Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/52 |
| Labels              | duplicate, task, spec                          |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-53 - Prototype header+string-pool decoder

Small Rust module reading header + string pool.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #53
GH issue createdAt: 2025-10-30T08:00:04Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #53                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:00:04Z                           |
| Updated at          | 2025-10-30T09:25:16Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/53 |
| Labels              | duplicate, task, spec                          |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-54 - Docs sync + decision log

Link spec from snapshot docs; add entry to decision-log.md.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #54
GH issue createdAt: 2025-10-30T08:00:09Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #54                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:00:09Z                           |
| Updated at          | 2025-10-30T09:25:21Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/54 |
| Labels              | duplicate, task, spec                          |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-55 - Draft signing spec

Create docs/spec-commit-signing.md with canonicalization rules.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #55
GH issue createdAt: 2025-10-30T08:00:13Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #55                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:00:13Z                           |
| Updated at          | 2025-10-30T09:25:27Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/55 |
| Labels              | duplicate, task, spec, tooling, security       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-56 - CI: sign release artifacts (dry run)

Add job to produce detached signatures for artifacts.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #56
GH issue createdAt: 2025-10-30T08:00:20Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #56                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:00:20Z                           |
| Updated at          | 2025-10-30T09:25:32Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/56 |
| Labels              | duplicate, task, spec, tooling, security       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-57 - CLI verify path

Add 'echo verify --sig' to check signatures.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #57
GH issue createdAt: 2025-10-30T08:00:28Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #57                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:00:28Z                           |
| Updated at          | 2025-10-30T09:25:36Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/57 |
| Labels              | duplicate, task, spec, tooling, security       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-58 - Key management doc

Describe key storage/rotation and env secrets.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #58
GH issue createdAt: 2025-10-30T08:00:35Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #58                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:00:35Z                           |
| Updated at          | 2025-10-30T09:25:41Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/58 |
| Labels              | duplicate, task, spec, tooling, security       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-59 - CI: verify signatures

Add verification gate for uploaded artifacts.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #59
GH issue createdAt: 2025-10-30T08:00:42Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #59                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:00:42Z                           |
| Updated at          | 2025-10-30T09:25:46Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/59 |
| Labels              | duplicate, task, spec, tooling, security       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-60 - Draft security contexts spec

Create docs/spec-security-contexts.md with defaults and error codes.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #60
GH issue createdAt: 2025-10-30T08:00:49Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #60                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:00:49Z                           |
| Updated at          | 2025-10-30T09:25:50Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/60 |
| Labels              | duplicate, task, security                      |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-61 - FFI limits and validation

Enforce memory/time/recursion caps; UTF-8, path checks, safe math.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #61
GH issue createdAt: 2025-10-30T08:00:52Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #61                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:00:52Z                           |
| Updated at          | 2025-10-30T09:25:55Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/61 |
| Labels              | duplicate, task, security                      |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-62 - WASM input validation

Buffer length checks and guard rails in rmg-wasm.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #62
GH issue createdAt: 2025-10-30T08:00:56Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #62                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:00:56Z                           |
| Updated at          | 2025-10-30T09:26:00Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/62 |
| Labels              | duplicate, task, security                      |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-63 - Unit tests for denials

Cover overflow/invalid inputs across FFI/WASM.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #63
GH issue createdAt: 2025-10-30T08:01:00Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #63                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:01:00Z                           |
| Updated at          | 2025-10-30T09:26:04Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/63 |
| Labels              | duplicate, task, security                      |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-64 - README+docs

Document defaults and toggles.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #64
GH issue createdAt: 2025-10-30T08:01:04Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #64                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:01:04Z                           |
| Updated at          | 2025-10-30T09:26:09Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/64 |
| Labels              | duplicate, task, security                      |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-65 - Create benches crate

Set up Criterion workspace crate.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #65
GH issue createdAt: 2025-10-30T08:01:48Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #65                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:01:48Z                           |
| Updated at          | 2025-10-30T09:26:14Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/65 |
| Labels              | duplicate, task, tooling                       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-66 - Snapshot hash microbench

Benchmark compute_state_root over sample graphs.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #66
GH issue createdAt: 2025-10-30T08:01:53Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #66                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:01:53Z                           |
| Updated at          | 2025-10-30T09:26:18Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/66 |
| Labels              | duplicate, task, tooling                       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-67 - Scheduler drain microbench

Benchmark DeterministicScheduler::drain_for_tx.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #67
GH issue createdAt: 2025-10-30T08:01:57Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #67                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:01:57Z                           |
| Updated at          | 2025-10-30T09:26:23Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/67 |
| Labels              | duplicate, task, tooling                       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-68 - JSON report + CI upload

Write report and upload as artifact.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #68
GH issue createdAt: 2025-10-30T08:02:02Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #68                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:02Z                           |
| Updated at          | 2025-10-30T09:26:28Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/68 |
| Labels              | duplicate, task, tooling                       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-69 - Regression thresholds gate

Fail CI on >X% regression; script in scripts/.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #69
GH issue createdAt: 2025-10-30T08:02:06Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #69                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:06Z                           |
| Updated at          | 2025-10-30T09:26:33Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/69 |
| Labels              | duplicate, task, tooling                       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-70 - Scaffold CLI subcommands

Add verify/bench/inspect to rmg-cli.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #70
GH issue createdAt: 2025-10-30T08:02:09Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #70                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:09Z                           |
| Updated at          | 2025-10-30T09:26:38Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/70 |
| Labels              | duplicate, task, tooling                       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-71 - Implement 'verify'

Snapshot hash and signature checks (when enabled).

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #71
GH issue createdAt: 2025-10-30T08:02:13Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #71                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:13Z                           |
| Updated at          | 2025-10-30T09:26:42Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/71 |
| Labels              | duplicate, task, tooling                       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-72 - Implement 'bench'

Invoke benches; print summary and path to JSON.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #72
GH issue createdAt: 2025-10-30T08:02:17Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #72                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:17Z                           |
| Updated at          | 2025-10-30T09:26:47Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/72 |
| Labels              | duplicate, task, tooling                       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-73 - Implement 'inspect'

Print snapshot summary and basic graph stats.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #73
GH issue createdAt: 2025-10-30T08:02:21Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #73                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:21Z                           |
| Updated at          | 2025-10-30T09:26:51Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/73 |
| Labels              | duplicate, task, tooling                       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-74 - Docs/man pages

Update README and help text.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #74
GH issue createdAt: 2025-10-30T08:02:25Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #74                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:25Z                           |
| Updated at          | 2025-10-30T09:26:56Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/74 |
| Labels              | duplicate, task, tooling                       |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-75 - Draft hot-reload spec

docs/spec-editor-hot-reload.md with invariants.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 3; upstream max 1
GH issue #: #75
GH issue createdAt: 2025-10-30T08:02:29Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #75                                            |
| State               | OPEN                                           |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:29Z                           |
| Updated at          | 2025-11-02T01:22:19Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/75 |
| Labels              | task, tooling, backlog                         |
| Mapped METHOD tasks | M099                                           |

<hr />

## GH-76 - File watcher/debounce

Add cross-platform watcher (notify) for editor builds.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M099 (Draft Hot-Reload Spec (#75))
DAG chain depth: downstream max 2; upstream max 2
GH issue #: #76
GH issue createdAt: 2025-10-30T08:02:33Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #76                                            |
| State               | OPEN                                           |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:33Z                           |
| Updated at          | 2025-11-02T01:22:20Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/76 |
| Labels              | task, tooling, backlog                         |
| Mapped METHOD tasks | M100                                           |

<hr />

## GH-77 - Atomic snapshot swap

Engine wrapper with version counter + deferred cleanup.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #77
GH issue createdAt: 2025-10-30T08:02:38Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #77                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:38Z                           |
| Updated at          | 2026-01-17T16:45:46Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/77 |
| Labels              | task, tooling, backlog                         |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-78 - Editor gate + tests

cfg(editor); unit tests for swap behavior.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #78
GH issue createdAt: 2025-10-30T08:02:41Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #78                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:41Z                           |
| Updated at          | 2026-01-17T16:45:34Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/78 |
| Labels              | task, tooling, backlog                         |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-79 - Docs/logging

Update execution-plan and decision-log.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #79
GH issue createdAt: 2025-10-30T08:02:45Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #79                                            |
| State               | OPEN                                           |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:45Z                           |
| Updated at          | 2025-11-02T01:22:25Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/79 |
| Labels              | task, tooling, backlog                         |
| Mapped METHOD tasks | M110                                           |

<hr />

## GH-80 - Draft importer spec

docs/spec-importer-turtlgraph.md with mapping tables.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as historical source until the mapped METHOD rows are also retired.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #80
GH issue createdAt: 2025-10-30T08:02:49Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #80                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:49Z                           |
| Updated at          | 2026-01-17T16:44:36Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/80 |
| Labels              | task, tooling, runtime, backlog                |
| Mapped METHOD tasks | M103                                           |

<hr />

## GH-81 - Minimal reader

Parse string pool and property KVs from bundle.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as historical source until the mapped METHOD rows are also retired.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #81
GH issue createdAt: 2025-10-30T08:02:55Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #81                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:02:55Z                           |
| Updated at          | 2026-01-17T16:44:33Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/81 |
| Labels              | task, tooling, runtime, backlog                |
| Mapped METHOD tasks | M103                                           |

<hr />

## GH-82 - Echo store loader

Materialize nodes/edges payloads from parsed data.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as historical source until the mapped METHOD rows are also retired.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #82
GH issue createdAt: 2025-10-30T08:03:01Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #82                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:03:01Z                           |
| Updated at          | 2026-01-17T16:44:24Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/82 |
| Labels              | task, tooling, runtime, backlog                |
| Mapped METHOD tasks | M103                                           |

<hr />

## GH-83 - Integrity verification

Check BLAKE3 digests; fail on mismatch.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as historical source until the mapped METHOD rows are also retired.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #83
GH issue createdAt: 2025-10-30T08:03:07Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #83                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:03:07Z                           |
| Updated at          | 2026-01-17T16:44:59Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/83 |
| Labels              | task, tooling, runtime, backlog                |
| Mapped METHOD tasks | M103                                           |

<hr />

## GH-84 - Sample + tests

Add tiny sample bundle and unit tests.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as historical source until the mapped METHOD rows are also retired.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #84
GH issue createdAt: 2025-10-30T08:03:13Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #84                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:03:13Z                           |
| Updated at          | 2026-01-17T16:44:51Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/84 |
| Labels              | task, tooling, runtime, backlog                |
| Mapped METHOD tasks | M103                                           |

<hr />

## GH-85 - Draft C ABI spec

docs/spec-plugin-abi-c.md with lifecycle + capabilities.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 5; upstream max 1
GH issue #: #85
GH issue createdAt: 2025-10-30T08:03:19Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #85                                            |
| State               | OPEN                                           |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:03:19Z                           |
| Updated at          | 2025-11-02T01:22:34Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/85 |
| Labels              | task, runtime, backlog                         |
| Mapped METHOD tasks | M088                                           |

<hr />

## GH-86 - C header + host loader

Provide header; load/unload minimal plugin.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as historical source until the mapped METHOD rows are also retired.

DAG blocked by: M088 (Draft C ABI Spec (#85))
DAG chain depth: downstream max 4; upstream max 2
GH issue #: #86
GH issue createdAt: 2025-10-30T08:03:23Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #86                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:03:23Z                           |
| Updated at          | 2026-03-04T01:45:25Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/86 |
| Labels              | task, runtime, backlog                         |
| Mapped METHOD tasks | M089                                           |

<hr />

## GH-87 - Version negotiation

Implement and test version handshake.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as historical source until the mapped METHOD rows are also retired.

DAG blocked by: M089 (C Header + Host Loader (#86))
DAG chain depth: downstream max 3; upstream max 3
GH issue #: #87
GH issue createdAt: 2025-10-30T08:03:28Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #87                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:03:28Z                           |
| Updated at          | 2026-03-04T01:45:27Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/87 |
| Labels              | task, runtime, backlog                         |
| Mapped METHOD tasks | M090                                           |

<hr />

## GH-88 - Capability tokens

Deny registration without required tokens.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as historical source until the mapped METHOD rows are also retired.

DAG blocked by: M090 (Version Negotiation (#87))
DAG chain depth: downstream max 2; upstream max 4
GH issue #: #88
GH issue createdAt: 2025-10-30T08:03:32Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #88                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:03:32Z                           |
| Updated at          | 2026-03-04T01:45:29Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/88 |
| Labels              | task, runtime, backlog                         |
| Mapped METHOD tasks | M091                                           |

<hr />

## GH-89 - Example plugin + tests

Build toy plugin and CI test.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep as historical source until the mapped METHOD rows are also retired.

DAG blocked by: M091 (Capability Tokens (#88))
DAG chain depth: downstream max 1; upstream max 5
GH issue #: #89
GH issue createdAt: 2025-10-30T08:03:36Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #89                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T08:03:36Z                           |
| Updated at          | 2026-03-04T01:45:31Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/89 |
| Labels              | task, runtime, backlog                         |
| Mapped METHOD tasks | M092                                           |

<hr />

## GH-91 - Tests: Golden motion fixtures (JSON)

Add JSON golden fixtures and minimal harness for motion rule; ensure bytes round-trip and positions match expected component-wise using to_bits() equality.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #91
GH issue createdAt: 2025-10-30T09:34:43Z

| Field               | Value                                          |
| ------------------- | ---------------------------------------------- |
| Source              | GitHub issue                                   |
| GH issue #          | #91                                            |
| State               | CLOSED                                         |
| Author              | flyingrobots                                   |
| Created at          | 2025-10-30T09:34:43Z                           |
| Updated at          | 2025-11-02T01:00:24Z                           |
| URL                 | https://github.com/flyingrobots/echo/issues/91 |
| Labels              | task, tooling                                  |
| Mapped METHOD tasks | none                                           |

<hr />

## GH-103 - Policy: Require PR↔Issue linkage and 'Closes #…' in PRs

Adopt a repo-wide policy that every PR must be tied to a GitHub Issue, and PR descriptions must include a closing keyword to auto-close the issue on merge.\n\nScope\n- Update AGENTS.md with the policy and process.\n- Ensure future PRs use 'Closes #<id>' in the body.\n- Encourage 1 PR = 1 thing; open an issue first if one does not exist.\n\nAcceptance\n- AGENTS.md updated with the new section.\n- Announce in next PR description; optional: add a PR checklist reminder.\n

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #103
GH issue createdAt: 2025-11-02T00:55:17Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #103                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-02T00:55:17Z                            |
| Updated at          | 2026-01-17T14:25:15Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/103 |
| Labels              | task, spec                                      |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-105 - M2.0 — Scalar Foundation (umbrella)

Umbrella for deterministic numeric layer. Tracks:

- Scalar trait and backends
- Deterministic transcendentals (LUT)
- Motion rule port + v2 payload (Q32.32)
- Feature gates and CI lanes
- Lint guard against raw libm
- Cross‑platform determinism tests

Acceptance

- det_fixed and det_float lanes green across CI (glibc, musl, macOS)
- Motion rule uses Scalar; v2 payload with dual decode
- LUT tables checked in; generator pinned to rust 1.90.0
- forbid raw libm passes in engine crates

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #105
GH issue createdAt: 2025-11-02T06:55:56Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #105                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-02T06:55:56Z                            |
| Updated at          | 2026-01-01T18:49:11Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/105 |
| Labels              | feature, runtime, math, core                    |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-106 - Scalar trait and backends: F32Scalar + DFix64 (Q32.32) with unit tests

Define `Scalar` trait (core ops, transcendentals via trait fns) and implement:

- `F32Scalar` (deterministic wrappers, canonicalize -0.0, flush subnormals)
- `DFix64` (Q32.32: i64 with i128 accumulators; nearest‑even rounding; saturating overflow in release)

Deliver:

- Unit tests for basic ops and conversions
- Feature flags to compile either backend

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #106
GH issue createdAt: 2025-11-02T06:55:58Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #106                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-02T06:55:58Z                            |
| Updated at          | 2026-01-01T18:26:51Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/106 |
| Labels              | task, runtime, math, core                       |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-107 - Deterministic sin/cos wrappers + LUT generator (tables checked in)

Implement deterministic `sin`/`cos` via LUT+refinement.

- LUT generator pinned to Rust 1.90.0
- Tables checked into repo
- Error bounds documented and tested

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #107
GH issue createdAt: 2025-11-02T06:55:59Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #107                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-02T06:55:59Z                            |
| Updated at          | 2026-01-01T17:59:54Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/107 |
| Labels              | task, tooling, runtime, math, core              |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-108 - Motion rule → Scalar; v2 payload (6×i64 Q32.32), dual decode v1/v2

Port motion rule to use `Scalar`. Introduce payload v2 (Q32.32) with dual decode to preserve fixtures.

- Golden tests updated for v2; preserve v1 path for compatibility

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #108
GH issue createdAt: 2025-11-02T06:56:00Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #108                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-02T06:56:00Z                            |
| Updated at          | 2026-01-01T18:49:04Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/108 |
| Labels              | task, runtime, math, core                       |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-109 - Feature gates + CI lanes (det_fixed, det_float across glibc/musl/macOS)

Add feature flags and CI matrix lanes for both scalar backends across OS targets.

- Document how to run locally

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #109
GH issue createdAt: 2025-11-02T06:56:02Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #109                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-02T06:56:02Z                            |
| Updated at          | 2026-01-01T18:26:58Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/109 |
| Labels              | task, tooling, runtime, math, core              |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-110 - Lint guard: forbid raw libm in engine crates (math::scalar only)

Enforce that engine crates call deterministic wrappers only.

- Deny/CI check or custom lint pattern

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #110
GH issue createdAt: 2025-11-02T06:56:04Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #110                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-02T06:56:04Z                            |
| Updated at          | 2026-01-01T18:05:14Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/110 |
| Labels              | task, tooling, runtime, math, core              |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-111 - Cross‑platform determinism tests for Scalar ops

Add tests to validate bit‑stable results across glibc/musl/macOS for both backends.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #111
GH issue createdAt: 2025-11-02T06:56:06Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #111                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-02T06:56:06Z                            |
| Updated at          | 2026-01-01T18:27:06Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/111 |
| Labels              | task, tooling, runtime, math, core              |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-114 - task: `F32Scalar` and Wrapped Transcendentals

\### Parent Feature Issue

106

\### Steps

- [x] Define `F32Scalar` type
- [x] Deterministic wrappers around transcendentals:
    - [x] `sin`
    - [x] `cos`
- [x] Tests

\### Acceptance Criteria

- [x] `F32Scalar` exists
- [x] `sin` exists
- [x] `cos` exists
- [x] They are deterministic
- [x] Tests for everything

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #114
GH issue createdAt: 2025-11-03T03:33:11Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #114                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-03T03:33:11Z                            |
| Updated at          | 2025-11-30T01:36:53Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/114 |
| Labels              | task, runtime, math, core                       |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-115 - task: `Scalar` trait

\### Parent Feature Issue

106

\### Steps

Define `Scalar` trait

\### Acceptance Criteria

- [ ] Define `Scalar` triat
- [ ] Core operators
- [ ] Core transcendentals

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #115
GH issue createdAt: 2025-11-03T03:34:50Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #115                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-03T03:34:50Z                            |
| Updated at          | 2025-11-03T05:22:11Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/115 |
| Labels              | task, runtime, math, core                       |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-116 - task: `DFix64` type

\### Parent Feature Issue

106

\### Steps

Implementing `DFix64` type might be its own, separate epic task...

\### Acceptance Criteria

_No response_

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #116
GH issue createdAt: 2025-11-03T03:35:53Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #116                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-03T03:35:53Z                            |
| Updated at          | 2026-01-01T18:26:44Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/116 |
| Labels              | task, runtime, math, core                       |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-117 - task: Canonicalize `-0.0`

\### Parent Feature Issue

_No response_

\### Steps

**Canonicalize `-0.0`**: The `f32` standard has two zeros: `+0.0` and `-0.0`. They compare as equal `(-0.0 == 0.0)`, _but they have different bit patterns_.

Some arithmetic operations can produce `-0.0` (e.g., `-5.0 + 5.0`). To ensure determinism, you must "canonicalize" them by forcing any `-0.0` result to become `+0.0`.

A simple trick for this is result `+ 0.0`, which converts `-0.0` to `+0.0`.

\### Acceptance Criteria

- [ ] Tests demonstrate canonicalization of `-0.0` to `+0.0` for `Scalar` types.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #117
GH issue createdAt: 2025-11-03T03:40:03Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #117                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-03T03:40:03Z                            |
| Updated at          | 2025-11-30T21:04:52Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/117 |
| Labels              | task, runtime, math, core                       |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-118 - task: Flush subnormals

\### Parent Feature Issue

#106

\### Steps

**Subnormals** are incredibly small numbers very close to zero. On some hardware, calculating subnormals can be much slower or even handled in non-deterministic ways.

"Flushing subnormals" is a technique where you treat any number as `0.0` to maintain consistent performance and behavior.

\### Acceptance Criteria

- [ ] Tests exist which demonstrate that subnormals are treated as `0.0` for `Scalar` types

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #118
GH issue createdAt: 2025-11-03T03:44:09Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #118                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-03T03:44:09Z                            |
| Updated at          | 2025-12-01T05:41:10Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/118 |
| Labels              | task, runtime, math, core                       |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-122 - rmg-core: scheduler radix-drain performance tuning

This tracks performance work around the deterministic O(n) radix drain in the scheduler.\n\nAlready in PR #121:\n- Fail-fast drain (no silent drops) using `unreachable!` with `map_or_else`.\n- Keep histogram as `Vec<u32>` (65,536 buckets) with wrapping prefix-sum/scatter.\n- Use `usize` handle to avoid truncation casts.\n- Clippy pedantic cleanup; docs guard updates.\n\nFollow-ups (nice-to-have, gated by benches):\n- Validate `SMALL_SORT_THRESHOLD` (currently 1024) across distributions; tune if necessary.\n- Micro-ops: review branchless bucket extraction and pass ordering; consider precomputing pair indices.\n- Histogram zeroing: investigate partial reset strategies vs. full fill, keeping determinism.\n- Bench coverage: add skewed/clustered scope distributions and large-rule-id ranges.\n- Telemetry counters: optional scope for measuring pass counts and distribution characteristics (feature-gated).\n\nAcceptance: scheduler drain remains byte-lex consistent by (scope_hash, rule_id, nonce); benches show no regression; docs stay in sync.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #122
GH issue createdAt: 2025-11-06T10:52:21Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #122                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-06T10:52:21Z                            |
| Updated at          | 2025-11-30T06:19:00Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/122 |
| Labels              | runtime                                         |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-124 - warning: echo/Cargo.toml: `panic` setting is ignored for `bench` profile

\### Area

build

\### Steps to Reproduce

1. run `cargo check`
2. see warning

\### Expected Behavior

No warnings

\### Actual Behavior

```
warning: echo/Cargo.toml: `panic` setting is ignored for `bench` profile
```

\### Stack Trace / Error Logs

_No response_

\### Version / Commit

_No response_

\### Environment

- [ ] Linux (glibc)
- [ ] Linux (musl)
- [x] macOS (local)
- [ ] Other

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #124
GH issue createdAt: 2025-11-30T01:27:25Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #124                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-30T01:27:25Z                            |
| Updated at          | 2025-12-01T04:48:38Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/124 |
| Labels              | bug                                             |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-125 - Proper LICENSE

We have switched to Apache 2.0 for code
Apache 2.0 OR MIND-UCAL for everything that is not code

\### Steps

1. Ensure that all files have the correct Apache 2.0 OR MIND-UCAL license and SPDX header

\### Acceptance Criteria

- [x] All files have the SPDX header
- [x] CI/CD job that validates SPDX headers exists and works

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #125
GH issue createdAt: 2025-11-30T06:52:43Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #125                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-30T06:52:43Z                            |
| Updated at          | 2025-11-30T08:37:21Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/125 |
| Labels              | task                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-128 - feat: F32Scalar and Nan, -NaN

\### Problem / Motivation

1. Canonicalize -NaN to Nan
2. Ensure that reflexivity holds with x == x after canonicalizing NaN

\### Proposal / Scope

Just NaNs

\### Related Docs/Issues/PRs

#118, #117

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #128
GH issue createdAt: 2025-11-30T21:24:31Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #128                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-30T21:24:31Z                            |
| Updated at          | 2025-12-01T02:24:03Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/128 |
| Labels              | feature, math                                   |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-129 - feat: F32Scalar and "freaky numbers"

\### Problem / Motivation

1. Ensure that F32Scalar passes the "freaky numbers" test suite.

\### Proposal / Scope

Whatever is needed to pass the "freaky numbers" test suite.

\### Related Docs/Issues/PRs

#118, #117, #128

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #129
GH issue createdAt: 2025-11-30T21:26:02Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #129                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-30T21:26:02Z                            |
| Updated at          | 2026-01-01T18:00:43Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/129 |
| Labels              | feature, math                                   |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-130 - feat: Serialization for Scalar types

\## Problem / Motivation

Raw deserialization of floating-point values can bypass runtime invariants if not carefully guarded. For example, a serialized -0.0 or non-canonical NaN could
be loaded directly into memory, violating the strict determinism guarantees required for Echo's simulation loop. We must ensure that no path exists to
instantiate an F32Scalar without passing through the canonicalization logic.

\## Proposal / Scope

1. Guard Deserialization: Verify that the serde::Deserialize implementation for F32Scalar strictly routes all inputs through F32Scalar::new() to apply canonicalization (mapping -0.0 to +0.0, sanitizing NaNs).
2. Forbid Zerocopy: Explicitly ensure/document that F32Scalar does not implement zerocopy::FromBytes or similar traits that allow reinterpret casts from raw bytes.
3. Verify Guard: Enable and pass test_policy_serialization_guard in crates/rmg-core/tests/determinism_policy_tests.rs to prove that deserializing a "bad" payload (e.g., -0.0) results in a clean +0.0 instance in memory.

\## Related Docs/Issues/PRs

- [docs/SPEC_DETERMINISTIC_MATH.md](./docs/SPEC_DETERMINISTIC_MATH.md) (Policy 2: Zerocopy & Serialization)
- #118, #117, #128

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #130
GH issue createdAt: 2025-11-30T21:26:13Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #130                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-11-30T21:26:13Z                            |
| Updated at          | 2026-01-01T18:00:50Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/130 |
| Labels              | feature, math                                   |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-137 - task: write failing tests for sin_cos LUT

\### Parent Feature Issue

_No response_

\### Steps

Write tests for F32Scalar sin/cos LUT (failing)

\### Acceptance Criteria

_No response_

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #137
GH issue createdAt: 2025-12-01T07:32:51Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #137                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-12-01T07:32:51Z                            |
| Updated at          | 2026-01-01T17:59:46Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/137 |
| Labels              | task                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-142 - Rename legacy RMG naming to WARP + link AIΩN Foundations

Echo still contains legacy “RMG / recursive metagraph” naming drift that makes the repo harder to map onto the AIΩN Foundations (WARP Graphs) papers.

Backlog-driving goals:

- Rename `rmg-*` crates and public identifiers to `warp-*` / `Warp*`.
- Sweep docs/specs/book text to WARP-first terminology (“WARP graph”, plural “WARPs”).
- Update session proto/service/client/viewer + protocol docs to prefer `subscribe_warp` / `warp_stream` and `warp_id`.
- Keep a short decode-compat window for legacy `subscribe_rmg` / `rmg_stream` and `rmg_id`.
- Refresh README to link the AIΩN Framework repo and DOI links for Papers I–IV.
- Record any intentional deviations from the Foundations series in `docs/decision-log.md` + bridge note.

Acceptance:

- `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` are green.
- Only remaining `rmg_*` mentions are explicit compat/historical references.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #142
GH issue createdAt: 2025-12-28T22:59:11Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #142                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-12-28T22:59:11Z                            |
| Updated at          | 2025-12-28T23:14:04Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/142 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-144 - Promote AIΩN bridge doc + implement tick receipts

Backlog-driving docs + Paper II alignment.

Goals:

- Promote the AIΩN Foundations ↔ Echo bridge from `docs/notes/` into a canonical doc under `docs/` (keep a stub at the old path for historical links).
- Implement a minimal Paper II tick receipt structure in `warp-core`:
    - expose `Engine::commit_with_receipt` returning `(Snapshot, TickReceipt)`.
    - record accepted vs rejected candidates in canonical plan order.
    - commit to those outcomes via `Snapshot.decision_digest`.
- Document the canonical `decision_digest` encoding in `docs/spec-merkle-commit.md`.

Acceptance:

- `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings -D missing_docs` are green.
- Decision log and execution plan updated with rationale.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #144
GH issue createdAt: 2025-12-28T23:35:08Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #144                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-12-28T23:35:08Z                            |
| Updated at          | 2025-12-29T01:06:18Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/144 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-147 - TickReceipt: blocking causality (poset edges)

Paper II tick receipts describe a blocking causality poset: when a candidate rewrite is rejected due to footprint conflict, record which already-accepted candidates blocked it.

This issue tracks adding blocking attribution to `warp-core` tick receipts so tools can explain _why_ a candidate was rejected (not just that it was).

Scope:

- Extend `TickReceipt` to include a per-entry list of blocking predecessor indices (canonical plan order).
- Initially only for `Rejected(FootprintConflict)`; future rejection reasons can extend this.
- Keep `decision_digest` encoding stable (do not incorporate blockers into commit hashes yet).
- Add tests that exercise multi-blocker scenarios.
- Update docs (`docs/aion-papers-bridge.md`, `docs/spec-merkle-commit.md` notes, and decision log/execution plan).

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #147
GH issue createdAt: 2025-12-29T06:17:28Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #147                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-12-29T06:17:28Z                            |
| Updated at          | 2025-12-29T15:08:02Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/147 |
| Labels              | task, runtime, core                             |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-149 - WarpTickPatchV1 delta patches + commit hash v2

Delta Tick Patches (WarpTickPatchV1) + Commit Hash v2 (Paper III boundary semantics)

Goal

- Implement Paper III-style boundary artefacts: a per-tick _delta patch_ (not recipe) that is replayable without search.
- Move commit hashing to v2 so commit_id commits ONLY to the replayable delta (patch_digest), not to planner/scheduler narration.

Decisions (locked)

1. Patch stores unversioned slots only.
    - WarpTickPatchV1 includes in_slots/out_slots as sets of SlotId.
    - ValueVersionId := (slot_id, tick_index) is derived by interpretation along a worldline (position in payload P), not stored.
2. Commit hash v2 commits to patch_digest ONLY.
    - v2 commit header commits to parents + state_root + patch_digest (+ policy_id/version tags).
    - plan_digest / decision_digest / rewrites_digest / receipts / applied_rewrites are diagnostics only.

Implementation outline

- Add WarpTickPatchV1 + WarpOp minimal set to warp-core (delta ops + canonical encoding + patch_digest).
- Generate delta ops by deterministic diff of GraphStore pre/post commit (v0 correctness first).
- Conservative in/out derivation (over-approx OK; under-approx forbidden).
- Update Snapshot to carry patch_digest and com...

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #149
GH issue createdAt: 2025-12-29T13:11:30Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #149                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-12-29T13:11:30Z                            |
| Updated at          | 2025-12-29T15:30:51Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/149 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-151 - warp-core: parameterize policy_id (Aion policy semantics)

Today, warp-core hardcodes a placeholder policy id in commit hashing / tick patches.

- Define how policy_id is derived from the active Aion policy configuration.
- Thread it through Engine construction / commit path.
- Update specs and tests so commit_id commits to the correct policy boundary.

This issue tracks replacing the v0 placeholder with real policy semantics.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #151
GH issue createdAt: 2025-12-29T15:21:13Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #151                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-12-29T15:21:13Z                            |
| Updated at          | 2025-12-29T16:14:37Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/151 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-152 - warp-core: edge reverse index for tick patch replay

Tick patch replay currently needs to locate an existing edge by EdgeId when applying an UpsertEdge. Today this is implemented as an O(total_edges) scan across all outbound buckets.

Implement a reverse index in GraphStore (e.g., EdgeId -> from NodeId) so patch replay can migrate edges in O(bucket_edges) instead of O(total_edges).

Acceptance:

- No behavioral change for state_root / commit_id semantics.
- Clippy/tests/rustdoc gates remain green.
- Tick patch apply no longer performs a full scan for UpsertEdge.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #152
GH issue createdAt: 2025-12-29T15:39:30Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #152                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-12-29T15:39:30Z                            |
| Updated at          | 2025-12-29T16:14:37Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/152 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-154 - warp-core: derive Eq for EdgeRecord (remove edge_record_eq helper)

Follow-up cleanup:

- Derive `PartialEq` + `Eq` on `EdgeRecord`.
- Replace `edge_record_eq(a, b)` helper usage in `tick_patch` with idiomatic `a == b` / `a != b`.
- Delete the now-unused helper.

Rationale: avoid drift between helper-based equality and the struct definition; keep comparisons single-source-of-truth.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #154
GH issue createdAt: 2025-12-29T16:20:52Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #154                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-12-29T16:20:52Z                            |
| Updated at          | 2025-12-29T16:24:48Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/154 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-156 - warp-core: tick_patch hygiene (dedup ops, thiserror, docs)

Follow-up cleanup in `crates/warp-core/src/tick_patch.rs`:

- Avoid double map lookups in `diff_store` and expand rustdoc (intent/invariants/semantics/edge cases/perf).
- Use `thiserror` for `TickPatchError`.
- Document that `encode_ops` tag bytes are distinct from replay ordering (`WarpOp::sort_key`).
- Dedupe duplicate ops in `WarpTickPatchV1::new` to prevent replay errors.
- Alias `crate::ident::Hash` to `ContentHash` to avoid confusion with `derive(Hash)` on `SlotId`.

This is mechanical hygiene to keep the tick patch boundary deterministic and harder to misuse.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #156
GH issue createdAt: 2025-12-29T16:31:30Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #156                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-12-29T16:31:30Z                            |
| Updated at          | 2025-12-29T16:43:46Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/156 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-158 - feat: WarpInstances + descended attachments (WARP Stage B1)

Implements Stage B1: descended attachments as flattened indirection via WarpInstances.

Acceptance:

- AttachmentValue supports Descend(WarpId)
- Instance-scoped identity (WarpId/NodeKey/EdgeKey)
- Descent-chain footprinting (reads portal chain)
- state_root reachability follows Descend across instances
- tick patch ops + patch_digest updated for instances/attachments
- Paper III slice closure includes portal chain

Docs:

- ADR-0002 WarpInstances + Descended Attachments
- SPEC-0002 Descended Attachments v1
- spec-warp-tick-patch.md v2
- spec-merkle-commit.md updated for multi-instance state_root

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #158
GH issue createdAt: 2025-12-30T05:11:27Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #158                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-12-30T05:11:27Z                            |
| Updated at          | 2025-12-30T11:45:31Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/158 |
| Labels              | feature, runtime, core                          |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-161 - Add THEORY.md with AIΩN paper alignment callouts

Track adding a WARP Foundations paraphrase doc (Papers I–IV) with explicit "Echo does this differently" callouts, and commit the echo-white-radial SVG asset (remove local-only assets/readme scratch).

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #161
GH issue createdAt: 2025-12-30T13:16:20Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #161                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-12-30T13:16:20Z                            |
| Updated at          | 2025-12-30T16:25:20Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/161 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-163 - WVP demo path: bidirectional client + viewer publish/subscribe toggles

Complete the WARP View Protocol (WVP) demo path from docs/tasks.md.

Deliverables

- echo-session-client: add a bidirectional/tool-friendly connection that supports outbound publish (send Message::WarpStream) while preserving the existing receive-only API.
- warp-viewer: implement publish/subscribe toggles in UI, plus a minimal deterministic local mutation path that marks the graph dirty and publishes snapshot/diff frames when publishing is enabled.
- Demo: doc/script for running 1 session hub + 2 viewers (publisher + subscriber) and observing shared WARP updates.
- Tests: protocol conformance (authority rejection, gapless diff enforcement, publish loop behavior).

Acceptance

- One viewer can publish and another can subscribe to see updates.
- No attachment decoding in any hot path (viewer publishes ops; no deep decode required).
- Errors from hub (forbidden publish / epoch gap / snapshot required) surface to the tool as notifications.

Refs

- docs/spec-warp-view-protocol.md
- docs/tasks.md (WARP View Protocol Tasks)

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #163
GH issue createdAt: 2025-12-30T18:26:10Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #163                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2025-12-30T18:26:10Z                            |
| Updated at          | 2025-12-30T21:59:10Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/163 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-165 - PR: merge deterministic math milestone (M2.0 Scalar Foundation)

This issue exists solely to satisfy the repo linkage policy for the PR that merges the deterministic-math milestone work currently on `F32Scalar/sin-cos`.

\### Scope

- Open a PR from the branch tip to `main`.
- Ensure PR body includes `Closes #<this-issue-number>`.
- Follow the CodeRabbit review loop; do **not** merge via admin bypass.

\### Major deliverables in the branch

- Deterministic `F32Scalar` trig via LUT + guardrails.
- `det_fixed` backend lane (`DFix64`) + CI lanes (glibc/musl/macOS).
- Motion payload v2: Q32.32 (6×i64 LE) with dual decode (v0/v2) and executor ported to `Scalar`.

\### CodeRabbit workflow reminder

After CodeRabbit posts review comments, follow `/Users/james/git/jitos/docs/procedures/EXTRACT-PR-COMMENTS.md` to extract actionable comments, batch-fix, and iterate until approvals.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #165
GH issue createdAt: 2026-01-01T19:21:49Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #165                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-01T19:21:49Z                            |
| Updated at          | 2026-01-02T15:49:14Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/165 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-166 - TT0: Time model spec lock (TimeStreams + admission digests)

Context\n- We are formalizing time travel semantics for Echo (multi-stream time + pause/rewind/buffer/catch-up).\n- We need the spec + digest pins to be merged so future implementation work has a stable contract.\n\nGoal\n- Land the TimeStreams/Wormholes/admission decisions spec changes and align commit/tick digest docs so replay/audit stays truthful.\n\nScope\n- [ ] Keep as the canonical time model spec (HostTime vs HistoryTime, TimeStreams/Cursors, StreamAdmissionDecision).\n- [ ] Keep pinned as HistoryTime via (diagnostic lane, not in v2).\n- [ ] Ensure the related specs stay consistent:\n - ()\n - (admission decisions are non-canonical metadata, pinned via )\n - (planned )\n - (decision recorded)\n\nExit Criteria\n- [ ] Specs above are consistent and merged to .\n- [ ] ROADMAP includes TT0 and this issue is linked from the roadmap Issue Table.\n\nLinks\n- \n- \n- \n- \n

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #166
GH issue createdAt: 2026-01-01T19:22:26Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #166                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-01T19:22:26Z                            |
| Updated at          | 2026-01-03T01:30:22Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/166 |
| Labels              | documentation, spec                             |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-168 - T2: Embedded tooling UI baseline (Open Props + screenshot regen)

Context

- We want embedded dashboards that require no web build step: run a binary, open a page.
- Tooling UIs should share a token-based CSS baseline and keep docs screenshots honest via Playwright.

Goal

- Establish the baseline embedded UI theme/asset pattern so future tooling dashboards are consistent and offline-friendly.

Scope

- Vendor Open Props CSS assets for embedded dashboards and serve them from binaries (offline).
- Ensure the session dashboard uses the baseline tokens (cards/nav/modals/toasts, light/dark).
- Ensure Playwright can regenerate a dashboard screenshot for docs.

Exit Criteria

- pnpm exec playwright test green.
- Embedded dashboard uses vendored tokens and the theme is consistent.
- Docs include instructions to regenerate screenshots.

Links

- docs/guide/wvp-demo.md
- crates/echo-session-ws-gateway/assets/dashboard.html
- e2e/session-dashboard.spec.ts

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #168
GH issue createdAt: 2026-01-01T19:24:35Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #168                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-01T19:24:35Z                            |
| Updated at          | 2026-01-02T18:38:21Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/168 |
| Labels              | documentation, feature, tooling                 |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-169 - T1: WVP demo hardening (conformance + loopback integration tests)

Context

- WARP View Protocol (WVP) demo is the "run a hub + two tools" story.
- The demo path is useful only if regressions are caught automatically.

Goal

- Add test coverage that proves the WVP demo path remains correct: authority, gapless epochs, and multi-client loopback.

Scope

- Conformance tests for:
    - authority rejection (non-owner publish rejected)
    - gapless epoch enforcement
    - dirty-loop behavior (snapshot then diffs)
    - publish/subscribe toggle respect
- Integration test: start server + two clients (publisher/subscriber) loopback; verify subscriber observes expected progress.
- Keep tests deterministic (avoid wall-clock assertions).

Exit Criteria

- docs/tasks.md WVP tasks checkbox is fully checked (tests item complete).
- CI/local run includes these tests and they are green.

Links

- docs/tasks.md
- docs/guide/wvp-demo.md

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #169
GH issue createdAt: 2026-01-01T19:24:36Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #169                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-01T19:24:36Z                            |
| Updated at          | 2026-01-01T22:49:51Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/169 |
| Labels              | feature, tooling                                |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-170 - TT1: StreamsFrame inspector support (backlog + cursors + admission decisions)

Context

- Time travel requires visibility into streams (NetRx/GameInput/ToolInput/etc), their backlogs, and what has been admitted into a given view/worldline.
- We already specified the concepts and want a concrete inspector frame next.

Goal

- Implement a StreamsFrame inspector payload and emit it deterministically so tooling can display backlog/cursors/admission decisions.

Scope

- Define the StreamsFrame payload shape (stream ids, backlog counts, per-view cursors, recent StreamAdmissionDecision summaries + digests).
- Emit StreamsFrame post timeline_flush in stable order.
- Keep the stream read-only and capability-gated like other inspector frames.

Exit Criteria

- A live run shows StreamsFrame updates.
- A replay can reproduce the same StreamsFrame sequence from history.

Links

- docs/spec-editor-and-inspector.md
- docs/spec-time-streams-and-wormholes.md

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M173 (Spec — dt policy: fixed timestep vs admitted dt stream (#243)), M174 (Spec — TimeStream retention, spool compaction, wormhole density (#244)), M175 (Spec — Merge semantics for admitted stream facts across worldlines (#245)), M176 (Spec — Security/capabilities for fork/rewind/merge in multiplayer (#246))
DAG chain depth: downstream max 6; upstream max 5
GH issue #: #170
GH issue createdAt: 2026-01-01T19:24:38Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #170                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-01T19:24:38Z                            |
| Updated at          | 2026-01-01T19:24:38Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/170 |
| Labels              | feature, tooling, runtime                       |
| Mapped METHOD tasks | M177                                            |

<hr />

## GH-171 - TT2: Time Travel MVP (pause/rewind/buffer/catch-up)

Context

- We have a spec-level model for multi-stream time + admission decisions + wormholes/checkpoints.
- Next we need an MVP that makes time travel real (not just theoretical).

Goal

- Implement a minimal time travel workflow:
    - pause simulation view while tools stay live
    - rewind/fork simulation view
    - buffer network input without admitting while rewound
    - catch-up via checkpoint/wormhole or explicit resync/merge

Scope

- Define the minimal runtime API surface for pause/rewind/fork.
- Implement buffering semantics for NetRx under pause-buffer.
- Ensure admission decisions are recorded or integrity-pinned so replay/audit stays truthful.
- Provide a demo harness showing the workflow end-to-end (local first).

Exit Criteria

- A user can rewind locally while network keeps spooling, then either catch up or resync without paradox.
- Tooling/inspector can show the backlog and admission decisions (depends on TT1).

Links

- docs/spec-time-streams-and-wormholes.md
- docs/spec-networking.md

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M175 (Spec — Merge semantics for admitted stream facts across worldlines (#245)), M176 (Spec — Security/capabilities for fork/rewind/merge in multiplayer (#246)), M177 (Implement StreamsFrame inspector support (#170))
DAG chain depth: downstream max 5; upstream max 6
GH issue #: #171
GH issue createdAt: 2026-01-01T19:24:40Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #171                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-01T19:24:40Z                            |
| Updated at          | 2026-01-01T19:24:40Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/171 |
| Labels              | feature, runtime                                |
| Mapped METHOD tasks | M148                                            |

<hr />

## GH-172 - TT3: Rulial diff / worldline compare MVP

Context

- Once runs are deterministic and history is digest-pinned, the next high-leverage tool is "compare two runs" (first divergence, per-tick diffs).

Goal

- Build a minimal worldline compare tool: side-by-side scrubber + per-tick delta view + jump to first divergence.

Scope

- Define an export format (or reuse existing logs) that includes tick ids, patch digests, admission digests, and key receipts.
- Implement diff alignment + first divergence detection.
- Create a thin UI prototype that renders the comparison.

Exit Criteria

- Two runs can be compared deterministically and the tool can explain divergence sites.

Links

- docs/spec-timecube.md
- docs/spec-time-streams-and-wormholes.md

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M148 (Implement time travel core — pause/rewind/buffer/catch-up (#171)), M149 (Implement Reliving debugger MVP — scrub timeline + causal slice + fork branch (#205))
DAG chain depth: downstream max 3; upstream max 8
GH issue #: #172
GH issue createdAt: 2026-01-01T19:24:41Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #172                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-01T19:24:41Z                            |
| Updated at          | 2026-01-01T19:24:41Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/172 |
| Labels              | feature, tooling                                |
| Mapped METHOD tasks | M142                                            |

<hr />

## GH-173 - S1: Deterministic Rhai surface (sandbox + claims/effects)

Context

- Rhai is the intended authoring layer, but it must not punch holes in determinism (no HostTime/IO without Views/claims).

Goal

- Define and implement a minimal deterministic Rhai embedding surface that routes side effects through Echo Views.

Scope

- Define feature mask / allowed Rhai features for determinism.
- Define host modules as Views (clock, rng, kv, net emit) that produce replay-safe claims/decision records.
- Optional: fiber model (yield/await with claim/effect receipts).

Exit Criteria

- A simple Rhai script can run in the sandbox and produce deterministic state changes + receipts.

Links

- docs/spec-concurrency-and-authoring.md
- docs/spec-time-streams-and-wormholes.md

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M085 (Rhai Sandbox Configuration (#173, part a))
DAG chain depth: downstream max 2; upstream max 2
GH issue #: #173
GH issue createdAt: 2026-01-01T19:24:43Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #173                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-01T19:24:43Z                            |
| Updated at          | 2026-01-17T16:47:15Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/173 |
| Labels              | feature, spec, runtime                          |
| Mapped METHOD tasks | M085, M086                                      |

<hr />

## GH-174 - W1: Wesley as a boundary grammar (hashable view artifacts)

Context

- Treat Wesley as a language/spec for views (portable, hashable artifacts), not as a mandatory service.
- Determinism requires schema/IR pinning so old logs are not reinterpreted under new compilers.

Goal

- Deliver Wesley V0 as a boundary grammar with canonical AST + logical plan and hash pins that can be compiled to targets (PG/Echo).

Scope

- Define the grammar surface (select/where/join/params; minimal first).
- Implement canonical AST normalization + stable hashing.
- Define a schema hash pinning strategy for receipts/events (wesley_ir_schema_hash or equivalent).
- Add at least one target backend (PG mode first), plus an explain mode.

Exit Criteria

- View specs/plans are stable, hashable artifacts and can be verified during replay/audit.

Links

- docs/capability-ownership-matrix.md

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 3; upstream max 1
GH issue #: #174
GH issue createdAt: 2026-01-01T19:24:45Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #174                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-01T19:24:45Z                            |
| Updated at          | 2026-01-01T19:24:45Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/174 |
| Labels              | feature, spec                                   |
| Mapped METHOD tasks | M132                                            |

<hr />

## GH-177 - Deterministic trig: pin error budget + deterministic oracle for audit test

Background:

- `crates/warp-core/tests/deterministic_sin_cos_tests.rs` contains `test_sin_cos_error_budget_wip`, currently `#[ignore]` because it compares against platform libm-derived f64 trig.

Goal:

- Replace the reference/oracle with a deterministic one (or a pinned golden-vector oracle) and establish an explicit, documented error budget for the LUT-backed `warp_core::math::trig` backend.

Exit criteria:

- Define and document acceptable max ULP / absolute error for `sin` and `cos` over a specified domain (e.g. [-2*TAU, 2*TAU]).
- Implement deterministic measurement that does not depend on host libm (e.g. precomputed vectors or a deterministic polynomial reference).
- Un-ignore the test (or split into deterministic golden + optional audit).

Notes:

- Determinism requirement: `sin(-x)` must be the exact bitwise negation of `sin(x)`; `cos(-x)` must match `cos(x)`.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #177
GH issue createdAt: 2026-01-01T20:58:15Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #177                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-01T20:58:15Z                            |
| Updated at          | 2026-01-02T15:57:58Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/177 |
| Labels              | task, runtime, math, core                       |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-180 - Docs: Paper VI time+determinism notes + capability ownership matrix

Goal

- Capture near-term Paper VI (aion-paper-06) notes on determinism/time: HostTime vs HistoryTime, Decision Records, multi-clock/stream model, pause/catch-up/merge, and how external stimuli become graph rewrites.
- Add an Echo-side Capability Ownership Matrix doc to keep ownership/determinism/provenance boundaries crisp.

Deliverables

- [ ] /Users/james/git/aion-paper-06/<new>.md: Paper VI notes distilled from recent Pulse articles + our discussion (HistoryTime/HostTime, multi-clock worldlines, wormholes/catch-up).
- [ ] docs/capability-ownership-matrix.md: Matrix template + first-pass fill for Echo stack (Platform/Kernel/Views/Tooling/Docs).

Acceptance

- Notes are concrete enough to seed Paper VI sections and figures.
- Matrix is actionable: each cell includes role/stability/determinism/provenance/deps.

Refs

- Pulse: HistoryTime/HostTime decision-record rule
- Echo: determinism-first, provenance-first, time-travel/debug tooling requirements

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #180
GH issue createdAt: 2026-01-01T23:01:42Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #180                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-01T23:01:42Z                            |
| Updated at          | 2026-01-02T15:34:32Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/180 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-183 - Docs: align ROADMAP with GitHub milestone tracks

Goal

- Ensure GitHub milestones (M*, T*, TT*, S*, W\*) match the canonical roadmap docs and that the active/open milestone issues are listed in our roadmap tables.

Work

- [ ] Update `docs/ROADMAP.md` milestone list to include the existing tooling/time-travel milestone tracks (T2, TT0–TT3, S1, W1) with the same targets as the GitHub milestones.
- [ ] Update `docs/ROADMAP.md` Issue Table to include the active/open issues for those milestones (e.g. #166, #168, #170–#174) and any missing active issues in existing milestones (e.g. #177 under M4).
- [ ] Update `docs/ISSUES_MATRIX.md` similarly so it mirrors active issues in Project 9.

Acceptance

- Roadmap docs mention every open GitHub milestone track we actively use.
- Every open issue that is assigned to a milestone is present in the ROADMAP/ISSUES_MATRIX tables (or explicitly called out as intentionally omitted).

Notes

- Keep this docs-only; do not rework runtime code.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #183
GH issue createdAt: 2026-01-02T16:02:06Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #183                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T16:02:06Z                            |
| Updated at          | 2026-01-03T01:33:52Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/183 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-185 - M1: Domain-separated hash contexts for core commitments (state_root/patch_digest/commit_id)

Pulse context: domain separation + belt-and-suspenders hashing. We are OK with breaking hashes (2026-01-02 discussion).

Goal

- Switch security-critical digests in `warp-core` to BLAKE3 derive-key contexts so the same bytes hashed under the wrong function cannot accidentally collide across domains.

Scope

- `state_root`: introduce explicit version tag (v1) and compute using derive-key context (e.g. `ECHO/warp-core/state_root/v1`).
- `patch_digest`: bump to v3 (hash mode change) and compute using derive-key context (e.g. `ECHO/warp-core/patch_digest/v3`).
- `commit_id`: bump to v3 and compute using derive-key context (e.g. `ECHO/warp-core/commit_hash/v3`).
- Centralize the domain strings and hasher constructors in one module to prevent future footguns.

Acceptance criteria

- Tests prove: hashing the same canonical byte stream under different domains yields different digests.
- Specs/docs updated to name the new versions and contexts.
- Any golden vectors/fixtures updated accordingly.

Notes

- This should land before we treat hash goldens as stable (M1).

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #185
GH issue createdAt: 2026-01-02T16:51:28Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #185                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T16:51:28Z                            |
| Updated at          | 2026-04-04T19:14:47Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/185 |
| Labels              | security, runtime, core                         |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-186 - M1: Domain-separated digest context for RenderGraph canonical bytes

Pulse context: domain separation in Merkle-style graph hashing.

Goal

- Ensure `echo-graph` RenderGraph digest is domain-separated (derive-key context) and versioned so it cannot be confused with other digests.

Scope

- Add/confirm an explicit encoding version tag in the canonical byte stream.
- Compute digest using a derive-key context (e.g. `ECHO/echo-graph/render_graph_digest/v1`).

Acceptance criteria

- Unit test: same bytes hashed with a different domain yields different digest.
- Any existing viewer/session code that relies on this digest updated accordingly.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #186
GH issue createdAt: 2026-01-02T16:51:30Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #186                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T16:51:30Z                            |
| Updated at          | 2026-04-04T19:14:49Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/186 |
| Labels              | tooling, security, core                         |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-187 - M4: Worldline convergence property suite (replay-from-patches converges)

Pulse context: temporal-algebraic determinism tests.

Goal

- Prove worldline convergence: same seed + admitted inputs + rule-pack => same terminal commitment, and replay from the recorded boundary artifacts reproduces the same worldline.

Scope

- Add a proptest-driven harness that:
    - runs an engine under a pinned seed to produce a sequence of tick patches (and/or receipts),
    - replays those patches into a fresh engine,
    - asserts per-step `patch_digest` sequence matches and terminal `state_root`/`commit_id` match.
- On mismatch, emit a minimal repro tuple (seed, inputs, schedule parameters) for deterministic re-run.

Acceptance criteria

- Test suite runs in CI (non-ignored) and is stable across OSes.
- Clear failure output with a repro seed.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep or import into METHOD. The GitHub issue is open but no METHOD row was found.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #187
GH issue createdAt: 2026-01-02T16:51:31Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #187                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T16:51:31Z                            |
| Updated at          | 2026-01-02T16:55:30Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/187 |
| Labels              | tooling, runtime, core                          |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-188 - M4: Kernel nondeterminism tripwires (forbid ambient HostTime/entropy sources)

Pulse context: kernel-grade determinism enforcement.

Goal

- Prevent accidental reintroduction of ambient nondeterminism into kernel crates (warp-core + closely coupled crates).

Scope

- Add CI/script checks analogous to the raw-trig guard that forbid:
    - direct host clock reads in kernel crates (e.g., `Instant::now`, `SystemTime::now`),
    - OS entropy / non-seeded RNG sources,
    - nondeterministic iteration traps (where relevant).
- Scope must be kernel-only (tools/adapters may use HostTime).

Acceptance criteria

- CI fails fast when forbidden patterns appear in kernel crates.
- Documentation describes the boundary: HostTime is telemetry only; semantics are ticks/epochs.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #188
GH issue createdAt: 2026-01-02T16:51:33Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #188                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T16:51:33Z                            |
| Updated at          | 2026-01-17T14:25:12Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/188 |
| Labels              | security, runtime, core                         |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-189 - M4: Concurrency litmus suite for scheduler determinism (overlap detection + canonical reduction)

Pulse context: LKMM/RCU-style litmus + stress, adapted to Echo.

Goal

- Back the claim that concurrency is “solved” by proving the scheduler’s overlap detection and reduction semantics are deterministic and correct.

Scope

- Add a set of minimal litmus scenarios that exercise:
    - overlapping footprints => cannot run concurrently / must be ordered,
    - disjoint footprints => results commute and converge,
    - deterministic reduction: any parallel combine step is stable/canonical.
- Use `loom` where it fits (interleavings), otherwise deterministic shims.

Acceptance criteria

- Tests cover both “should commute” and “must not commute” cases.
- Demonstrate equivalence between 1-thread reference execution and multi-thread execution for the litmus set.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #189
GH issue createdAt: 2026-01-02T16:51:35Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #189                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T16:51:35Z                            |
| Updated at          | 2026-01-17T14:25:09Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/189 |
| Labels              | runtime, core                                   |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-190 - M4: Determinism torture harness (1-thread vs N-thread + snapshot/restore fuzz)

Pulse context: rcutorture-style stress for determinism.

Goal

- Catch rare concurrency/time-travel regressions by running seeded stress workloads under varying internal concurrency and snapshot/restore points.

Scope

- Seeded generator produces workloads (rule applications, attachment edits, etc.).
- Run the same workload under:
    - 1 thread
    - N threads
    - randomized snapshot/restore injection points
- Assert terminal `state_root`/`commit_id` (and ideally per-step `patch_digest`) match.

Acceptance criteria

- Stable, deterministic repro output (seed + knobs).
- Can be configured for “quick CI” vs “deep local soak”.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep or import into METHOD. The GitHub issue is open but no METHOD row was found.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #190
GH issue createdAt: 2026-01-02T16:51:37Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #190                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T16:51:37Z                            |
| Updated at          | 2026-01-02T16:55:49Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/190 |
| Labels              | tooling, runtime, core                          |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-191 - TT0: Session stream time fields (HistoryTime ordering vs HostTime telemetry)

Decision (2026-01-02): kernel semantics are defined only in ticks/epochs (HistoryTime). HostTime never changes semantic state.

Goal

- Make the time model in session streams explicit: correctness uses HistoryTime fields; HostTime is telemetry only.

Scope

- Define/confirm canonical fields for stream correctness: `epoch`/`seq`/`tick`.
- If a host timestamp is present (e.g. `ts`), explicitly define it as HostTime telemetry and forbid using it for ordering/dedup/gap enforcement.
- Update docs/specs and add regression tests in session pipeline where applicable.

Acceptance criteria

- Spec clarifies ordering keys vs telemetry.
- Tests prove hash/digest outcomes are invariant to HostTime differences.

Parent

- Child of #166 (TT0).

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 10; upstream max 1
GH issue #: #191
GH issue createdAt: 2026-01-02T16:56:17Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #191                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T16:56:17Z                            |
| Updated at          | 2026-01-02T16:56:17Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/191 |
| Labels              | spec, tooling                                   |
| Mapped METHOD tasks | M049                                            |

<hr />

## GH-192 - TT0: TTL/deadline semantics are ticks/epochs only (no host-time semantic deadlines)

Decision (2026-01-02): kernel semantics are defined only in ticks/epochs.

Goal

- Remove ambiguity around TTL: if TTL affects semantics, it is expressed only in HistoryTime (ticks/epochs). HostTime-based TTL is allowed only as non-semantic retention policy.

Scope

- Define semantic TTL as `expires_at_tick` (or equivalent) in specs.
- Explicitly separate operational retention (tool/hub memory policy) from semantic expiry.

Acceptance criteria

- TT0 docs updated with crisp definitions and examples.
- Any future implementation work references this definition.

Parent

- Child of #166 (TT0).

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M049 (Spec — HistoryTime vs HostTime field classification (#191))
DAG chain depth: downstream max 9; upstream max 2
GH issue #: #192
GH issue createdAt: 2026-01-02T16:56:18Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #192                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T16:56:18Z                            |
| Updated at          | 2026-01-02T16:56:18Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/192 |
| Labels              | spec                                            |
| Mapped METHOD tasks | M050                                            |

<hr />

## GH-193 - W1: Schema hash chain pinning (SDL→IR→bundle) recorded in receipts

Pulse context: bi-directional compiler loop.

Goal

- Prevent drift by pinning schema artifacts via a hash chain (SDL hash → IR hash → codegen bundle hash), and recording those hashes in receipts/events.

Scope

- Define the hash boundaries and where they are recorded.
- Identify the minimal receipt/event surfaces that must include these pins.

Acceptance criteria

- Spec describes the hash chain and failure mode (fail closed when pins mismatch).
- Any prototype codegen bundle can report its bundle hash.

Parent

- Child of #174 (W1).

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M132 (Hashable View Artifacts (#174))
DAG chain depth: downstream max 2; upstream max 2
GH issue #: #193
GH issue createdAt: 2026-01-02T16:56:20Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #193                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T16:56:20Z                            |
| Updated at          | 2026-01-02T16:56:20Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/193 |
| Labels              | feature, spec                                   |
| Mapped METHOD tasks | M133                                            |

<hr />

## GH-194 - W1: SchemaDelta vocabulary (read-only MVP) + wesley patch dry-run plan

Pulse context: bidirectional SDL loop; runtime emits schema delta proposals.

Goal

- Define a minimal, review-only SchemaDelta vocabulary and a dry-run patch-plan flow so schema evolution is auditable and replay-safe.

Scope

- Define `SchemaDelta` enum (start with small safe set: `AddIndex`, `SetDefault`, etc.).
- Define `DeltaProvenance` (tick/epoch, rulepack id, module id, justification).
- Define `PatchPlan` as ordered/invertible edits with human notes.
- Specify `wesley patch --dry-run` output (exact SDL diff, no auto-apply).

Acceptance criteria

- Spec exists with concrete types and examples.
- No auto-mutation: deltas are proposals only until reviewed.

Parent

- Child of #174 (W1).

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #194
GH issue createdAt: 2026-01-02T16:56:22Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #194                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T16:56:22Z                            |
| Updated at          | 2026-01-02T16:56:22Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/194 |
| Labels              | feature, spec                                   |
| Mapped METHOD tasks | M134                                            |

<hr />

## GH-195 - Backlog: JS-ABI packet checksum v2 (domain-separated hasher context)

Pulse context: domain separation as a security footgun preventer.

Goal

- If/when we bump the JS-ABI wire protocol version, compute packet checksums using a derive-key context so packet checksums can never be confused with other BLAKE3 digests.

Scope

- Define checksum context string and version bump strategy.
- Keep v1 decode support (or document migration) as needed.

Notes

- Intentionally Backlog until we want to do a protocol-version bump.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #195
GH issue createdAt: 2026-01-02T16:56:24Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #195                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T16:56:24Z                            |
| Updated at          | 2026-01-02T16:56:24Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/195 |
| Labels              | tooling, security, backlog                      |
| Mapped METHOD tasks | M096                                            |

<hr />

## GH-196 - Docs: document issue dependency management via gh api

We track "blocked by"/"blocking" relationships using GitHub native issue dependencies.

Add a short, concrete guide to the repo docs showing how to list/add/remove dependencies using `gh api` (REST endpoints), since the GitHub CLI does not currently expose a stable first-class command for dependency mutation in this environment.

Acceptance criteria

- `docs/ISSUES_MATRIX.md` includes a "Managing Issue Dependencies" section.
- Includes copy/pasteable `gh api` commands for:
    - listing `blocked_by`
    - listing `blocking`
    - adding a blocked-by dependency
    - removing a blocked-by dependency

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #196
GH issue createdAt: 2026-01-02T17:02:46Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #196                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T17:02:46Z                            |
| Updated at          | 2026-01-03T01:13:46Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/196 |
| Labels              | documentation, tooling                          |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-198 - W1: Provenance as query semantics (tick directive + proof objects + deterministic cursors)

Pulse context: "provenance as query semantics".

Goal

- Treat Wesley queries as deterministic, time-addressed slices through a worldline.
- Make provenance/proof artifacts an explicit (opt-in) part of query semantics so outputs are replayable and explainable.

Scope

- Define a time-addressing directive (e.g. `@tick(at: Tick!)`) and what it means for compilation targets.
    - Semantics must be HistoryTime-only (ticks/epochs/commit ids). No HostTime semantic deadlines.
- Define proof output conventions:
    - Field-parallel proof objects (e.g. `__proof_<field>` or a uniform `__proof` field) that can return `tick`, `digest`, and optional `lineage` steps.
    - Proof modes: `brief` (tick + final digest), `lineage` (full chain), `cost` (plan/row counts for audits).
- Deterministic pagination:
    - Cursor shape includes `{tick, digest, stable_order_key}` so slices are stable across replays.
    - Prohibit/avoid offset pagination for replay-stability.
- Capability/security gating:
    - Proof emission must be opt-in and capability-gated to avoid leaking sensitive provenance.

Acceptance criteria

- Spec section (or new PROV-\* spec) defines:
    - directive syntax
    - proof object shapes
    - cursor...

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M133 (Schema Hash Chain Pinning (#193))
DAG chain depth: downstream max 1; upstream max 3
GH issue #: #198
GH issue createdAt: 2026-01-02T17:08:03Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #198                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T17:08:03Z                            |
| Updated at          | 2026-01-02T17:08:03Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/198 |
| Labels              | feature, spec                                   |
| Mapped METHOD tasks | M135                                            |

<hr />

## GH-199 - TT3: Wesley worldline diff (compare query outputs/proofs across ticks)

Pulse context: provenance-aware queries enable first-class worldline comparison.

Goal

- Provide a tool workflow to compare Wesley query results across two ticks (or tick ranges) and explain _why_ outputs changed.

Scope

- Define a CLI UX for something like:
    - `wesley diff --ticks T1..T2 --query <file.graphql> --vars <vars.json>`
- Output includes:
    - stable row/field-level diff
    - proof-step diff (which lineage steps changed)
    - pinned hashes for schema/IR/bundle if applicable
- Must be deterministic and replay-safe: same inputs + same tick range => same diff output.

Acceptance criteria

- Spec/notes describing expected output format and determinism requirements.
- Identify minimum provenance hooks required from W1 to power TT3.

Related

- This should lean on proof objects / cursors defined under W1.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M142 (Implement rulial diff / worldline compare MVP (#172))
DAG chain depth: downstream max 2; upstream max 9
GH issue #: #199
GH issue createdAt: 2026-01-02T17:08:29Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #199                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T17:08:29Z                            |
| Updated at          | 2026-01-02T17:08:29Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/199 |
| Labels              | spec, tooling                                   |
| Mapped METHOD tasks | M143                                            |

<hr />

## GH-200 - Automate dependency DAG refresh + document contributor workflows

Tracks dependency DAG generation + automation + contributor workflow docs.

Acceptance:

- `cargo xtask dags --snapshot-label none` regenerates deterministic DOT/SVG
- Scheduled workflow opens PR only when `docs/assets/dags/*` changes
- README links to workflow docs

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #200
GH issue createdAt: 2026-01-02T17:10:06Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #200                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T17:10:06Z                            |
| Updated at          | 2026-01-03T01:50:29Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/200 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-202 - Spec: Provenance Payload (PP) v1 (canonical envelope for artifact lineage + signatures)

Pulse context: RFC for a versioned "Provenance Payload" (PP) envelope shared across JITOS/Echo/Wesley.

Goal

- Define a minimal, portable provenance envelope that:
    - pins artifact content by hash (content_digest),
    - records lineage/parents + worldline/epoch references,
    - captures derivation metadata (tool/procedure/parameters),
    - supports signatures/attestations over canonical bytes.

Non-goals / constraints (Echo invariants)

- PP must not introduce HostTime-driven semantics into the kernel.
- Any host timestamps / environment observations are telemetry/audit only and must not affect `state_root` / `patch_digest` / `commit_id`.

Scope

- Specify PP v1 fields and canonicalization rules for:
    - JSON (JCS RFC 8785) and CBOR (dag-cbor-style canonical rules).
    - multihash representation (alg code + digest encoding) and how it maps onto Echo’s existing BLAKE3-based digests.
- Define stable `artifact_kind` values relevant to Echo/Wesley:
    - `warp_patch`, `warp_tick_receipt`, `wesley_query_plan`, `wesley_migration`, `rule_pack`, etc.
- Define deterministic subset vs audit-only fields:
    - deterministic core: content_digest, parents, epoch/worldline (HistoryTime), procedure + p...

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #202
GH issue createdAt: 2026-01-02T17:10:55Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #202                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T17:10:55Z                            |
| Updated at          | 2026-01-02T17:10:55Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/202 |
| Labels              | spec, tooling, security, backlog                |
| Mapped METHOD tasks | M097                                            |

<hr />

## GH-203 - TT1: Constraint Lens panel (admission/scheduler explain-why + counterfactual sliders)

Pulse context: "Constraint Lens" UX for explainable scheduling/admission + counterfactuals.

Goal

- Make tick admission/scheduler decisions debuggable at a glance by exposing:
    - why a step ran (predicates satisfied / constraints / scheduler choice)
    - what was blocked (alternatives)
    - simple counterfactual recomputation for a small set of knobs.

Scope (MVP)

- Read-only first: record an "admission trace" per tick/step:
    - predicate ids
    - rule-pack hash
    - resource snapshot (caps)
    - chosen scheduler rule + blocked alternatives
- Viewer panel (TT1) showing per-tick timeline with explain-why details.
- Counterfactual knobs (start with 2):
    - CPU cap
    - priority weight / scheduling policy selector
- Counterfactual recompute strategy:
    - recompute only affected prefix/suffix (cache partial folds) to keep UI responsive.

Metrics

- Decision Delta Coverage: % of constraints where a counterfactual flips at least one admission decision.
- Median Replay Latency for counterfactual prefix/suffix recompute.

Acceptance criteria

- Spec/notes: define admission trace schema + counterfactual request schema.
- Viewer panel renders read-only traces from logs.
- At least one knob chang...

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M177 (Implement StreamsFrame inspector support (#170))
DAG chain depth: downstream max 5; upstream max 6
GH issue #: #203
GH issue createdAt: 2026-01-02T17:12:19Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #203                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T17:12:19Z                            |
| Updated at          | 2026-03-04T03:34:05Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/203 |
| Labels              | feature, tooling                                |
| Mapped METHOD tasks | M178                                            |

<hr />

## GH-204 - TT3: Provenance heatmap (blast radius / cohesion over time)

Pulse context: "Provenance Heatmap" UX to visualize diffusion/blast radius of changes over ticks.

Goal

- Provide a visual that colors regions of the provenance/execution graph by cohesion (local vs diffuse effects) and supports scrubbing across ticks.

Scope (MVP)

- Define an effect-provenance capture format suitable for replay:
    - per-tick affected set (nodes/edges/attachments) + classification tags (config/data/code)
    - stable module/region mapping (initially heuristic or author-provided)
- Compute a simple Cohesion Index per region:
    - e.g. affected_inside_region / affected_total, smoothed over a window.
- Viewer integration:
    - time scrubber
    - filter by rule-pack id, change class, and region
- Precompute per-tick rollups to keep scrubbing <200ms.

Metrics

- Triage Time Δ (measured in drills initially).
- Blast-radius correlation (predicted vs actual diff footprint).

Acceptance criteria

- Spec/notes define data needed + cohesion formula.
- A demo dataset renders in the viewer with at least one detectable low-cohesion event.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M142 (Implement rulial diff / worldline compare MVP (#172)), M143 (Implement Wesley worldline diff — compare query outputs/proofs across ticks (#199))
DAG chain depth: downstream max 1; upstream max 10
GH issue #: #204
GH issue createdAt: 2026-01-02T17:12:26Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #204                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T17:12:26Z                            |
| Updated at          | 2026-01-02T17:12:26Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/204 |
| Labels              | feature, tooling                                |
| Mapped METHOD tasks | M144                                            |

<hr />

## GH-205 - TT2: Reliving debugger MVP (scrub timeline + causal slice + fork branch)

Pulse context: "reliving" debugger (scrubbable replay with causal slices + branching).

Goal

- Build a debugger UX where a run can be replayed like a movie:
    - scrub ticks
    - pause on a frame and see a minimal causal slice (why we’re here)
    - fork an alternate future from any tick.

Scope (MVP)

- Recorder
    - Wrap scheduler/engine to emit an append-only event DAG log (deterministic clocks; no HostTime semantics).
    - Each tick links to a content-addressed snapshot/commit and the boundary artifact(s) (`patch_digest`, receipts).
- Slicer
    - Compute a backward slice from a selected event/variable to its causal roots.
    - Surface minimal set of steps/inputs that explain the current state.
- Viewer UI
    - Timeline scrubber (ticks + bookmarks)
    - Causal pane (minimal graph)
    - State pane (structured diff + hover-to-origin)
    - "Fork here" action to create a sandbox rerun from tick T.

Ethics/security (spec-first)

- Provenance labels / policy tags for sensitive data.
- Redaction-at-rest + masked-by-default UI; unmask audited.
- Exported replays include explicit consent footprint.

Acceptance criteria

- Spec/notes define:
    - event DAG schema
    - slice algorithm outline
    - fork/r...

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M148 (Implement time travel core — pause/rewind/buffer/catch-up (#171)), M178 (Implement Constraint Lens panel — admission explain-why + counterfactual sliders (#203))
DAG chain depth: downstream max 4; upstream max 7
GH issue #: #205
GH issue createdAt: 2026-01-02T17:13:36Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #205                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T17:13:36Z                            |
| Updated at          | 2026-01-02T17:13:37Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/205 |
| Labels              | feature, tooling                                |
| Mapped METHOD tasks | M149                                            |

<hr />

## GH-206 - M2.1: DPO concurrency theorem coverage (critical pair / rule composition litmus tests)

Pulse context: adhesive/quasi-adhesive category foundations for DPO rewriting + concurrency theorem / rule composition.

Goal

- Strengthen Echo’s determinism + concurrency claims by grounding and testing DPO-style rule composition/independence properties:
    - commuting (parallel independent) rule applications
    - conflicting overlaps (critical pairs)
    - composite/concurrent rule reasoning as executable tests.

Scope

- Spec work:
    - Document the assumed ambient category conditions (pragmatically: typed directed graphs behave like an adhesive setting for the rewrite operations Echo uses).
    - Map Echo/WARP concepts (footprints, in/out slots, dangling-edge prevention) to DPO conditions (gluing/dangling) at the level we need.
- Tests (litmus set):
    - Add a minimal suite of rule-pair scenarios that:
        - commute (apply A then B == B then A) and match terminal digests.
        - conflict (non-commuting overlap) and demonstrate deterministic resolution (reject/serialize or conflict resolver).
    - Where possible, express expected behavior using critical-pair style classification (independent vs dependent).

Acceptance criteria

- A spec note explains which parts of the DPO concurrency st...

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #206
GH issue createdAt: 2026-01-02T17:15:48Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #206                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T17:15:48Z                            |
| Updated at          | 2026-01-03T07:41:27Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/206 |
| Labels              | spec, runtime, core                             |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-207 - Backlog: Run noisy-line test for naming (Echo / WARP / Wesley / Engram)

Pulse context: "noisy-line test" for brand/name clarity in spoken word.

Context

- "Echo" is the game engine built on WARP graphs.
- "WARP" is the deterministic graph rewrite substrate.
- "Wesley" is the boundary grammar / query surface.
- "Engram" is a candidate name for an adjacent project: the OS/canonical implementation of the JITOS concept from Paper VI (i.e., using "Engram" as the public name instead of "JITOS").

Goal

- Validate that "Engram" travels reliably over speech and doesn’t mutate into unsearchable variants.

Scope

- Run a 5-person noisy-line test for "Engram" (diverse accents/backgrounds).
- Record results: exact spellings received and score (e.g. 5/5).
- If results are shaky, decide on:
    - a phonetic bumper ("Engram — like en-gram")
    - a subtitle/tagline that anchors meaning (e.g., "Engram — deterministic OS for replayable compute")
    - helper domains/handles that match common mishearings.

Acceptance criteria

- Notes recorded in `docs/decision-log.md` with result counts + any action taken.

Notes

- Backlog/optional: do before any public-facing pushes (talks/podcasts/docs refresh).

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #207
GH issue createdAt: 2026-01-02T17:19:18Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #207                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T17:19:18Z                            |
| Updated at          | 2026-01-02T17:22:08Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/207 |
| Labels              | documentation, backlog                          |
| Mapped METHOD tasks | M111                                            |

<hr />

## GH-208 - Docs audit: refresh math validation plan + purge/merge/splurge memo

This issue tracks a documentation audit pass to reduce drift and improve discoverability.

Scope:

- Refresh `docs/math-validation-plan.md` to reflect the current Rust `warp-core` deterministic math implementation and CI lanes.
- Add a short `docs/docs-audit.md` memo listing purge/merge/splurge candidates.
- Fix obviously stale/broken docs landing links (VitePress `docs/index.md`).
- Clarify which deterministic-math docs are normative vs legacy.

Acceptance:

- `docs/math-validation-plan.md` references real tests/commands and current CI lanes.
- `docs/docs-audit.md` exists and lists next candidates.
- No runtime behavior changes; docs-only PR.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #208
GH issue createdAt: 2026-01-02T17:32:20Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #208                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T17:32:20Z                            |
| Updated at          | 2026-01-03T01:14:53Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/208 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-210 - Docs: consolidate scheduler docs (warp-core vs Echo system scheduler)

Scheduler docs currently mix two concepts: the Rust `warp-core` deterministic rewrite scheduler (reserve/drain ordering) and the future Echo ECS/system scheduler spec.

Scope:

- Add a single landing doc that explains the two scheduler layers and links to canonical specs.
- Update scheduler docs (`spec-scheduler.md`, `scheduler-benchmarks.md`, `scheduler-reserve-*.md`) to clearly state scope/status and link back to the landing doc.
- Keep docs accurate: refer to actual code/tests/benches, avoid stale line-number citations.

Acceptance:

- Contributors can answer “which scheduler doc do I read?” in <60s.
- No misleading claims that TS system scheduler is implemented today.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #210
GH issue createdAt: 2026-01-02T17:39:22Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #210                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T17:39:22Z                            |
| Updated at          | 2026-01-03T01:54:58Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/210 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-212 - Upgrade VitePress (docs site) to modern release

Docs site is currently pinned to `vitepress@0.1.1`, which fails on modern Node (e.g. Node 25) due to removed `fs.promises.rmdir({ recursive: true })` behavior.

Upgrade VitePress to the latest stable release and ensure `pnpm docs:build` passes (including dead-link enforcement).

Acceptance:

- `pnpm docs:build` passes on a modern Node.
- Lockfile updated.
- Any newly-enforced dead links / invalid HTML fixed (or explicitly justified).

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #212
GH issue createdAt: 2026-01-02T18:39:11Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #212                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T18:39:11Z                            |
| Updated at          | 2026-01-03T02:00:57Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/212 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-214 - echo-session-ws-gateway: allowlist Origin check rejects requests missing Origin header

Context: Follow-up from PR #176 (embedded dashboard / ws gateway).

\## Problem

`crates/echo-session-ws-gateway/src/main.rs` has `origin_allowed(state, headers)` that behaves like this when `--allow-origin` is configured:

- if `Origin` header is present: allow only if it matches one of the configured origins
- if `Origin` header is missing: **deny** (returns `false`)

This can cause unexpected 403s for:

- non-browser clients that don't send `Origin`
- (possibly) same-origin browser traffic depending on browser/header behavior

Code pointer: `origin_allowed` around line ~1156.

\## Why this matters

- It's easy to configure `--allow-origin` intending “lock down cross-origin”, but accidentally lock out legitimate clients.
- The embedded dashboard is served from the same binary, so it's important the default behavior is sane for “serve dashboard + connect WS”.

\## Options

1. **Allow missing Origin by default** when `--allow-origin` is set (treat missing Origin as same-origin / non-browser and allow).
2. Keep current strict behavior, but **document it explicitly** and/or add an opt-in flag like `--require-origin` / `--deny-missing-origin`.
3. Hybrid: allow missing Origin only when the...

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #214
GH issue createdAt: 2026-01-02T18:43:55Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #214                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T18:43:55Z                            |
| Updated at          | 2026-01-02T21:44:08Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/214 |
| Labels              | bug, tooling, security                          |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-215 - CI: add Playwright e2e job for dashboard smoke + optional screenshot artifact

PR #176 added Playwright e2e coverage (`e2e/session-dashboard.spec.ts`) and an optional docs screenshot regen path gated by `ECHO_CAPTURE_DASHBOARD_SCREENSHOT=1`.

Right now `.github/workflows/ci.yml` runs Rust fmt/clippy/tests/guards only; it does **not** run Playwright, so:

- e2e regressions won't be caught in PRs
- screenshot drift is only caught manually

\## Proposal

Add a Playwright job to CI, with a cautious default to avoid flakiness / long runtimes:

\### Option A (recommended): PR smoke + artifacts

- Run `pnpm exec playwright test e2e/session-dashboard.spec.ts` on PRs.
- Upload Playwright report + screenshot attachments as artifacts.
- Keep `ECHO_CAPTURE_DASHBOARD_SCREENSHOT` **off** in CI (don't mutate repo), but still attach the PNG to the CI artifacts so humans can compare.

\### Option B: manual / nightly screenshot regen

- Add a `workflow_dispatch` job that runs the same test with `ECHO_CAPTURE_DASHBOARD_SCREENSHOT=1` and uploads the produced `docs/assets/wvp/session-dashboard.png` as an artifact (or opens a PR via bot if desired).

\## Implementation notes

- Node is already in-repo (`package.json`, `pnpm-lock.yaml`), so CI job should use `pnpm` with cache.
- Install...

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #215
GH issue createdAt: 2026-01-02T18:50:59Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #215                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T18:50:59Z                            |
| Updated at          | 2026-01-02T19:02:47Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/215 |
| Labels              | enhancement, tooling                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-219 - Repo policy: block merges until CodeRabbitAI approves

Right now it is _possible_ to merge PRs without CodeRabbitAI approval using the ruleset bypass (and `gh pr merge --admin`).

We want this to be a hard rule: **no merge to `main` until CodeRabbitAI has approved the current head commit**.

\## Plan

1. Add a lightweight GitHub Actions workflow that runs on `pull_request` + `pull_request_review` and reports a required status check (e.g. `CodeRabbit/Approval Gate`).
2. The gate fails unless the latest review from `coderabbitai[bot]` is `APPROVED` _for the PR head SHA_.
3. Update the `main` ruleset to:
    - remove the bypass actor,
    - disallow rebase merges,
    - stop requiring a human code-owner review (since this is a solo repo),
    - require the new CodeRabbit gate status check.

\## Acceptance criteria

- Any PR to `main` is blocked until CodeRabbitAI has approved the current head SHA.
- The gate updates automatically when CodeRabbitAI submits/edits/dismisses a review (no manual reruns required).
- `main` has no bypass path that allows merging without satisfying the gate.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #219
GH issue createdAt: 2026-01-02T21:59:12Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #219                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T21:59:12Z                            |
| Updated at          | 2026-01-03T02:07:34Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/219 |
| Labels              | tooling, security                               |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-221 - Demo 2: Splash Guy — course outline + scenario spec

Goal: define the teaching path and the canonical scenario for the networking-first course.

Deliverables

- A canonical scenario spec doc for "Splash Guy" (grid arena, water balloons, fuse timers, chain reactions, pickups).
- Course outline: module list, outcomes, exercises, and "failure demos" (controlled desyncs) per module.
- Glossary (public-facing terms first; formal names later).

Acceptance criteria

- A reader can understand: what we’re building, what’s special about Echo, and how the course will prove it.
- Scenario is clearly not branded as any existing commercial IP (theme + naming original).

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #221
GH issue createdAt: 2026-01-02T22:10:32Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #221                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:10:32Z                            |
| Updated at          | 2026-01-03T02:09:46Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/221 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-222 - Demo 2: Splash Guy — deterministic rules + state model

Goal: implement the Splash Guy gameplay model as deterministic rules over Echo’s state representation.

Scope (initial)

- Grid arena + walls/blocks.
- Players with deterministic movement + collision resolution.
- Water balloon placement with fuse timers.
- Explosion/splash propagation with chain reactions.
- Pickups/powerups (deterministic spawn policy; may start with fixed map placements).

Acceptance criteria

- Same initial state + same tick input log => identical state fingerprints across runs.
- A minimal set of invariants are tested (e.g., collision determinism, chain reaction ordering).

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #222
GH issue createdAt: 2026-01-02T22:10:47Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #222                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:10:47Z                            |
| Updated at          | 2026-01-02T22:10:47Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/222 |
| Labels              | none                                            |
| Mapped METHOD tasks | M147                                            |

<hr />

## GH-223 - Demo 2: Splash Guy — lockstep input protocol + two-peer harness

Goal: provide a repeatable way to run two peers locally and prove they stay in sync.

Deliverables

- Input log format (per tick, per player) + canonical encoding.
- A harness that runs:
    - Peer A and Peer B from the same seed + same input stream
    - compares per-tick fingerprints
    - produces a clear desync report on mismatch (tick number + minimal diff summary).

Notes

- Transport can be in-process initially; sockets/WVP integration can be a follow-up.

Acceptance criteria

- One command can run a short match and print PASS/FAIL.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #223
GH issue createdAt: 2026-01-02T22:11:02Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #223                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:11:02Z                            |
| Updated at          | 2026-01-02T22:11:02Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/223 |
| Labels              | none                                            |
| Mapped METHOD tasks | M146                                            |

<hr />

## GH-224 - Demo 2: Splash Guy — controlled desync lessons (make it fail on purpose)

Goal: create small, intentional nondeterminism toggles used for teaching and testing.

Examples

- Using wall-clock time inside the sim.
- Unseeded randomness.
- Unstable iteration order (e.g., unordered map iteration) affecting conflict resolution.

Deliverables

- A documented set of "breakers" that can be enabled locally to demonstrate divergence.
- A teaching flow: show mismatch -> reproduce -> fix -> verify.

Acceptance criteria

- Each breaker causes a reproducible divergence, and the harness points at the tick of first mismatch.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #224
GH issue createdAt: 2026-01-02T22:11:20Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #224                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:11:20Z                            |
| Updated at          | 2026-01-02T22:11:21Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/224 |
| Labels              | none                                            |
| Mapped METHOD tasks | M145                                            |

<hr />

## GH-225 - Demo 2: Splash Guy — minimal rendering / visualization path

Goal: make the demo watchable.

Options (pick one first, expand later)

- Terminal renderer (grid ASCII) driven by sim state.
- Simple 2D renderer (wasm/web or native) if already low-friction.

Acceptance criteria

- A human can see two peers side-by-side (or sequentially) and observe that they behave identically.
- Rendering is explicitly non-authoritative: it is derived from state and does not influence simulation.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #225
GH issue createdAt: 2026-01-02T22:11:36Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #225                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:11:36Z                            |
| Updated at          | 2026-01-02T22:11:36Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/225 |
| Labels              | none                                            |
| Mapped METHOD tasks | M172                                            |

<hr />

## GH-226 - Demo 2: Splash Guy — docs: networking-first course modules

Goal: write the docs course that teaches Echo by building Splash Guy with networking-first framing.

Deliverables

- Course index + modules with outcomes, small exercises, and links to code.
- Two-track structure:
    - Concept track (no code required)
    - Builder track (hands-on)

Acceptance criteria

- A motivated reader can go from zero to building/modifying the demo.
- Every module includes at least one "verify" step (replay or hash check).

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #226
GH issue createdAt: 2026-01-02T22:11:50Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #226                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:11:50Z                            |
| Updated at          | 2026-01-02T22:11:50Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/226 |
| Labels              | none                                            |
| Mapped METHOD tasks | M139, M145, M147, M172                          |

<hr />

## GH-228 - PR triage: support round-based ack comments

We currently ack review-thread actionables by replying inline to each GitHub review comment. This causes notification floods.\n\nAdd support in for a single PR timeline comment per review round that acks multiple review-thread comment IDs at once (while still validating commit SHAs and requiring human authorship).\n\nAlso update docs/procedures to recommend one per-round ack comment instead of per-thread replies.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #228
GH issue createdAt: 2026-01-02T22:20:50Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #228                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:20:50Z                            |
| Updated at          | 2026-01-03T01:55:49Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/228 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-230 - Demo 3: Tumble Tower — scenario spec + proof claim

Goal: define the canonical Demo 3 scenario and the staged physics ladder.

Deliverables

- Scenario spec doc for "Tumble Tower" (blocks, gravity, stacking, poke/remove interactions).
- Proof claim: same initial state + same tick inputs => identical per-tick fingerprints across peers.
- Stages (explicit):
    - Stage 0: 2D AABB blocks (no rotation)
    - Stage 1: rotation (OBB) + angular
    - Stage 2: friction + restitution
    - Stage 3: sleeping + stack stability
    - Stage 4+: optional 3D extension

Acceptance criteria

- The scenario is compelling to watch and clearly demonstrates determinism + physics.
- Each stage has a crisp definition of "done" and "verify".

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #230
GH issue createdAt: 2026-01-02T22:36:28Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #230                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:36:28Z                            |
| Updated at          | 2026-01-03T00:21:57Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/230 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-231 - Demo 3: Tumble Tower — Stage 0 physics (2D AABB stacking)

Goal: implement a deterministic 2D rigid-body-ish simulation with AABB blocks and gravity.

Scope

- Fixed tick dt.
- Bodies: axis-aligned rectangles only.
- Gravity, velocity integration.
- Collisions with static floor/walls and between blocks.
- Deterministic contact resolution ordering (stable sort key).

Acceptance criteria

- Same seed + same inputs => identical per-tick fingerprints across runs.
- A small suite of tests pin stability (stacking does not drift across runs; collision ordering stable).

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #231
GH issue createdAt: 2026-01-02T22:36:44Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #231                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:36:44Z                            |
| Updated at          | 2026-01-02T22:36:44Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/231 |
| Labels              | none                                            |
| Mapped METHOD tasks | M154, M155                                      |

<hr />

## GH-232 - Demo 3: Tumble Tower — Stage 1 physics (rotation + angular, OBB contacts)

Goal: extend Tumble Tower physics to support rotation and angular dynamics deterministically.

Scope

- Oriented boxes (OBB) with rotation.
- Angular velocity and torque.
- Deterministic contact manifold generation (stable ordering of contact points).
- Deterministic constraint solver ordering.

Acceptance criteria

- Stage 0 scenarios remain deterministic.
- A rotation-specific test suite pins contact generation and stacking behavior.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #232
GH issue createdAt: 2026-01-02T22:37:01Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #232                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:37:01Z                            |
| Updated at          | 2026-01-02T22:37:01Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/232 |
| Labels              | none                                            |
| Mapped METHOD tasks | M154, M155, M156                                |

<hr />

## GH-233 - Demo 3: Tumble Tower — Stage 2 physics (friction + restitution)

Goal: add friction and restitution while preserving deterministic behavior.

Scope

- Coefficient-based restitution (bounciness).
- Static + kinetic friction.
- Deterministic solver behavior across platforms (stable ordering + deterministic math).

Acceptance criteria

- A dedicated suite of tests pins outcomes for a few canonical interactions (slide, bounce, settle).
- Fingerprint stability remains green across runs.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #233
GH issue createdAt: 2026-01-02T22:37:16Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #233                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:37:16Z                            |
| Updated at          | 2026-01-02T22:37:16Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/233 |
| Labels              | none                                            |
| Mapped METHOD tasks | M156, M157                                      |

<hr />

## GH-234 - Demo 3: Tumble Tower — Stage 3 physics (sleeping + stack stability)

Goal: implement sleeping and long-run stack stability deterministically.

Why

- Long-run drift is where many physics engines desync across machines.

Scope

- Sleep thresholds and wake rules.
- Deterministic island building and solver ordering.
- Stability tests that run for many ticks and assert no divergence.

Acceptance criteria

- A multi-thousand-tick test run stays deterministic and stable.
- No platform-dependent iteration order affects sleep/wake transitions.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #234
GH issue createdAt: 2026-01-02T22:37:31Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #234                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:37:31Z                            |
| Updated at          | 2026-01-02T22:37:31Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/234 |
| Labels              | none                                            |
| Mapped METHOD tasks | M157                                            |

<hr />

## GH-235 - Demo 3: Tumble Tower — lockstep harness + per-tick fingerprinting

Goal: run two peers locally with the same seed + input stream, compare per-tick fingerprints, and produce a clear desync report.

Notes

- Start with in-process harness; integrate real transport later.

Acceptance criteria

- One command prints PASS/FAIL and the first mismatch tick.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #235
GH issue createdAt: 2026-01-02T22:37:45Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #235                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:37:45Z                            |
| Updated at          | 2026-01-02T22:37:46Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/235 |
| Labels              | none                                            |
| Mapped METHOD tasks | M151, M154                                      |

<hr />

## GH-236 - Demo 3: Tumble Tower — controlled desync breakers (physics edition)

Goal: create intentional nondeterminism toggles tailored to physics so we can teach and test desync diagnosis.

Examples

- Unstable contact ordering.
- Floating-point nondeterminism mode toggle (if applicable).
- Non-canonical math operations (e.g., platform transcendentals if they sneak in).
- Non-deterministic sleeping/island ordering.

Acceptance criteria

- Each breaker produces a reproducible divergence with a clear first mismatch tick.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #236
GH issue createdAt: 2026-01-02T22:38:01Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #236                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:38:01Z                            |
| Updated at          | 2026-01-02T22:38:01Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/236 |
| Labels              | none                                            |
| Mapped METHOD tasks | M150                                            |

<hr />

## GH-237 - Demo 3: Tumble Tower — visualization (2D view + debug overlays)

Goal: make the physics demo watchable and debuggable.

Options

- Terminal renderer (fast, deterministic, low deps) for early stages.
- Later: lightweight 2D viewer (wasm/web or native) with overlays.

Overlays to support debugging

- contact points
- velocities
- sleep state
- per-tick fingerprint

Acceptance criteria

- Viewer is explicitly non-authoritative: derived from state only.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #237
GH issue createdAt: 2026-01-02T22:38:15Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #237                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:38:15Z                            |
| Updated at          | 2026-01-02T22:38:15Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/237 |
| Labels              | none                                            |
| Mapped METHOD tasks | M179                                            |

<hr />

## GH-238 - Demo 3: Tumble Tower — docs course (physics ladder)

Goal: write a course that teaches deterministic physics in Echo in stages, using Tumble Tower as the running scenario.

Deliverables

- Course index + staged modules aligned to the physics ladder.
- Verify steps for each stage (fingerprints, replay, bisect on mismatch).
- Clear discussion of what makes physics determinism hard and how Echo approaches it.

Acceptance criteria

- A motivated reader can implement/modify the demo and keep it deterministic.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #238
GH issue createdAt: 2026-01-02T22:38:31Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #238                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:38:31Z                            |
| Updated at          | 2026-01-02T22:38:31Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/238 |
| Labels              | none                                            |
| Mapped METHOD tasks | M140, M150, M151, M154, M179                    |

<hr />

## GH-239 - Tooling: Reliving debugger UX (Constraint Lens + Provenance Heatmap)

Pulse ideas to make a complex deterministic scheduler debuggable:

\## Concepts

1. **Constraint Lens**: per-tick overlay showing why each step ran (preconditions, resource caps, scheduler rule, blocked alternatives) + ability to scrub and try counterfactual constraints.
2. **Provenance Heatmap**: visualize diffusion / blast radius of a change over the provenance/execution graph over time; highlight low-cohesion changes.
3. **Reliving** (movie scrubbing): timeline scrubber where each frame shows a minimal causal slice and supports branching/forking from any tick.

\## Why this matters for Echo

Echo already has:

- deterministic ticks/worldlines
- receipts/causal logs (or planned)

These UX tools make the determinism _usable_ for debugging + incident triage.

\## MVP (read-only)

- Define data shapes:
    - `AdmissionTrace` / `ConstraintPayload` per tick/step (predicate IDs, rule-pack hash, resource snapshot)
    - `CounterfactualRequest` (which constraint changed + new value)
    - `CohesionRollup` per tick (blast radius, diffusion edges)
- Record these during normal runs (no UI yet) and ensure they are replayable from logs.

\## Stretch

- Add 1-2 counterfactual sliders (CPU cap, priority we...

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #239
GH issue createdAt: 2026-01-02T22:43:10Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #239                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T22:43:10Z                            |
| Updated at          | 2026-01-02T22:43:10Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/239 |
| Labels              | enhancement, spec, tooling, backlog             |
| Mapped METHOD tasks | M112                                            |

<hr />

## GH-241 - docs: document issue dependency workflow

We are standardizing on native GitHub issue dependencies (blocked-by / blocking) rather than custom text fields.

This issue tracks adding a short procedure doc that covers:

- How to list dependencies from CLI
- How to add/remove "blocked by" edges
- The critical gotcha: REST endpoints require the blocking issue's numeric `issue_id`, not the `#issue_number`
- Notes about secondary rate limiting when creating dependency content quickly

Acceptance criteria:

- `docs/procedures/ISSUE-DEPENDENCIES.md` exists and is linked from `docs/ROADMAP.md`.
- `docs/decision-log.md` records the workflow choice.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #241
GH issue createdAt: 2026-01-02T23:54:49Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #241                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-02T23:54:49Z                            |
| Updated at          | 2026-01-03T02:13:39Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/241 |
| Labels              | documentation                                   |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-243 - TT1: dt policy (fixed timestep vs admitted dt stream)

From `docs/spec-time-streams-and-wormholes.md`: clarify whether simulation `dt` is fixed (Chronos tick implies fixed delta) or whether `dt` is itself an admitted stream.

Scope:

- Decide on default policy for Echo (likely fixed timestep), and define the rule for allowing variable dt (if allowed) as a TimeStream + StreamAdmissionDecision.
- Specify how dt interacts with determinism, replay, and catch-up predicates.
- Update relevant specs + docs.

Acceptance:

- Spec defines dt semantics in ticks/epochs, with explicit HostTime boundary and decision-record requirements if variable.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M049 (Spec — HistoryTime vs HostTime field classification (#191)), M050 (Spec — TTL/deadline semantics are ticks only (#192))
DAG chain depth: downstream max 8; upstream max 3
GH issue #: #243
GH issue createdAt: 2026-01-03T01:20:09Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #243                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-03T01:20:09Z                            |
| Updated at          | 2026-01-03T01:20:09Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/243 |
| Labels              | feature, spec                                   |
| Mapped METHOD tasks | M173                                            |

<hr />

## GH-244 - TT1: TimeStream retention + spool compaction + wormhole density

From `docs/spec-time-streams-and-wormholes.md`: define retention and compaction behavior for TimeStreams (backlogs/spools) and how they relate to wormholes/checkpoints and durability/WAL epochs.

Scope:

- Define retention periods and compaction rules for stream payloads (semantic envelopes vs raw bytes).
- Define how spools are represented on disk and how they interact with wormhole segments / checkpointing.
- Define minimal invariants for replay correctness when compaction occurs.

Acceptance:

- Spec section defines retention/compaction and how replays validate digests across compacted segments.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M049 (Spec — HistoryTime vs HostTime field classification (#191)), M050 (Spec — TTL/deadline semantics are ticks only (#192))
DAG chain depth: downstream max 8; upstream max 3
GH issue #: #244
GH issue createdAt: 2026-01-03T01:20:24Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #244                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-03T01:20:24Z                            |
| Updated at          | 2026-01-03T01:20:24Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/244 |
| Labels              | feature, spec                                   |
| Mapped METHOD tasks | M052, M174                                      |

<hr />

## GH-245 - TT1: Merge semantics for admitted stream facts across worldlines

From `docs/spec-time-streams-and-wormholes.md`: clarify merge semantics for stream-derived observation facts when clients buffer "future" events and later fork/merge/resync.

Scope:

- Define whether previously-buffered stream events admitted into a forked branch remain valid on merge, and under what authority/policy.
- Define how StreamAdmissionDecision records participate in merge conflicts.
- Define any "paradox quarantine" rules at the stream/fact layer.

Acceptance:

- Spec defines deterministic merge behavior (including what is forbidden) and how tools surface it.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M049 (Spec — HistoryTime vs HostTime field classification (#191)), M050 (Spec — TTL/deadline semantics are ticks only (#192))
DAG chain depth: downstream max 7; upstream max 3
GH issue #: #245
GH issue createdAt: 2026-01-03T01:20:40Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #245                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-03T01:20:40Z                            |
| Updated at          | 2026-01-03T01:20:40Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/245 |
| Labels              | feature, spec                                   |
| Mapped METHOD tasks | M175                                            |

<hr />

## GH-246 - TT1: Security/capabilities for fork/rewind/merge in multiplayer

From `docs/spec-time-streams-and-wormholes.md`: define capability/security model for time travel operations in multiplayer (fork/rewind/merge/resync) and how provenance sovereignty constrains tooling.

Scope:

- Define who can perform fork/rewind/merge actions; how those actions are authenticated/authorized.
- Define what tooling can observe/control during pause/rewind (and how it is recorded).
- Define the fail-closed behavior when capabilities are missing.

Acceptance:

- Spec defines the minimal capability matrix and how decisions are recorded into HistoryTime.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: M173 (Spec — dt policy: fixed timestep vs admitted dt stream (#243)), M174 (Spec — TimeStream retention, spool compaction, wormhole density (#244))
DAG chain depth: downstream max 7; upstream max 4
GH issue #: #246
GH issue createdAt: 2026-01-03T01:20:55Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #246                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-03T01:20:55Z                            |
| Updated at          | 2026-01-03T01:20:55Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/246 |
| Labels              | feature, spec, security                         |
| Mapped METHOD tasks | M051, M176                                      |

<hr />

## GH-248 - Remove CodeRabbit approval gate workflow

Context

- The workflow `PR Merge Gate / CodeRabbit approval required` fails on PR open/sync before CodeRabbit reviews, leaving red checks even when the bot later approves.
- This creates noise and requires manual reruns to clear stale failures.

Decision

- Remove the CodeRabbit approval gate workflow job and update any docs that refer to it.

Acceptance

- `.github/workflows/coderabbit-approval-gate.yml` removed.
- No docs instruct contributors to rely on this check.
- CI remains green on PRs.

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #248
GH issue createdAt: 2026-01-03T05:54:09Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #248                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-03T05:54:09Z                            |
| Updated at          | 2026-01-03T07:39:05Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/248 |
| Labels              | task, tooling                                   |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-258 - BOAW Phase 6B: Parallel Execution Engine Integration

\## Summary

Integrate the BOAW (Bag of Active Workers) sharded parallel execution into `engine_impl.rs::apply_reserved_rewrites()`.

\## Context

Phase 6B is the culmination of the BOAW parallel execution architecture. The primitives were implemented in previous phases:

- Phase 1-5: TickDelta, GraphView, WarpOp emission, read-only execution
- Phase 6A: `execute_parallel_sharded()` with atomic shard claiming

This phase wires everything into the main engine loop.

\## Deliverables

- [x] `apply_reserved_rewrites()` uses `execute_parallel_sharded()` internally
- [x] Per-warp parallelism (rewrites grouped by warp_id)
- [x] Configurable worker count via `ECHO_WORKERS` env var or `EngineBuilder::workers(n)`
- [x] Fix non-determinism bug in view op ID generation (`emit_view_op_delta_scoped()`)
- [x] All tests pass including DIND golden hashes
- [x] Tech debt documented in `TECH-DEBT-BOAW.md`

\## Related Docs

- `docs/adr/ADR-0007-BOAW-Storage.md` — full architecture
- `docs/adr/PLAN-PHASE-6B-VIRTUAL-SHARDS.md` — phase spec
- `docs/adr/TECH-DEBT-BOAW.md` — future work

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #258
GH issue createdAt: 2026-01-20T05:20:59Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #258                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-20T05:20:59Z                            |
| Updated at          | 2026-01-20T09:52:07Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/258 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-259 - SPEC-0004: Worldlines, Playback, TruthBus implementation

\## Summary

Implement SPEC-0004 (Worldlines, Playback, TruthBus) on the `graph-boaw` branch, building on the BOAW Phase 6B parallel execution infrastructure.

\## Scope

\### BOAW Phase 6B Completion (post-PR #257)

- Cross-warp parallelism via global work queue
- Stride fallback removal + dead code cleanup
- Phase 6B benchmark baseline

\### SPEC-0004 Implementation (6-commit OPORD)

1. MBUS v2 Encoder/Decoder
2. Types + IDs + ProvenanceStore
3. Seek + Verification
4. ViewSession + TruthSink + step()
5. Record Outputs + publish_truth
6. RetentionPolicy + checkpoint + fork

\### Test Coverage

- 22/22 SPEC-0004 tests passing (T1-T22)
- 510 total warp-core tests, zero warnings
- Self-review: 87 issues documented (P0 fixed, P1-P3 filed for follow-up)

\## Key Files

- `crates/warp-core/src/worldline.rs` — WorldlineId, HashTriplet, apply_warp_op_to_store
- `crates/warp-core/src/playback.rs` — PlaybackCursor, ViewSession, TruthSink, TruthFrame
- `crates/warp-core/src/provenance_store.rs` — ProvenanceStore trait, LocalProvenanceStore
- `crates/warp-core/src/retention.rs` — RetentionPolicy enum
- `crates/warp-core/src/materialization/frame_v2.rs` — MBUS v2 wire format

\## References

- Spec: `d...

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #259
GH issue createdAt: 2026-01-23T07:13:43Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #259                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-01-23T07:13:43Z                            |
| Updated at          | 2026-01-24T01:30:17Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/259 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-275 - [P0][DET] Multi-platform determinism matrix (macOS/Linux + wasm build checks)

Owner: CI Engineer\nEstimate: 6h\nAC:\n- CI matrix includes macOS+Linux parity suite\n- wasm build checks for DET-critical crates\n- digest mismatch fails CI\n- upload digest-table.csv with run ID + commit SHA

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #275
GH issue createdAt: 2026-02-14T16:37:59Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #275                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-02-14T16:37:59Z                            |
| Updated at          | 2026-03-08T08:25:38Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/275 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-276 - [P0][CI] Enforce artifact-backed VERIFIED claims

Owner: CI Engineer\nEstimate: 4h\nAC:\n- VERIFIED requires workflow/job, run ID, commit SHA, artifact name\n- enforce claim states VERIFIED/INFERRED/UNVERIFIED\n- fail CI on violations

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #276
GH issue createdAt: 2026-02-14T16:38:11Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #276                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-02-14T16:38:11Z                            |
| Updated at          | 2026-03-08T08:25:40Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/276 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-277 - [P0][DET] Classify workspace crates in det-policy.yaml

Owner: Architect\nEstimate: 3h\nAC:\n- every crate classified once: DET_CRITICAL/IMPORTANT/NONCRITICAL\n- schema validation in CI\n- owners for DET_CRITICAL

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #277
GH issue createdAt: 2026-02-14T16:38:22Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #277                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-02-14T16:38:22Z                            |
| Updated at          | 2026-03-08T08:22:24Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/277 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-278 - [P0][CI] Path-aware gate triggering from det-policy.yaml

Owner: CI Engineer\nEstimate: 3h\nDepends on: det-policy.yaml\nAC:\n- DET_CRITICAL => full gates\n- DET_IMPORTANT => reduced gates\n- DET_NONCRITICAL => skip heavy suite\n- CI summary explains run/skip

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #278
GH issue createdAt: 2026-02-14T16:38:33Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #278                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-02-14T16:38:33Z                            |
| Updated at          | 2026-03-08T08:22:25Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/278 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-279 - [P0][SEC] Explicit negative test mapping for decoder controls

Owner: Security Engineer\nEstimate: 5h\nAC:\n- map controls to explicit tests: trailing bytes, MAX_OPS+1, truncated payload, bad version handling\n- export sec-claim-map.json\n- mapped failure marks SEC claim UNVERIFIED

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep or import into METHOD. The GitHub issue is open but no METHOD row was found.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #279
GH issue createdAt: 2026-02-14T16:38:45Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #279                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-02-14T16:38:45Z                            |
| Updated at          | 2026-02-14T16:38:45Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/279 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-280 - [P0][PRF] Criterion baseline + regression threshold for materialization path

Owner: Performance Engineer\nEstimate: 7h\nAC:\n- criterion harness for hot path\n- baseline pre-change + current\n- threshold policy (e.g., p95 < 15%)\n- upload perf-report.json\n- gate G3 reflects data

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #280
GH issue createdAt: 2026-02-14T16:38:56Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #280                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-02-14T16:38:56Z                            |
| Updated at          | 2026-03-08T09:15:20Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/280 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-281 - [P0][POLICY] Staging vs production blocker matrix

Owner: Architect\nEstimate: 2h\nAC:\n- docs/RELEASE_POLICY.md with explicit blocker sets\n- recommendation logic tied to gate states only

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep or import into METHOD. The GitHub issue is open but no METHOD row was found.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #281
GH issue createdAt: 2026-02-14T16:39:07Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #281                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-02-14T16:39:07Z                            |
| Updated at          | 2026-02-14T16:39:07Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/281 |
| Labels              | none                                            |
| Mapped METHOD tasks | none                                            |

<hr />

## GH-282 - [P0][OPS] Commit-ordered rollback playbooks for TTD integration

Owner: Release Engineer\nEstimate: 4h\nDepends on: release policy\nAC:\n- full/partial rollback sequences\n- dependency constraints\n- post-revert checks\n- docs/ROLLBACK_TTD.md

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #282
GH issue createdAt: 2026-02-14T16:39:19Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #282                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-02-14T16:39:19Z                            |
| Updated at          | 2026-02-14T16:39:19Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/282 |
| Labels              | none                                            |
| Mapped METHOD tasks | M037                                            |

<hr />

## GH-284 - CI: Per-crate gate overrides in det-policy classification system

\## Context

`warp-benches` was promoted from `DET_NONCRITICAL` to `DET_IMPORTANT` in PR #283 as a pragmatic compromise. Ideally, the classification system should support **per-crate gate overrides** so a crate can declare which specific gates it requires (e.g., `required_gates: [G3]`) without needing to change its entire classification tier.

\## Current Behavior

- `classify_changes.cjs` only reads `crateInfo.class` to determine gates
- The crate-level `required_gates` field (removed in #283 as dead config) was never consumed

\## Desired Behavior

- `det-policy.yaml` supports optional crate-level `required_gates` that **augments** the class-level gates
- `classify_changes.cjs` merges class gates + crate-level overrides
- Example: `warp-benches` could be `DET_NONCRITICAL` with `required_gates: [G3]`, triggering G3 when benchmarks change without triggering G1/G2/G4

\## Acceptance Criteria

- [ ] `classify_changes.cjs` reads and merges crate-level `required_gates`
- [ ] `validate_det_policy.cjs` validates crate-level gates against `ALLOWED_GATES`
- [ ] At least one crate uses the override (warp-benches)

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #284
GH issue createdAt: 2026-02-15T18:48:49Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #284                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-02-15T18:48:49Z                            |
| Updated at          | 2026-02-15T18:48:49Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/284 |
| Labels              | enhancement                                     |
| Mapped METHOD tasks | M009                                            |

<hr />

## GH-285 - CI: Auto-generate DETERMINISM_PATHS from det-policy.yaml DET_CRITICAL entries

\## Context

In PR #283, `DETERMINISM_PATHS` in `det-gates.yml` was expanded from 1 crate to all 14 DET_CRITICAL crates. However, this list is **hardcoded** in the workflow YAML. If a new DET_CRITICAL crate is added to `det-policy.yaml`, the workflow won't automatically pick it up.

\## Desired Behavior

Generate the `DETERMINISM_PATHS` env var dynamically at CI time by reading `det-policy.yaml` and extracting all crate paths classified as `DET_CRITICAL`.

\## Options

1. **Script approach**: Add a step that reads `det-policy.json` and outputs paths
2. **classify_changes.cjs enhancement**: Output DET_CRITICAL paths alongside classification

\## Acceptance Criteria

- [ ] `DETERMINISM_PATHS` is derived from `det-policy.yaml` at CI time
- [ ] Adding a new DET_CRITICAL crate to policy automatically includes it in static inspection
- [ ] No hardcoded crate list in `det-gates.yml`

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #285
GH issue createdAt: 2026-02-15T18:48:55Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #285                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-02-15T18:48:55Z                            |
| Updated at          | 2026-02-15T18:48:55Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/285 |
| Labels              | enhancement                                     |
| Mapped METHOD tasks | M009                                            |

<hr />

## GH-286 - CI: Add unit tests for classify_changes.cjs and matches()

\## Context

In PR #283, `classify_changes.cjs` was updated to export `classifyChanges` and `matches` via `module.exports`. These functions are now testable but have no unit tests.

\## Scope

- Test `matches()` glob function: `**` recursive, `*` single-level, literal paths, dots
- Test `classifyChanges()`: all three tiers, `require_full_classification` error path, empty file list
- Test edge cases: overlapping patterns, missing policy fields

\## Acceptance Criteria

- [ ] Unit test file (e.g., `scripts/__tests__/classify_changes.test.cjs`)
- [ ] Tests cover all classification tiers and error paths
- [ ] Tests run in CI (add to workflow or package.json scripts)

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep. The GitHub issue is open and has METHOD backlog coverage.

DAG blocked by: none
DAG chain depth: downstream max 1; upstream max 1
GH issue #: #286
GH issue createdAt: 2026-02-15T18:49:18Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #286                                            |
| State               | OPEN                                            |
| Author              | flyingrobots                                    |
| Created at          | 2026-02-15T18:49:18Z                            |
| Updated at          | 2026-02-15T18:49:18Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/286 |
| Labels              | tooling, backlog                                |
| Mapped METHOD tasks | M009                                            |

<hr />

## GH-287 - Docs: Document ban-nondeterminism.sh allowlist process in RELEASE_POLICY.md

\## Context

`ban-nondeterminism.sh` supports a file-based allowlist mechanism (`.ban-nondeterminism-allowlist`) for exempting specific paths from determinism checks. This is powerful but undocumented in `RELEASE_POLICY.md`.

\## Current State

- The script reads `DETERMINISM_ALLOWLIST` env var (defaults to `.ban-nondeterminism-allowlist`)
- Allowlist format: one glob pattern per line, `#` comments supported
- No governance documentation for when/how to use exemptions

\## Desired Behavior

Add a section to `docs/RELEASE_POLICY.md` documenting:

- What the allowlist is and where it lives
- When an exemption is acceptable vs. when code should be refactored
- Approval requirements for adding allowlist entries
- How allowlist entries are audited

\## Acceptance Criteria

- [ ] New section in RELEASE_POLICY.md covering allowlist governance
- [ ] Cross-reference from ban-nondeterminism.sh header comments

### Decision

> [!danger] Delete?
>
> - [x] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion/archive candidate. The GitHub issue is closed and no METHOD row references it.

DAG blocked by: n/a; no METHOD mapping found
DAG chain depth: n/a; no METHOD mapping found
GH issue #: #287
GH issue createdAt: 2026-02-15T18:49:20Z

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| Source              | GitHub issue                                    |
| GH issue #          | #287                                            |
| State               | CLOSED                                          |
| Author              | flyingrobots                                    |
| Created at          | 2026-02-15T18:49:20Z                            |
| Updated at          | 2026-03-08T09:15:20Z                            |
| URL                 | https://github.com/flyingrobots/echo/issues/287 |
| Labels              | documentation, backlog                          |
| Mapped METHOD tasks | none                                            |

<hr />

# Active Design Cycles

## DESIGN-0003 - Lock the dt policy

_Ratify fixed timestep as a history-plane invariant: dt is fixed per
worldline, no committed tick carries its own dt, and wall-clock time
never enters semantic history._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: #243
GH issue createdAt: #243: 2026-01-03T01:20:09Z

| Field                    | Value                                |
| ------------------------ | ------------------------------------ |
| Source                   | METHOD active design cycle           |
| Design id                | DESIGN-0003                          |
| Source path              | docs/design/0003-dt-policy/design.md |
| Referenced GitHub issues | #243                                 |

<hr />

## DESIGN-0004 - Strand contract

_Define the strand as a first-class relation in Echo with exact fields,
invariants, lifecycle, and TTD mapping._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                      |
| ------------------------ | ------------------------------------------ |
| Source                   | METHOD active design cycle                 |
| Design id                | DESIGN-0004                                |
| Source path              | docs/design/0004-strand-contract/design.md |
| Referenced GitHub issues | none                                       |

<hr />

## DESIGN-0005 - Echo TTD witness surface

_Define how Echo's current runtime objects map to `warp-ttd` neighborhood core, reintegration detail, and receipt shell._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                               |
| ------------------------ | --------------------------------------------------- |
| Source                   | METHOD active design cycle                          |
| Design id                | DESIGN-0005                                         |
| Source path              | docs/design/0005-echo-ttd-witness-surface/design.md |
| Referenced GitHub issues | none                                                |

<hr />

## DESIGN-0006 - Echo Continuum alignment

_Decide what Echo must change so Continuum tools can consume one honest shared
observer/debugger noun stack without flattening Echo’s runtime-specific truth._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                               |
| ------------------------ | --------------------------------------------------- |
| Source                   | METHOD active design cycle                          |
| Design id                | DESIGN-0006                                         |
| Source path              | docs/design/0006-echo-continuum-alignment/design.md |
| Referenced GitHub issues | none                                                |

<hr />

## DESIGN-0007 - Braid geometry and neighborhood publication

_Make Echo strands capable of read-only braid geometry and publish one honest
local site object for Continuum / `warp-ttd` consumption._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                  |
| ------------------------ | ---------------------------------------------------------------------- |
| Source                   | METHOD active design cycle                                             |
| Design id                | DESIGN-0007                                                            |
| Source path              | docs/design/0007-braid-geometry-and-neighborhood-publication/design.md |
| Referenced GitHub issues | none                                                                   |

<hr />

## DESIGN-0008 - Strand settlement and conflict artifacts

_Define Echo's first deterministic settlement runway for strands:
compare -> plan -> import -> conflict artifact._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                        |
| ------------------------ | -------------------------------------------- |
| Source                   | METHOD active design cycle                   |
| Design id                | DESIGN-0008                                  |
| Source path              | docs/design/0008-strand-settlement/design.md |
| Referenced GitHub issues | none                                         |

<hr />

## DESIGN-0009 - Witnessed causal suffix export and import

_Define Echo's runtime-side handoff law for simultaneous hot/cold operation
with `git-warp`: export and import witnessed suffix bundles, not state._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                   |
| ------------------------ | ------------------------------------------------------- |
| Source                   | METHOD active design cycle                              |
| Design id                | DESIGN-0009                                             |
| Source path              | docs/design/0009-witnessed-causal-suffix-sync/design.md |
| Referenced GitHub issues | none                                                    |

<hr />

## DESIGN-0010 - Live-basis settlement correction plan

_Record the runtime decisions, consequences, and implementation runway for
moving Echo from frozen-fork strand settlement toward live holographic strand
semantics._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                 |
| ------------------------ | ----------------------------------------------------- |
| Source                   | METHOD active design cycle                            |
| Design id                | DESIGN-0010                                           |
| Source path              | docs/design/0010-live-basis-settlement-plan/design.md |
| Referenced GitHub issues | none                                                  |

<hr />

## DESIGN-0011 - Optic and observer runtime doctrine

_Formalize the runtime subset of WARP optics and Observer Geometry so Echo can
implement live strands, settlement, observation, and witnessed shells with one
shared noun stack._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                      |
| ------------------------ | ---------------------------------------------------------- |
| Source                   | METHOD active design cycle                                 |
| Design id                | DESIGN-0011                                                |
| Source path              | docs/design/0011-optic-observer-runtime-doctrine/design.md |
| Referenced GitHub issues | none                                                       |

<hr />

## DESIGN-0012 - Witnessed suffix posture canonicalization

_Add named canonical construction for witnessed suffix local admission
postures._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: #323
GH issue createdAt: #323: unknown

| Field                    | Value                                                                |
| ------------------------ | -------------------------------------------------------------------- |
| Source                   | METHOD active design cycle                                           |
| Design id                | DESIGN-0012                                                          |
| Source path              | docs/design/0012-witnessed-suffix-posture-canonicalization/design.md |
| Referenced GitHub issues | #323                                                                 |

<hr />

## DESIGN-0013 - Wesley Compiled Contract Hosting Doctrine

_Define Echo as a generic host for Wesley-compiled contract families, not as an
application-specific runtime API._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                |
| ------------------------ | -------------------------------------------------------------------- |
| Source                   | METHOD active design cycle                                           |
| Design id                | DESIGN-0013                                                          |
| Source path              | docs/design/0013-wesley-compiled-contract-hosting-doctrine/design.md |
| Referenced GitHub issues | none                                                                 |

<hr />

## DESIGN-0014 - EINT, Registry, And Observation Boundary Inventory

_Inventory the existing Echo intent, registry, and observation substrate before
adding Wesley-generated contract hosting behavior._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                   |
| ------------------------ | ----------------------------------------------------------------------- |
| Source                   | METHOD active design cycle                                              |
| Design id                | DESIGN-0014                                                             |
| Source path              | docs/design/0014-eint-registry-observation-boundary-inventory/design.md |
| Referenced GitHub issues | none                                                                    |

<hr />

## DESIGN-0015 - Registry Provider Host Boundary Decision

_Choose the first host boundary for Wesley-generated registries without
changing Echo's app-agnostic EINT ingress._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                               |
| ------------------------ | ------------------------------------------------------------------- |
| Source                   | METHOD active design cycle                                          |
| Design id                | DESIGN-0015                                                         |
| Source path              | docs/design/0015-registry-provider-host-boundary-decision/design.md |
| Referenced GitHub issues | none                                                                |

<hr />

## DESIGN-0016 - Wesley To Echo Toy Contract Proof

_Prove one boring Wesley-generated contract path from generated op metadata to
EINT dispatch and an observation/read bridge._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                        |
| ------------------------ | ------------------------------------------------------------ |
| Source                   | METHOD active design cycle                                   |
| Design id                | DESIGN-0016                                                  |
| Source path              | docs/design/0016-wesley-to-echo-toy-contract-proof/design.md |
| Referenced GitHub issues | none                                                         |

<hr />

## DESIGN-0017 - Authenticated Wesley Intent Admission Posture

_Name the missing security and artifact-trust boundary between
Wesley-generated contract helpers and Echo tick admission._

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                                    |
| ------------------------ | ------------------------------------------------------------------------ |
| Source                   | METHOD active design cycle                                               |
| Design id                | DESIGN-0017                                                              |
| Source path              | docs/design/0017-authenticated-wesley-intent-admission-posture/design.md |
| Referenced GitHub issues | none                                                                     |

<hr />

## DESIGN-0018 - Echo Optics API Design

Source request: [request.md](./request.md)

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: keep while the design cycle remains active; delete or move only through the METHOD close/pivot process.

DAG blocked by: n/a; active design cycles are outside `task-matrix.csv`
DAG chain depth: n/a; active design cycles are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                             |
| ------------------------ | ------------------------------------------------- |
| Source                   | METHOD active design cycle                        |
| Design id                | DESIGN-0018                                       |
| Source path              | docs/design/0018-echo-optics-api-design/design.md |
| Referenced GitHub issues | none                                              |

<hr />

# METHOD Graveyard Notes

## GRAVEYARD-5X-DUTY-MODEL - 5x Duty Model

**Rejected.** The 5x Duty Model (documentation + implementation +
interactive demo + living test + certification from a single source)
was an ambitious vision for Echo's methodology. It was never practiced.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion candidate only if no active backlog/design/GitHub item still cites this note; it is already outside the active METHOD lanes.

DAG blocked by: n/a; graveyard notes are outside `task-matrix.csv`
DAG chain depth: n/a; graveyard notes are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                  |
| ------------------------ | -------------------------------------- |
| Source                   | METHOD graveyard note                  |
| Graveyard id             | GRAVEYARD-5X-DUTY-MODEL                |
| Source path              | docs/method/graveyard/5x-duty-model.md |
| Referenced GitHub issues | none                                   |

<hr />

## GRAVEYARD-KERNEL-DOMAIN-SEPARATED-HASHES - Domain-Separated Hash Contexts

> **Milestone:** Lock the Hashes | **Priority:** P0

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion candidate only if no active backlog/design/GitHub item still cites this note; it is already outside the active METHOD lanes.

DAG blocked by: n/a; graveyard notes are outside `task-matrix.csv`
DAG chain depth: n/a; graveyard notes are outside `task-matrix.csv`
GH issue #: #185, #186, #265, #266
GH issue createdAt: #185: 2026-01-02T16:51:28Z, #186: 2026-01-02T16:51:30Z, #265: unknown, #266: unknown

| Field                    | Value                                                   |
| ------------------------ | ------------------------------------------------------- |
| Source                   | METHOD graveyard note                                   |
| Graveyard id             | GRAVEYARD-KERNEL-DOMAIN-SEPARATED-HASHES                |
| Source path              | docs/method/graveyard/KERNEL_domain-separated-hashes.md |
| Referenced GitHub issues | #185, #186, #265, #266                                  |

<hr />

## GRAVEYARD-KERNEL-STRANDS-AND-BRAIDING - Strands and braiding for Echo

Echo has fork infrastructure but no strand or braiding semantics.
git-warp has a full implementation. This item tracks bringing the
concept to Echo.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion candidate only if no active backlog/design/GitHub item still cites this note; it is already outside the active METHOD lanes.

DAG blocked by: n/a; graveyard notes are outside `task-matrix.csv`
DAG chain depth: n/a; graveyard notes are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                                |
| ------------------------ | ---------------------------------------------------- |
| Source                   | METHOD graveyard note                                |
| Graveyard id             | GRAVEYARD-KERNEL-STRANDS-AND-BRAIDING                |
| Source path              | docs/method/graveyard/KERNEL_strands-and-braiding.md |
| Referenced GitHub issues | none                                                 |

<hr />

## GRAVEYARD-KERNEL-STREAM-MERGE-SEMANTICS - Merge semantics for admitted stream facts across worldlines

Ref: #245

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion candidate only if no active backlog/design/GitHub item still cites this note; it is already outside the active METHOD lanes.

DAG blocked by: n/a; graveyard notes are outside `task-matrix.csv`
DAG chain depth: n/a; graveyard notes are outside `task-matrix.csv`
GH issue #: #245
GH issue createdAt: #245: 2026-01-03T01:20:40Z

| Field                    | Value                                                  |
| ------------------------ | ------------------------------------------------------ |
| Source                   | METHOD graveyard note                                  |
| Graveyard id             | GRAVEYARD-KERNEL-STREAM-MERGE-SEMANTICS                |
| Source path              | docs/method/graveyard/KERNEL_stream-merge-semantics.md |
| Referenced GitHub issues | #245                                                   |

<hr />

## GRAVEYARD-PLATFORM-BENCHMARKS-CLEANUP - Benchmarks Pipeline Cleanup

> **Milestone:** Lock the Hashes | **Priority:** P0

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion candidate only if no active backlog/design/GitHub item still cites this note; it is already outside the active METHOD lanes.

DAG blocked by: n/a; graveyard notes are outside `task-matrix.csv`
DAG chain depth: n/a; graveyard notes are outside `task-matrix.csv`
GH issue #: #22, #41, #42, #43, #44, #45, #46, #265, #266
GH issue createdAt: #22: 2025-10-30T07:54:59Z, #41: 2025-10-30T07:58:52Z, #42: 2025-10-30T07:58:56Z, #43: 2025-10-30T07:59:00Z, #44: 2025-10-30T07:59:04Z, #45: 2025-10-30T07:59:08Z, #46: 2025-10-30T07:59:12Z, #265: unknown, #266: unknown

| Field                    | Value                                                |
| ------------------------ | ---------------------------------------------------- |
| Source                   | METHOD graveyard note                                |
| Graveyard id             | GRAVEYARD-PLATFORM-BENCHMARKS-CLEANUP                |
| Source path              | docs/method/graveyard/PLATFORM_benchmarks-cleanup.md |
| Referenced GitHub issues | #22, #41, #42, #43, #44, #45, #46, #265, #266        |

<hr />

## GRAVEYARD-BOAW-NAMING - BOAW naming

**Rejected.** The name "BOAW" (Bag of Active Warps) was used for the
parallel execution engine through Phases 5–6B. It was confusing,
unexplained in most contexts, and added jargon without clarity.

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion candidate only if no active backlog/design/GitHub item still cites this note; it is already outside the active METHOD lanes.

DAG blocked by: n/a; graveyard notes are outside `task-matrix.csv`
DAG chain depth: n/a; graveyard notes are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                |
| ------------------------ | ------------------------------------ |
| Source                   | METHOD graveyard note                |
| Graveyard id             | GRAVEYARD-BOAW-NAMING                |
| Source path              | docs/method/graveyard/boaw-naming.md |
| Referenced GitHub issues | none                                 |

<hr />

## GRAVEYARD-UNIMPLEMENTED-FUTURE-SPECS - Unimplemented future specs

**Rejected.** 17 spec files were deleted during the 2026-04-03 docs
audit. They described features that do not exist:

### Decision

> [!danger] Delete?
>
> - [ ] Yes, Delete
> - [ ] No, keep

### Info

Best guess: deletion candidate only if no active backlog/design/GitHub item still cites this note; it is already outside the active METHOD lanes.

DAG blocked by: n/a; graveyard notes are outside `task-matrix.csv`
DAG chain depth: n/a; graveyard notes are outside `task-matrix.csv`
GH issue #: none
GH issue createdAt: n/a

| Field                    | Value                                               |
| ------------------------ | --------------------------------------------------- |
| Source                   | METHOD graveyard note                               |
| Graveyard id             | GRAVEYARD-UNIMPLEMENTED-FUTURE-SPECS                |
| Source path              | docs/method/graveyard/unimplemented-future-specs.md |
| Referenced GitHub issues | none                                                |

<hr />
