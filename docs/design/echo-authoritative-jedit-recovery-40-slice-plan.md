<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
<!-- markdownlint-disable MD026 -->

# Echo-Authoritative jedit Recovery 40-Slice Campaign

Last updated: 2026-05-26.

This plan defines the next forty slices for proving that `jedit` can treat Echo
as the durable causal authority for an edit across process death.

The milestone is:

```text
[jedit] app-owned edit intent
-> [Echo] WAL-backed accepted submission
-> [Echo] scheduler-owned receipt
-> [Echo] bounded reading evidence
-> process death
-> [Echo] recovery from committed history
-> [jedit] editor-facing status and materialization from Echo truth
```

The plan is intentionally cross-repo. Echo owns generic causal runtime,
submission posture, receipts, readings, WAL, recovery, and authority
boundaries. jedit owns editor nouns, session UX, text contract semantics,
status mapping, and materialization policy. Echo must not learn editor nouns.

This is a gated campaign, not forty equal-status work items. Slice 0 is a
no-count doctrine slice that must land before the forty budgeted slices begin.
The numbered slices keep stable identifiers, but execution follows the gate
order below rather than raw numeric order.

## Gate Structure

| Gate                                     | Slices                               | Gate question                                                                                                                  | Required artifact                                               |
| ---------------------------------------- | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------- |
| Gate 0: Invariant Charter                | Slice 0                              | Do Echo and jedit share one state model, duplicate law, crashpoint taxonomy, and source-of-truth rule?                         | State machine and invariant charter.                            |
| Gate A: Echo Recovery Truth              | Slices 1-17 plus Slice 36 early      | Can Echo prove accepted submissions, duplicate retries, receipts, readings, commit evidence, and recovery posture generically? | Echo recovery report fixture and no-editor-nouns guard.         |
| Gate B: jedit Consumes Echo Truth        | Slices 18-25 plus Slices 32-34 early | Can jedit consume Echo truth through ports without local memory fallback?                                                      | jedit recovery report with static and runtime tripwire results. |
| Gate C: Crash-Window Authority           | Slices 26-31                         | Can the system survive every named death window without double-apply or local fallback?                                        | Crash matrix report.                                            |
| Gate D: Boundary, Report, And Drift Lock | Slices 35 and 37-40                  | Can the final proof resist malicious adapters and produce stable machine/human evidence?                                       | Aggregate gate JSON and human receipt card.                     |

## Canonical State Model

Machine-readable recovery reports must not collapse retry disposition, lifecycle
posture, scheduler decision, and evidence health into one enum.

```json
{
    "submission": {
        "identity": "sub_...",
        "envelope_digest": "sha256:...",
        "contract_identity": "contract:...",
        "basis": "basis_..."
    },
    "intake": {
        "disposition": "accepted_new | duplicate_same_submission | conflicting_duplicate | validation_failed",
        "accepted_evidence": "present | absent"
    },
    "lifecycle": {
        "posture": "not_found | accepted_pending | accepted_deciding | decided"
    },
    "decision": {
        "result": "applied | rejected | obstructed | none"
    },
    "evidence_health": {
        "status": "complete | incomplete_evidence | corrupt_or_untrusted | missing_retention | redacted"
    }
}
```

`accepted_deciding` is allowed only if Echo can durably reconstruct a
scheduler-owned staged/claimed posture without a final committed receipt. If the
runtime cannot prove that distinction at implementation time, the slice must
collapse it back to `accepted_pending` rather than invent an unrecoverable mood.

## Machine Contract Rules

- Every machine-readable contract has a `schema_version`.
- Every report names `producer`, `producer_version`, and compatibility metadata.
- Unknown schema versions fail closed unless an explicit compatibility rule
  exists.
- The release gate must state the durability mode. Product authority requires a
  filesystem fsync-level mode or a strictly stronger configured mode.
- `source_of_truth: "echo"` is a gated conclusion, not decorative text.
- App-facing recovery commands and ports may expose submission posture,
  causal-chain posture, and recovery evidence; they must not expose WAL as the
  product noun. WAL-specific wording belongs in Echo diagnostics.

`source_of_truth` may be `echo` only when accepted evidence exists, any final
decision is scheduler-owned, readings come from retained or read-only
rederived Echo evidence, jedit used app-owned ports, recovery came from
WAL-backed Echo state, and no legacy tripwire fired.

`decision.result: "applied"` requires state evidence, not only a receipt. A
receipt can prove that the trusted scheduler decided; applied posture also needs
committed or recoverably rederived state/basis evidence that the decision
affected Echo causal state.

Reading evidence must say how it was obtained:

```json
{
    "reading_source": "retained | rederived_from_basis | unavailable",
    "reading_authority": "echo_committed_reading | echo_read_only_rederivation | none"
}
```

Retained readings are stronger than rederived readings. Rederived readings are
allowed only through read-only Echo observer paths over recovered causal basis.
Unavailable readings must not be replaced with local memory.

Basis mismatch is a first-class obstruction. It must not be collapsed into
generic rejection when the submitted operation's basis/preconditions no longer
match recovered causal state.

## Durability Mode Bar

| Mode                                   | Meaning                                                                                     | Product gate posture                            |
| -------------------------------------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| `memory_test`                          | Committed to deterministic in-memory test log only.                                         | Not sufficient for product claim.               |
| `filesystem_buffered`                  | Written to filesystem WAL without strict fsync guarantee.                                   | Soft durability only.                           |
| `filesystem_fsync`                     | Frame and commit flushed under filesystem policy.                                           | Minimum product gate mode.                      |
| `filesystem_fsync_with_directory_sync` | WAL file and relevant directory metadata durability boundaries are closed where applicable. | Preferred product gate mode.                    |
| `strict_object_store`                  | Conditional manifest/object semantics prove strict durability without filesystem fsync.     | Future equivalent if implemented and witnessed. |

## Gate Execution Tiers

The campaign has three executable tiers so the expensive proof does not become
an every-commit tax and the cheap proof does not pretend to be enough.

| Tier             | Required checks                                                                                                           | Purpose                                            |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| PR smoke         | Schema validation, no-editor-nouns guard, jedit no-legacy guard, duplicate fixtures, posture mapping fixtures.            | Catch drift and cheating before review.            |
| Integration gate | Filesystem WAL recovery, happy-path jedit recovery, local amnesia, one representative crashpoint.                         | Prove the main cross-repo path during active work. |
| Release gate     | Full crash matrix, malicious adapter suite, materialization, aggregate JSON report, human receipt card, docs drift audit. | Earn the final Echo-authoritative claim.           |

## Shared Golden Fixtures And Drift Tests

Echo should produce and jedit should consume shared golden fixtures before
either repo relies on a new report shape:

- `accepted_submission.v1.json`;
- `posture_pending.v1.json`;
- `posture_applied_with_reading.v1.json`;
- `posture_applied_missing_reading.v1.json`;
- `duplicate_same_submission.v1.json`;
- `conflicting_duplicate.v1.json`;
- `causal_chain_complete.v1.json`;
- `causal_chain_incomplete.v1.json`.

Every cross-repo JSON contract must have a drift test:

- Echo can still produce the schema jedit expects.
- jedit can still parse the schema Echo produces.
- Unknown schema versions fail closed.
- Fixture updates happen in the same slice as producer/consumer contract
  changes.

## Crashpoint Ownership Taxonomy

Echo crashpoints are runtime durability boundaries. jedit crashpoints are
application/adapter lifecycle boundaries. Cross-boundary crashpoints involve
handoff between the two.

| Owner          | Crashpoint                                     | Meaning                                                                   |
| -------------- | ---------------------------------------------- | ------------------------------------------------------------------------- |
| Echo           | `before_accept_commit`                         | Process dies before the WAL acceptance commit.                            |
| Echo           | `after_accept_commit_before_ack`               | Acceptance is committed but the app response is lost.                     |
| Echo           | `after_accept_before_tick`                     | Accepted submission exists; no final scheduler receipt exists.            |
| Echo           | `after_receipt_before_reading`                 | Scheduler receipt is committed; bounded reading is absent or rederivable. |
| Echo           | `after_reading_commit`                         | Reading evidence is committed before app observation.                     |
| Cross-boundary | `after_echo_publish_before_adapter_record`     | Echo published evidence; adapter-local status has not recorded it.        |
| jedit          | `after_reading_delivered_before_local_observe` | Adapter delivered reading; jedit app has not observed it.                 |
| jedit          | `after_app_observe_before_local_status`        | App observed result; local status was not persisted.                      |

## Slice Checklist

### Gate 0: Invariant Charter

- [ ] Slice 0: \[Echo]\[jedit] authority invariant charter and posture state
      machine.

### Gate A: Echo Recovery Truth

- [ ] Slice 1: [Echo] WAL reality and stale claim cleanup.
- [ ] Slice 2: [Echo] accepted-submission evidence contract.
- [ ] Slice 3: [Echo] generic recovery posture taxonomy.
- [ ] Slice 4: [Echo] duplicate submission and idempotency law.
- [ ] Slice 5: [Echo] crashpoint fixture contract for external apps.
- [ ] Slice 36: [Echo] no-editor-nouns trusted-runtime guard.
- [ ] Slice 6: [Echo] filesystem accepted-submission restart fixture.
- [ ] Slice 7: [Echo] crash-before-ACK retry witness.
- [ ] Slice 8: [Echo] conflicting duplicate rejection witness.
- [ ] Slice 9: [Echo] app-safe submission posture read surface.
- [ ] Slice 10: [Echo] recovery certificate submission posture counts.
- [ ] Slice 11: [Echo] receipt correlation recovery witness.
- [ ] Slice 12: [Echo] tick commit-before-publish failure injection.
- [ ] Slice 13: [Echo] bounded reading identity recovery witness.
- [ ] Slice 14: [Echo] retained material recovery obstruction witness.
- [ ] Slice 15: [Echo] receipt-to-reading causal chain read model.
- [ ] Slice 16: [Echo] causal commit evidence JSON contract.
- [ ] Slice 17: [Echo] generic external-app recovery gate command.

### Gate B: jedit Consumes Echo Truth

- [ ] Slice 18: [jedit] Echo recovery port interface.
- [ ] Slice 19: [jedit] Echo recovery adapter implementation.
- [ ] Slice 20: [jedit] generic-to-editor posture mapping.
- [ ] Slice 21: [jedit] stable edit submission identity.
- [ ] Slice 22: [jedit] recovery evidence report fields.
- [ ] Slice 32: [jedit] production legacy memory static guard.
- [ ] Slice 33: [jedit] release-gate runtime tripwire mode.
- [ ] Slice 23: [jedit] recovered bounded reading path.
- [ ] Slice 24: [jedit] happy-path recovery gate scenario.
- [ ] Slice 25: [jedit] retry after local amnesia scenario.
- [ ] Slice 34: [jedit] materialize artifact from recovered causal basis.

### Gate C: Crash-Window Authority

- [ ] Slice 26: \[Echo]\[jedit] crash runner harness.
- [ ] Slice 27: \[Echo]\[jedit] before-accept-response crash window.
- [ ] Slice 28: \[Echo]\[jedit] after-accept-before-tick crash window.
- [ ] Slice 29: \[Echo]\[jedit] after-receipt-before-reading crash window.
- [ ] Slice 30: \[Echo]\[jedit] after-reading-before-observe crash window.
- [ ] Slice 31: \[Echo]\[jedit] after-observe-before-local-status crash window.

### Gate D: Boundary, Report, And Drift Lock

- [ ] Slice 35: \[Echo]\[jedit] side-effect authorization boundary.
- [ ] Slice 37: \[Echo]\[jedit] malicious adapter negative suite.
- [ ] Slice 38: \[Echo]\[jedit] JSON causal durability report.
- [ ] Slice 39: \[Echo]\[jedit] human causal receipt card.
- [ ] Slice 40: \[Echo]\[jedit] aggregate gate, docs, and drift audit.

## Slice 0: \[Echo]\[jedit] Authority Invariant Charter And Posture State Machine

### 1. Feature Overview & Objectives:

Problem statement: The forty-slice campaign spans two repos and several
machine-readable reports. Without one canonical state model and vocabulary, the
slices will drift into incompatible meanings for accepted, applied, duplicate,
reading, recovery, and source of truth.

Target user/audience: Echo maintainers, jedit adapter authors, QA engineers,
review agents, and future application teams copying the pattern.

Success metrics:

- One documented state model separates intake disposition, lifecycle posture,
  scheduler decision, and evidence health.
- One crashpoint ownership taxonomy separates Echo-owned, jedit-owned, and
  cross-boundary cuts.
- One `source_of_truth: "echo"` rule defines the exact evidence needed before
  the final report may claim Echo authority.

### 2. Scope Definition:

In Scope:

- Add the canonical state model used by Echo and jedit reports.
- Define duplicate/intake disposition separately from recovery posture.
- Define crashpoint ownership taxonomy.
- Define schema-version and source-of-truth rules.

Out of Scope:

- Implementing any runtime behavior.
- Adding jedit adapter code.
- Renaming existing APIs outside documentation needed to prevent drift.

### 3. Detailed User Stories:

- US1: As an Echo maintainer, I want one state model so that recovery APIs do
  not overload one enum with lifecycle, decision, and evidence health.
- US2: As a jedit adapter author, I want duplicate retry disposition separated
  from recovered submission posture so that retry does not invent fake final
  states.
- US3: As a QA engineer, I want crashpoints classified by owner so that tests do
  not make Echo responsible for jedit-local status lifecycle.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                         | When                           | Then                                                                                                     |
| ----- | ----------------------------- | ------------------------------ | -------------------------------------------------------------------------------------------------------- |
| US1   | A recovery report is designed | The report names status fields | It separates `intake.disposition`, `lifecycle.posture`, `decision.result`, and `evidence_health.status`. |
| US2   | A duplicate retry is reported | The JSON is inspected          | Duplicate status appears under intake disposition, not as a recovery posture.                            |
| US3   | A crashpoint is added         | The taxonomy is checked        | The crashpoint declares Echo, jedit, or cross-boundary ownership.                                        |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario         | Fixture/Input           | Expected Result                                                                   |
| ---------------- | ----------------------- | --------------------------------------------------------------------------------- |
| State model lint | Report-schema examples  | No single enum carries retry, lifecycle, decision, and evidence health at once.   |
| Duplicate split  | Duplicate retry example | `duplicate_same_submission` and `applied` can coexist without new posture labels. |
| Crashpoint owner | Crashpoint manifest     | Every crashpoint has exactly one owner class.                                     |

Happy Path Testing:

1. Validate example JSON against the state model.
2. Validate the crashpoint table includes all planned death windows.
3. Confirm the completion bar includes safe retry without duplicate
   application.

Negative/Edge Case Testing:

- A report that sets `source_of_truth: "echo"` while a legacy tripwire fired
  must fail schema/gate validation.
- A report that encodes `already_applied` as lifecycle posture must fail review.
- A jedit-local crashpoint added to Echo's runtime manifest must be rejected or
  moved to the cross-repo runner taxonomy.

Non-Functional Testing:

- Performance: charter validation should be static/doc-schema level and fast.
- Security: source-of-truth rules must fail closed on unknown schema versions.
- Accessibility: tables must render in VitePress and remain readable in plain
  Markdown.

## Slice 1: [Echo] WAL Reality And Stale Claim Cleanup

### 1. Feature Overview & Objectives:

Problem statement: Echo now has filesystem WAL substrate and runtime ACK
witnesses, but some docs and comments still describe the WAL as only an
in-memory foundation. Stale claims make future agents underuse or reimplement
landed WAL work.

Target user/audience: Echo maintainers, external app integrators, and review
agents reading docs before implementation.

Success metrics:

- Zero stale claims that WAL "stops short of filesystem durability" in current
  production-facing docs or module comments.
- One authoritative summary names implemented WAL features and remaining gaps.
- `cargo xtask test-slice runtime-wal-ack` remains the named executable witness.

### 2. Scope Definition:

In Scope:

- Update `crates/warp-core/src/causal_wal.rs` module comment.
- Update BEARING and WAL design signposts to distinguish implemented filesystem
  WAL substrate from remaining product recovery work.
- Add stale-claim grep patterns for old in-memory-only language.

Out of Scope:

- New WAL behavior.
- jedit adapter changes.
- WSC export/import implementation.

### 3. Detailed User Stories:

- US1: As an Echo maintainer, I want WAL docs to match code so that I do not
  plan work around false gaps.
- US2: As a reviewer, I want a stale-claim guard so that old WAL status wording
  does not regress after future edits.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                      | When                                         | Then                                                                       |
| ----- | ------------------------------------------ | -------------------------------------------- | -------------------------------------------------------------------------- |
| US1   | `FilesystemWalStore` exists in `warp-core` | A maintainer reads the module-level WAL docs | The docs say filesystem WAL exists and name remaining gaps separately.     |
| US1   | BEARING names WAL as the durability hill   | A maintainer reads current status            | The status distinguishes substrate completion from jedit product recovery. |
| US2   | A stale phrase is reintroduced             | The stale-claim check runs                   | The check fails with the offending file and phrase.                        |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                | Fixture/Input                                              | Expected Result                         |
| ----------------------- | ---------------------------------------------------------- | --------------------------------------- |
| Current WAL status scan | `rg` over docs and `causal_wal.rs`                         | No stale in-memory-only claim remains.  |
| Witness name check      | `cargo xtask test-slice --list` or documented slice lookup | `runtime-wal-ack` remains discoverable. |
| Link validation         | touched docs                                               | Dead-ref lint passes.                   |

Happy Path Testing:

1. Run stale-claim grep for `stops short of filesystem durability`.
2. Run markdown lint on touched docs.
3. Run dead-ref lint on touched docs.

Negative/Edge Case Testing:

- Reintroduce the stale phrase in a scratch branch and confirm the guard fails.
- Confirm docs still mention WSC and jedit recovery as future where accurate.

Non-Functional Testing:

- Performance: stale-claim grep must complete in under one second on docs and
  `crates/warp-core/src/causal_wal.rs`.
- Security: no new app nouns are introduced into Echo core comments as runtime
  concepts.
- Accessibility: docs use plain headings and link text that render cleanly in
  VitePress.

## Slice 2: [Echo] Accepted-Submission Evidence Contract

### 1. Feature Overview & Objectives:

Problem statement: A returned accepted submission must carry enough generic
evidence for recovery correlation. Today the runtime ACK witness exists, but
the app-facing evidence contract needs to be documented and locked.

Target user/audience: Echo API consumers, jedit adapter authors, and WAL
recovery implementers.

Success metrics:

- Accepted evidence includes stable submission id, canonical envelope digest,
  contract/package identity when present, accepted basis, durability mode, and
  WAL commit evidence ref.
- No accepted response exposes scheduler, tick, or WAL append authority.
- A compile/test fixture proves the evidence contract shape is stable.

### 2. Scope Definition:

In Scope:

- Define the accepted-submission evidence fields.
- Add typed fixtures or ABI schema coverage for those fields, including
  `schema_version` and producer/version metadata where serialized.
- Document that accepted means WAL-committed under the configured durability
  mode.

Out of Scope:

- jedit status mapping.
- Receipt or reading recovery.
- Changing scheduler tick semantics.

### 3. Detailed User Stories:

- US1: As an application adapter, I want accepted submission evidence that can
  be retried and inspected after restart so that crash recovery is deterministic.
- US2: As Echo, I want accepted evidence to be generic so that app nouns do not
  leak into runtime APIs.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                                    | When                           | Then                                                                                                                                     |
| ----- | -------------------------------------------------------- | ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| US1   | A submission is accepted through the WAL-backed ACK path | Echo returns accepted evidence | The response includes submission id, canonical envelope digest, durability mode, writer epoch, transaction id or LSN, and commit digest. |
| US1   | A package-backed operation is accepted                   | Echo returns accepted evidence | The response includes contract/package identity without implying execution.                                                              |
| US2   | A caller inspects accepted evidence                      | The evidence is serialized     | It contains no jedit/editor nouns and no trusted control capability.                                                                     |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                         | Fixture/Input               | Expected Result                                                         |
| -------------------------------- | --------------------------- | ----------------------------------------------------------------------- |
| WAL-backed accepted response     | Valid canonical EINT        | Evidence contains required identifiers and WAL evidence pointer/digest. |
| Package-backed accepted response | Installed generated package | Evidence cites package identity and compatibility metadata.             |
| Authority leak scan              | Serialized evidence fields  | No tick, scheduler, recovery, install, or append capability is exposed. |

Happy Path Testing:

1. Submit a valid canonical intent through the WAL ACK path.
2. Assert returned evidence fields are populated.
3. Serialize and deserialize the evidence.
4. Assert field equality after round trip.

Negative/Edge Case Testing:

- WAL commit failure must return a typed error and no accepted evidence.
- Missing envelope digest must fail before acceptance.
- Durability modes below the product gate bar must not be reported as product
  authority.
- Package identity absence is allowed only for non-package fixtures and must be
  explicit.

Non-Functional Testing:

- Performance: evidence construction must remain O(1) over envelope size except
  for existing digest calculation.
- Security: evidence must not include raw vars bytes unless already allowed by
  the app-safe API.
- Accessibility: CLI/JSON field names must be stable and descriptive.

## Slice 3: [Echo] Generic Recovery Posture Taxonomy

### 1. Feature Overview & Objectives:

Problem statement: External apps need more than found/not-found recovery
answers. Echo must expose generic lifecycle posture, scheduler decision, and
evidence health without turning one enum into a junk drawer.

Target user/audience: jedit recovery adapter authors, Echo CLI users, and
future warp-ttd inspectors.

Success metrics:

- Lifecycle posture includes `not_found`, `accepted_pending`,
  `accepted_deciding` where reconstructable, and `decided`.
- Scheduler decision includes `applied`, `rejected`, `obstructed`, or `none`.
- Evidence health includes `complete`, `incomplete_evidence`,
  `corrupt_or_untrusted`, `missing_retention`, or `redacted`.
- The taxonomy is documented without editor nouns.
- Existing `echo-cli wal submission-posture` output maps to the taxonomy.

### 2. Scope Definition:

In Scope:

- Define generic lifecycle, decision, and evidence-health values plus
  transition meanings.
- Update CLI/read-model JSON contract docs.
- Add unit fixtures for every posture.

Out of Scope:

- jedit editor-facing status names.
- UI wording.
- Full observer-rights/revelation governance.

### 3. Detailed User Stories:

- US1: As a sibling app adapter, I want precise generic recovery posture so that
  product status mapping does not guess.
- US2: As an operator, I want incomplete evidence separated from corruption so
  that repair action is clear.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                 | When                        | Then                                                                                                                           |
| ----- | ------------------------------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| US1   | A submission exists with no receipt   | Recovery posture is queried | Echo returns lifecycle `accepted_pending`, decision `none`, and evidence health `complete` if acceptance evidence is complete. |
| US1   | A submission has a rejected receipt   | Recovery posture is queried | Echo returns lifecycle `decided`, decision `rejected`, and cites receipt evidence.                                             |
| US2   | Required retained material is missing | Recovery posture is queried | Echo returns `incomplete_evidence`, `missing_retention`, or `obstructed` by documented scope.                                  |
| US2   | Commit digest validation fails        | Recovery posture is queried | Echo returns `corrupt_or_untrusted` or blocks recovery.                                                                        |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario         | Fixture/Input                                    | Expected Result                                                        |
| ---------------- | ------------------------------------------------ | ---------------------------------------------------------------------- |
| Empty WAL        | Unknown submission id                            | lifecycle `not_found`.                                                 |
| Acceptance only  | Accepted transaction, no tick                    | lifecycle `accepted_pending`, decision `none`.                         |
| Receipt applied  | Accepted plus applied receipt and state evidence | lifecycle `decided`, decision `applied`.                               |
| Receipt rejected | Accepted plus rejection receipt                  | lifecycle `decided`, decision `rejected`.                              |
| Missing material | Accepted plus missing retained ref               | evidence health `incomplete_evidence` or scoped decision `obstructed`. |
| Corrupt commit   | Bad digest chain                                 | evidence health `corrupt_or_untrusted` or recovery fault.              |

Happy Path Testing:

1. Build one WAL fixture per posture.
2. Query posture through the public read surface.
3. Assert canonical JSON labels and evidence refs.

Negative/Edge Case Testing:

- Unknown future lifecycle, decision, or evidence labels must fail schema
  validation.
- Duplicate retry dispositions must not be encoded as lifecycle postures.
- Mixed receipt states for one submission must return a typed invariant error.

Non-Functional Testing:

- Performance: posture query over rebuilt indexes should be O(log n) or better
  by submission id.
- Security: corrupt/untrusted posture must not leak raw payload bytes.
- Accessibility: CLI labels should be lowercase snake case and stable for
  machine consumers.

## Slice 4: [Echo] Duplicate Submission And Idempotency Law

### 1. Feature Overview & Objectives:

Problem statement: jedit will retry after crash windows. Echo must prevent
double-application while still allowing intentional repeated edits with new
submission identity.

Target user/audience: Echo runtime developers and external app adapter authors.

Success metrics:

- Same submission id plus same envelope returns duplicate-compatible intake
  disposition plus the recovered submission posture.
- Same submission id plus different envelope returns protocol violation.
- New submission id plus same envelope is accepted as new unless explicit
  dedupe policy is present.

### 2. Scope Definition:

In Scope:

- Document idempotency law.
- Add fixtures around duplicate retry and conflicting duplicates.
- Expose generic duplicate disposition in intake APIs without polluting
  recovery posture labels.

Out of Scope:

- Application-specific dedupe policy.
- Text-edit semantic duplicate detection.
- Hidden retry queues.

### 3. Detailed User Stories:

- US1: As a retrying app, I want to resubmit the same operation id safely after
  crash so that Echo does not apply it twice.
- US2: As Echo, I want to reject same-id different-envelope submissions so that
  clients cannot mutate accepted history.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                                 | When            | Then                                                                                                                                                    |
| ----- | ----------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| US1   | Same submission id and envelope were already accepted | The app retries | Echo returns intake disposition `duplicate_same_submission` plus the recovered lifecycle/decision/evidence state without appending a second acceptance. |
| US1   | Same envelope uses a new submission id                | The app submits | Echo treats it as a new submission unless a dedupe policy says otherwise.                                                                               |
| US2   | Same submission id has a different envelope digest    | The app retries | Echo returns intake disposition `conflicting_duplicate` or protocol violation and does not mutate WAL.                                                  |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                    | Fixture/Input             | Expected Result                                                                   |
| --------------------------- | ------------------------- | --------------------------------------------------------------------------------- |
| Duplicate retry pending     | Accepted transaction only | Intake disposition `duplicate_same_submission` plus lifecycle `accepted_pending`. |
| Duplicate retry decided     | Accepted plus receipt     | Intake disposition `duplicate_same_submission` plus final decided posture.        |
| Conflicting duplicate       | Same id, different digest | Protocol violation or intake disposition `conflicting_duplicate`.                 |
| Intentional repeated intent | New id, same digest       | New submission accepted.                                                          |

Happy Path Testing:

1. Submit an intent through WAL ACK.
2. Retry with the same id and envelope digest.
3. Assert no second acceptance transaction is appended.

Negative/Edge Case Testing:

- Retry with same id and different basis must fail.
- Retry with same id and different contract identity must fail.
- Concurrent duplicate submissions must serialize to one accepted result.

Non-Functional Testing:

- Performance: duplicate detection should use submission index lookup, not WAL
  full scan on hot path.
- Security: duplicate conflict errors must not reveal raw envelope contents.
- Accessibility: CLI duplicate labels must be deterministic and documented.

## Slice 5: [Echo] Crashpoint Fixture Contract For External Apps

### 1. Feature Overview & Objectives:

Problem statement: The external-app proof must kill the process at named
boundaries. Echo needs a stable crashpoint vocabulary, but the vocabulary must
distinguish Echo-owned durability boundaries from jedit-owned local lifecycle
boundaries.

Target user/audience: Echo QA, jedit QA, and future app teams building recovery
gates.

Success metrics:

- Crashpoint names exist for Echo-owned, jedit-owned, and cross-boundary death
  windows.
- Echo's runtime manifest contains only Echo-owned boundaries; the cross-repo
  runner owns jedit and handoff boundaries.
- A manifest fixture can be consumed by Rust tests and external scripts.

### 2. Scope Definition:

In Scope:

- Define canonical crashpoint names and ownership classes.
- Add JSON/CLI-readable crashpoint manifest.
- Document expected posture for each crashpoint.

Out of Scope:

- Process-kill runner implementation.
- jedit-specific status mapping.
- Fault injection for object stores.

### 3. Detailed User Stories:

- US1: As QA, I want named crashpoints so that test logs identify the exact
  durability boundary under test.
- US2: As a sibling app, I want crashpoint expectations in generic Echo terms
  so that product mappings remain external.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                     | When                       | Then                                                                                                                                                                     |
| ----- | ----------------------------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| US1   | The crashpoint manifest is queried        | The manifest is serialized | It includes Echo-owned `before_accept_commit`, `after_accept_commit_before_ack`, `after_accept_before_tick`, `after_receipt_before_reading`, and `after_reading_commit`. |
| US1   | The cross-repo runner manifest is queried | The manifest is serialized | It includes cross-boundary and jedit-owned lifecycle crashpoints without adding them to Echo's runtime manifest.                                                         |
| US2   | A crashpoint has expected posture         | jedit reads the manifest   | The expectation is generic Echo posture, not editor wording.                                                                                                             |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario               | Fixture/Input                                       | Expected Result                                  |
| ---------------------- | --------------------------------------------------- | ------------------------------------------------ |
| Manifest serialization | Crashpoint manifest                                 | Stable JSON order, names, and ownership classes. |
| Missing crashpoint     | Remove one required name in fixture                 | Test fails with missing name.                    |
| Wrong owner            | Put jedit-local crashpoint in Echo runtime manifest | Test fails with ownership error.                 |
| App noun scan          | Echo-owned manifest labels                          | No editor-specific nouns appear.                 |

Happy Path Testing:

1. Serialize manifest.
2. Validate against schema.
3. Assert all required names and expected postures exist.

Negative/Edge Case Testing:

- Unknown crashpoint name should be rejected by the runner.
- Duplicate crashpoint names should fail manifest validation.
- Crashpoint expectations must not claim final outcome before scheduler
  decision.
- jedit-owned crashpoints must not require Echo to understand local editor
  status lifecycle.

Non-Functional Testing:

- Performance: manifest construction must be static or O(n) over small fixed
  descriptor set.
- Security: crashpoint injection must be test-only and unavailable from
  application-facing APIs.
- Accessibility: crashpoint names should be readable in CI logs.

## Slice 6: [Echo] Filesystem Accepted-Submission Restart Fixture

### 1. Feature Overview & Objectives:

Problem statement: Echo must prove that a committed accepted submission can be
recovered from the filesystem WAL after restart without transport re-arrival.

Target user/audience: Echo runtime maintainers and jedit integration authors.

Success metrics:

- Fixture writes accepted submission to `FilesystemWalStore`.
- Fresh recovery reports accepted pending or decided posture.
- No app callback or scheduler rerun is used during recovery.

### 2. Scope Definition:

In Scope:

- Add filesystem WAL fixture for accepted submission.
- Reopen WAL root in a fresh runtime/recovery context.
- Assert recovered posture and evidence refs.

Out of Scope:

- jedit process runner.
- Tick decision recovery.
- WSC export.

### 3. Detailed User Stories:

- US1: As Echo, I want accepted submissions to recover from disk so that ACK
  durability is not only in-memory.
- US2: As an external app, I want pending recovery to work without resending
  transport bytes.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                           | When                       | Then                                                                 |
| ----- | ----------------------------------------------- | -------------------------- | -------------------------------------------------------------------- |
| US1   | A committed acceptance exists in filesystem WAL | A fresh recovery runs      | The submission index contains the submission id and envelope digest. |
| US2   | No transport re-arrival occurs                  | The posture API is queried | Echo returns `accepted_pending` from WAL evidence only.              |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                  | Fixture/Input                | Expected Result                        |
| ------------------------- | ---------------------------- | -------------------------------------- |
| Fresh filesystem recovery | WAL root with one acceptance | `accepted_pending`.                    |
| Empty root                | No segments                  | `not_found`.                           |
| Torn acceptance tail      | Frame without commit         | `not_found` or would-truncate by mode. |

Happy Path Testing:

1. Create temp WAL root.
2. Commit acceptance transaction.
3. Drop store/runtime.
4. Reopen and recover.
5. Query posture by submission id.

Negative/Edge Case Testing:

- Commit without matching frame must fail validation.
- Frame with wrong segment id must be rejected.
- Read-only mode must not mutate torn tail.

Non-Functional Testing:

- Performance: single-submission recovery fixture should complete under 100 ms
  locally.
- Security: temp paths must not leak into causal identity assertions.
- Accessibility: failure messages include submission id and WAL root path.

## Slice 7: [Echo] Crash-Before-ACK Retry Witness

### 1. Feature Overview & Objectives:

Problem statement: If Echo commits acceptance and crashes before the app sees
the accepted response, retry must produce stable duplicate posture instead of a
second edit.

Target user/audience: external app adapter authors and Echo WAL developers.

Success metrics:

- Crash-after-commit-before-ACK fixture exists.
- Retry returns already-accepted posture.
- WAL acceptance transaction count remains one.

### 2. Scope Definition:

In Scope:

- Add a fixture simulating committed acceptance with no delivered response.
- Retry same submission id and envelope.
- Assert duplicate posture and transaction count.

Out of Scope:

- jedit UI retry policy.
- Network retry protocol.
- Scheduler decision recovery.

### 3. Detailed User Stories:

- US1: As a retrying app, I want Echo to remember acceptance even if the response
  was lost so that retry is safe.
- US2: As Echo, I want no duplicate WAL records for the same accepted envelope
  so that recovery remains unambiguous.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                           | When                             | Then                                                           |
| ----- | ----------------------------------------------- | -------------------------------- | -------------------------------------------------------------- |
| US1   | Acceptance commit exists but app lacks response | App retries same id and envelope | Echo returns already-accepted pending or decided posture.      |
| US2   | Duplicate retry succeeds                        | WAL is inspected                 | Only one acceptance transaction exists for that submission id. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                          | Fixture/Input                            | Expected Result                                     |
| --------------------------------- | ---------------------------------------- | --------------------------------------------------- |
| Commit-before-ACK loss            | Committed acceptance, no client response | Retry returns duplicate posture.                    |
| Commit-before-ACK then tick       | Acceptance plus receipt                  | Retry returns decided posture.                      |
| Commit-before-ACK corrupt receipt | Acceptance plus corrupt receipt          | Retry returns accepted/incomplete evidence posture. |

Happy Path Testing:

1. Commit acceptance transaction.
2. Simulate client response loss by discarding returned handle.
3. Reopen/recover.
4. Submit same id/envelope.
5. Assert duplicate posture.

Negative/Edge Case Testing:

- Different envelope after response loss must be protocol violation.
- Missing canonical digest must fail before duplicate lookup.
- Concurrent retries must not append duplicate transactions.

Non-Functional Testing:

- Performance: duplicate lookup should not scan all WAL frames.
- Security: duplicate response must not expose raw payload.
- Accessibility: CLI duplicate posture should explain "same submission id and
  envelope already accepted."

## Slice 8: [Echo] Conflicting Duplicate Rejection Witness

### 1. Feature Overview & Objectives:

Problem statement: Same submission id with different payload or basis is an
attempt to rewrite accepted history. Echo must reject it deterministically.

Target user/audience: Echo security reviewers, app adapter implementers, and
QA.

Success metrics:

- Same id/different digest fixture fails closed.
- Same id/different contract identity fixture fails closed.
- Error is typed as protocol violation, not ordinary rejection.

### 2. Scope Definition:

In Scope:

- Add conflict fixtures for envelope digest, basis digest, and contract identity.
- Ensure no WAL append occurs on conflicting duplicate.
- Document distinction from lawful scheduler rejection.

Out of Scope:

- App-specific semantic dedupe.
- User-facing retry UX.
- Conflict-resolution UI.

### 3. Detailed User Stories:

- US1: As Echo, I want conflicting duplicates to fail before mutation so that
  accepted history cannot be rewritten.
- US2: As an app adapter, I want a typed error so that I can surface a protocol
  bug rather than retrying forever.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                    | When                  | Then                                                        |
| ----- | ---------------------------------------- | --------------------- | ----------------------------------------------------------- |
| US1   | Submission id already maps to digest A   | Retry uses digest B   | Echo returns protocol violation and appends no WAL records. |
| US1   | Submission id already maps to contract X | Retry uses contract Y | Echo returns protocol violation.                            |
| US2   | Protocol violation occurs                | Error is serialized   | The code is distinct from lawful rejection and obstruction. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                   | Fixture/Input                       | Expected Result     |
| -------------------------- | ----------------------------------- | ------------------- |
| Different envelope digest  | Same id, different bytes            | Protocol violation. |
| Different basis digest     | Same id, same vars, different basis | Protocol violation. |
| Different package identity | Same id, different contract package | Protocol violation. |

Happy Path Testing:

1. Accept original submission.
2. Retry same id with changed field.
3. Assert typed protocol error.
4. Inspect WAL frame/commit count unchanged.

Negative/Edge Case Testing:

- Empty submission id must fail validation before duplicate law.
- Malformed digest must fail schema validation.
- Protocol violation must not quarantine scheduler heads.

Non-Functional Testing:

- Performance: conflict detection uses index lookup.
- Security: error does not leak original payload.
- Accessibility: error text names conflicting field class.

## Slice 9: [Echo] App-Safe Submission Posture Read Surface

### 1. Feature Overview & Objectives:

Problem statement: External apps should ask Echo for submission posture without
depending on WAL internals. The current CLI names WAL directly; the app-safe
surface should be recovery/posture oriented.

Target user/audience: jedit adapter authors and future JS/WASM/browser clients.

Success metrics:

- A generic posture read surface exists outside trusted runtime control.
- It returns posture without exposing WAL append/recovery authority.
- CLI can remain diagnostic while adapters target the app-safe port.

### 2. Scope Definition:

In Scope:

- Define app-safe posture request/response model.
- Bridge existing WAL-backed posture data into that model.
- Keep trusted recovery mutation unavailable.

Out of Scope:

- Removing diagnostic `echo-cli wal` commands.
- jedit status mapping.
- Browser package publishing.

### 3. Detailed User Stories:

- US1: As jedit, I want to query submission posture through a recovery port so
  that I do not treat WAL as my public API.
- US2: As Echo, I want posture queries to be read-only so that app code cannot
  recover, truncate, or append.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                            | When                        | Then                                                         |
| ----- | ------------------------------------------------ | --------------------------- | ------------------------------------------------------------ |
| US1   | A submission id and digest are provided          | The posture port is queried | Echo returns generic posture and evidence refs.              |
| US2   | App code calls the posture port                  | The call executes           | No trusted recovery or WAL mutation capability is available. |
| US2   | A caller requests truncation through posture API | The request is attempted    | It is impossible by type/API surface.                        |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario           | Fixture/Input   | Expected Result              |
| ------------------ | --------------- | ---------------------------- |
| Known submission   | id plus digest  | Correct posture.             |
| Unknown submission | id plus digest  | `not_found`.                 |
| Authority probe    | app-safe client | No mutation methods exposed. |

Happy Path Testing:

1. Build WAL-backed runtime fixture.
2. Query posture through app-safe API.
3. Assert response matches `echo-cli wal submission-posture`.

Negative/Edge Case Testing:

- Missing digest returns validation error.
- Mismatched digest returns protocol/conflict posture.
- Malformed id returns validation error without WAL scan.

Non-Functional Testing:

- Performance: posture query should use recovered indexes.
- Security: no raw WAL path required in app-safe request.
- Accessibility: response field names should not require WAL expertise.

## Slice 10: [Echo] Recovery Certificate Submission Posture Counts

### 1. Feature Overview & Objectives:

Problem statement: Operators need restart evidence summarizing accepted,
pending, decided, rejected, obstructed, and corrupt work. Recovery certificates
must report these counts consistently.

Target user/audience: Echo operators, jedit gate reports, and debug tools.

Success metrics:

- Recovery certificate includes submission posture counts.
- Counts reconcile with recovered indexes.
- CLI JSON exposes counts for release-gate reports.

### 2. Scope Definition:

In Scope:

- Extend recovery certificate read model.
- Add count reconciliation tests.
- Document count semantics.

Out of Scope:

- Per-editor status counts.
- Full WSC archive manifest.
- UI dashboards.

### 3. Detailed User Stories:

- US1: As an operator, I want restart summaries so that I can see whether Echo
  recovered pending or obstructed work.
- US2: As a test gate, I want counts to reconcile with indexes so that reports
  cannot lie.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                | When                     | Then                                                                         |
| ----- | ------------------------------------ | ------------------------ | ---------------------------------------------------------------------------- |
| US1   | Recovery replays mixed submissions   | Certificate is produced  | Counts include accepted pending, applied, rejected, obstructed, and faulted. |
| US2   | Index contains N applied submissions | Certificate is validated | Applied count equals N.                                                      |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario            | Fixture/Input              | Expected Result                          |
| ------------------- | -------------------------- | ---------------------------------------- |
| Mixed recovery      | Pending, applied, rejected | Counts match fixture.                    |
| Obstructed material | Missing retained ref       | Obstruction count increments.            |
| Corrupt commit      | Bad digest                 | Fault count or blocked recovery posture. |

Happy Path Testing:

1. Build WAL with mixed states.
2. Recover.
3. Assert certificate count fields.
4. Cross-check index queries.

Negative/Edge Case Testing:

- Count overflow must be impossible or typed.
- Duplicate submission must not double count.
- Corrupt history must not emit clean counts.

Non-Functional Testing:

- Performance: count construction should be linear in recovered transaction
  count during recovery, not per query.
- Security: counts do not expose payload.
- Accessibility: CLI count labels align with posture labels.

## Slice 11: [Echo] Receipt Correlation Recovery Witness

### 1. Feature Overview & Objectives:

Problem statement: Accepted input is not enough. Echo must recover the scheduler
decision and correlate it back to the submission and ticket.

Target user/audience: external app adapters, Echo runtime maintainers, and
debuggers.

Success metrics:

- Receipt-by-submission index rebuilds from WAL.
- Ticket-by-submission and receipt-by-ticket links rebuild.
- Applied decision claims require receipt evidence plus committed state/basis
  evidence.
- Missing/mismatched correlation or missing state evidence becomes invariant
  error or incomplete evidence.

### 2. Scope Definition:

In Scope:

- Add recovery fixture for submission-to-receipt correlation.
- Validate receipt/ticket/submission digest links.
- Expose correlation evidence in generic posture response.
- Define that a scheduler receipt alone is not sufficient to claim
  `decision.result: "applied"` without committed state/basis evidence.

Out of Scope:

- jedit-specific receipt names.
- Observer reading recovery.
- Full TTD UI integration.

### 3. Detailed User Stories:

- US1: As an app, I want to know whether accepted work was decided by the
  scheduler so that I can stop showing pending status.
- US2: As Echo, I want receipt correlation to be rebuilt from committed history
  so that recovery does not rerun scheduler logic.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                             | When                              | Then                                                                |
| ----- | ------------------------------------------------- | --------------------------------- | ------------------------------------------------------------------- |
| US1   | Accepted submission has committed receipt         | Posture is queried after recovery | Response includes receipt evidence and final decision.              |
| US1   | Receipt says applied but state evidence is absent | Posture is queried after recovery | Echo reports incomplete evidence rather than clean applied posture. |
| US2   | Runtime is recovering                             | Receipt indexes are rebuilt       | No scheduler callback or app callback is invoked.                   |
| US2   | Receipt correlation is missing                    | Recovery validates                | Echo returns incomplete evidence or invariant error by scope.       |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                      | Fixture/Input                        | Expected Result                         |
| ----------------------------- | ------------------------------------ | --------------------------------------- |
| Applied receipt               | Acceptance plus applied tick         | `applied` with receipt id/digest.       |
| Applied receipt missing state | Receipt without state/basis evidence | Incomplete evidence, not clean applied. |
| Rejected receipt              | Acceptance plus rejection tick       | `rejected` with blocker evidence.       |
| Missing correlation           | Receipt lacks submission link        | Incomplete/invariant posture.           |

Happy Path Testing:

1. Commit acceptance.
2. Commit tick receipt and correlation records.
3. Recover.
4. Query by submission id.

Negative/Edge Case Testing:

- Receipt references unknown ticket.
- Ticket references unknown submission.
- Multiple receipts claim final decision for one submission.
- Applied receipt with missing state evidence must not become clean applied.

Non-Functional Testing:

- Performance: correlation lookup should be index-backed.
- Security: blocker evidence should not reveal hidden payload.
- Accessibility: JSON names receipt and ticket fields explicitly.

## Slice 12: [Echo] Tick Commit-Before-Publish Failure Injection

### 1. Feature Overview & Objectives:

Problem statement: A visible receipt must imply recoverable committed history.
Echo needs injected failures proving tick WAL failure cannot leak half-visible
outcomes.

Target user/audience: Echo runtime developers and release gate reviewers.

Success metrics:

- Injected tick WAL failure prevents receipt publication.
- Runtime/provenance state rolls back or enters typed fault posture.
- No final outcome is visible without committed receipt evidence.

### 2. Scope Definition:

In Scope:

- Add failure injection around tick WAL commit.
- Assert no product-facing outcome leaks.
- Preserve existing fault/quarantine doctrine.

Out of Scope:

- New scheduler policy.
- App retry UX.
- Object-store-specific injection.

### 3. Detailed User Stories:

- US1: As Echo, I want tick WAL failures to be atomic so that observers never
  see uncommitted outcomes.
- US2: As an operator, I want fault posture when rollback cannot preserve normal
  execution.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                            | When                       | Then                                           |
| ----- | -------------------------------- | -------------------------- | ---------------------------------------------- |
| US1   | Tick decision is staged          | WAL commit fails           | No receipt is published to app-facing indexes. |
| US1   | Tick WAL commit succeeds         | Receipt publication occurs | Visible receipt has committed WAL evidence.    |
| US2   | Rollback cannot complete cleanly | Fault is recorded          | Runtime enters trusted recovery posture.       |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                      | Fixture/Input           | Expected Result                  |
| ----------------------------- | ----------------------- | -------------------------------- |
| Commit failure before publish | Injected WAL error      | No visible receipt.              |
| Commit success before publish | Normal tick             | Receipt visible and recoverable. |
| Rollback failure              | Injected rollback error | Runtime fault posture.           |

Happy Path Testing:

1. Stage accepted submission.
2. Run scheduler with WAL success.
3. Assert receipt visible after commit.

Negative/Edge Case Testing:

- Inject failure after state delta but before commit.
- Inject failure after commit but before index publish.
- Ensure repeated recovery does not invent missing receipt.

Non-Functional Testing:

- Performance: injection hooks must be test-only and disabled in production.
- Security: app code cannot trigger injection through public API.
- Accessibility: fault report names boundary where failure occurred.

## Slice 13: [Echo] Bounded Reading Identity Recovery Witness

### 1. Feature Overview & Objectives:

Problem statement: jedit must observe text through bounded reading evidence,
not direct memory. Echo must recover reading identity tied to causal basis.

Target user/audience: QueryView implementers, external apps, and retained
evidence reviewers.

Success metrics:

- Query reading identity survives recovery.
- Reading identity binds query id, vars digest, basis digest, aperture digest,
  observer plan, and package evidence.
- Reading evidence reports `reading_source` as retained, read-only rederived, or
  unavailable.
- Reading evidence reports `reading_authority` as committed reading, read-only
  rederivation, or none.
- Reading recovery remains read-only.

### 2. Scope Definition:

In Scope:

- Add recovery fixture for retained QueryView reading identity.
- Rebuild reading lookup by semantic coordinate.
- Validate basis/aperture/budget identity.
- Define retained versus rederived reading posture in the recovery response.

Out of Scope:

- jedit text rendering.
- Full observer-rights lattice.
- Streaming subscriptions.

### 3. Detailed User Stories:

- US1: As an app, I want recovered reading identity so that I can trust what
  basis was observed.
- US2: As Echo, I want reading recovery to remain observer-relative and
  read-only.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                             | When                          | Then                                                                                                             |
| ----- | ------------------------------------------------- | ----------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| US1   | A bounded reading was retained                    | Recovery rebuilds indexes     | Reading identity is queryable by semantic coordinate.                                                            |
| US1   | Reading payload is rederived from recovered basis | Recovery response is emitted  | Response states `reading_source: "rederived_from_basis"` and `reading_authority: "echo_read_only_rederivation"`. |
| US1   | Reading evidence is unavailable                   | Recovery response is emitted  | Response states reading unavailable and does not substitute local memory.                                        |
| US1   | Reading vars digest changes                       | Lookup is attempted           | Echo does not return the old reading as a match.                                                                 |
| US2   | Reading recovery runs                             | Recovery applies transactions | No handler mutation or tick occurs.                                                                              |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                          | Fixture/Input                 | Expected Result              |
| --------------------------------- | ----------------------------- | ---------------------------- |
| Retained reading                  | QueryView reading transaction | Identity recovered.          |
| Rederived reading                 | Recovered basis and observer  | Read-only rederived posture. |
| Same payload different coordinate | Two readings                  | Both remain distinct.        |
| Missing reading payload           | Retained envelope only        | Missing-retention posture.   |

Happy Path Testing:

1. Run installed QueryView.
2. Retain reading envelope and payload refs.
3. Recover from WAL.
4. Query reading identity and payload posture.

Negative/Edge Case Testing:

- Wrong basis digest must not match.
- Over-budget reading payload must obstruct.
- Observer plan mismatch must fail lookup.
- Local-memory reading substitution is forbidden.

Non-Functional Testing:

- Performance: semantic reading lookup should avoid payload scan.
- Security: redacted payload posture must not leak bytes.
- Accessibility: CLI JSON should clearly separate identity and payload posture.

## Slice 14: [Echo] Retained Material Recovery Obstruction Witness

### 1. Feature Overview & Objectives:

Problem statement: Missing retained material must become typed obstruction, not
empty success. Recovery needs scope-specific behavior for payload, receipt,
reading, checkpoint, and diagnostic material.

Target user/audience: Echo recovery maintainers and app adapters that display
blocked status.

Success metrics:

- Missing material scope matrix is executable.
- App-safe reads distinguish missing, redacted, corrupt, and obstructed.
- Recovery does not silently drop committed references.

### 2. Scope Definition:

In Scope:

- Add missing-material fixtures for submission payload, receipt, state delta,
  reading envelope, reading payload, and diagnostic trace.
- Assert scoped recovery posture.
- Document app-safe obstruction mapping.

Out of Scope:

- Building a full GC policy.
- WSC archival repair.
- User-facing remediation UI.

### 3. Detailed User Stories:

- US1: As recovery, I want missing committed material to produce typed posture
  so that history is honest.
- US2: As an app, I want missing reading payload separated from rejected work so
  that I can show blocked evidence status.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                                      | When                | Then                                                        |
| ----- | ---------------------------------------------------------- | ------------------- | ----------------------------------------------------------- |
| US1   | A committed record references missing required state delta | Recovery runs       | Echo blocks or faults by documented global scope.           |
| US2   | A reading payload is missing but envelope exists           | App queries reading | Echo returns missing-retention posture, not empty payload.  |
| US2   | Diagnostic material is missing                             | Recovery runs       | Causal recovery remains valid with diagnostic-loss posture. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                   | Fixture/Input          | Expected Result                         |
| -------------------------- | ---------------------- | --------------------------------------- |
| Missing submission payload | Acceptance ref missing | Submission-level obstruction.           |
| Missing receipt            | Decision ref missing   | Incomplete decision evidence.           |
| Missing state delta        | Runtime delta missing  | Global recovery fault or blocked scope. |
| Missing diagnostic         | Optional trace missing | Diagnostic loss only.                   |

Happy Path Testing:

1. Build one valid retained-material fixture.
2. Remove one referenced material item.
3. Recover or query.
4. Assert expected obstruction scope.

Negative/Edge Case Testing:

- Corrupt material digest must not be treated as missing.
- Redacted-by-policy must not be treated as corruption.
- Cache hit with wrong semantic coordinate must fail.

Non-Functional Testing:

- Performance: obstruction lookup should not require full CAS scan.
- Security: policy-hidden evidence must stay hidden.
- Accessibility: report should name material family and scope.

## Slice 15: [Echo] Receipt-To-Reading Causal Chain Read Model

### 1. Feature Overview & Objectives:

Problem statement: The external-app gate must prove more than text changed. It
must prove submission id to receipt to causal basis to bounded reading.

Target user/audience: jedit release gate, operators, and debug tooling.

Success metrics:

- Read model returns chain: submission, ticket, receipt, basis, reading.
- Chain is generic and contains no editor nouns.
- Complete applied chain requires receipt evidence, committed state/basis
  evidence, and retained or read-only rederived reading evidence.
- Missing links produce typed incomplete evidence.

### 2. Scope Definition:

In Scope:

- Define generic chain response shape.
- Populate chain from recovered indexes.
- Add fixtures for complete and incomplete chains.
- Define basis mismatch as a first-class obstruction in the chain.

Out of Scope:

- jedit status wording.
- TTD visual UI.
- Full graph export.

### 3. Detailed User Stories:

- US1: As a release gate, I want a causal chain so that the proof is stronger
  than comparing final payloads.
- US2: As Echo, I want incomplete chains to be explicit so that reports cannot
  imply certainty from partial evidence.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                              | When               | Then                                                                                      |
| ----- | -------------------------------------------------- | ------------------ | ----------------------------------------------------------------------------------------- |
| US1   | A submission is applied and observed               | Chain is queried   | Response includes submission id, receipt id, basis digest, reading id, and evidence refs. |
| US2   | Receipt exists but reading is absent               | Chain is queried   | Response reports incomplete reading evidence without failing the receipt.                 |
| US2   | Reading basis does not match receipt outcome       | Chain is validated | Echo returns chain inconsistency.                                                         |
| US2   | Submission basis no longer matches recovered state | Chain is validated | Echo reports obstruction by basis mismatch or stale basis, not generic rejection.         |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario               | Fixture/Input                         | Expected Result             |
| ---------------------- | ------------------------------------- | --------------------------- |
| Complete chain         | Applied receipt plus reading          | Complete chain.             |
| Missing reading        | Applied receipt only                  | Incomplete reading posture. |
| Basis mismatch         | Receipt basis A, reading basis B      | Chain inconsistency.        |
| Stale submission basis | Submission basis A, recovered state B | Basis-mismatch obstruction. |

Happy Path Testing:

1. Submit intent.
2. Tick to receipt.
3. Query bounded reading.
4. Recover.
5. Query chain.

Negative/Edge Case Testing:

- Unknown submission returns `not_found`.
- Multiple readings must require coordinate/basis selection.
- Stale basis must not be presented as current.
- Basis mismatch must not be collapsed into rejected work.

Non-Functional Testing:

- Performance: chain lookup should be bounded by indexed ids.
- Security: raw payload remains behind retained material policy.
- Accessibility: chain JSON should be stable and easy to diff.

## Slice 16: [Echo] Causal Commit Evidence JSON Contract

### 1. Feature Overview & Objectives:

Problem statement: jedit and warp-ttd need machine-readable commit evidence
without parsing raw WAL segments. Echo must publish a stable JSON contract.

Target user/audience: jedit adapters, warp-ttd read models, CLI users, and CI
gates.

Success metrics:

- JSON schema names source, posture, durability mode, writer epoch, LSN,
  transaction id, commit digest, checkpoint digest, and obstruction.
- JSON schema includes `schema_version`, `producer`, `producer_version`, and
  compatibility metadata.
- Schema is app-noun-free.
- Roundtrip tests lock field names.

### 2. Scope Definition:

In Scope:

- Define `CausalCommitEvidence` JSON.
- Add serialization/deserialization tests.
- Link it from recovery posture and chain read models.
- Add golden fixtures consumed by jedit and future warp-ttd evidence
  projection.

Out of Scope:

- Raw WAL segment parser for warp-ttd.
- Browser UI rendering.
- Cryptographic signing.

### 3. Detailed User Stories:

- US1: As an inspector, I want commit evidence anchors so that I can explain
  what survived.
- US2: As Echo, I want tooling to consume projections, not WAL internals.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                         | When                      | Then                                                                               |
| ----- | ----------------------------- | ------------------------- | ---------------------------------------------------------------------------------- |
| US1   | A committed acceptance exists | Evidence JSON is produced | It contains transaction id, LSN, writer epoch, commit digest, and durability mode. |
| US1   | Evidence JSON is produced     | Schema is validated       | `schema_version`, producer metadata, and compatibility metadata are present.       |
| US2   | A debugger consumes evidence  | It receives JSON          | It does not need segment parsing or recovery authority.                            |
| US2   | Evidence is absent            | JSON is produced          | Posture is `absent` with reason, not omitted.                                      |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario            | Fixture/Input         | Expected Result                               |
| ------------------- | --------------------- | --------------------------------------------- |
| Present evidence    | Committed transaction | `present` evidence JSON.                      |
| Absent evidence     | No WAL-backed claim   | `absent` with reason.                         |
| Obstructed evidence | Missing material      | `obstructed` with scope.                      |
| Unknown schema      | Future schema fixture | Fail closed unless compatibility rule exists. |

Happy Path Testing:

1. Build committed WAL fixture.
2. Generate evidence JSON.
3. Validate schema and stable fields.

Negative/Edge Case Testing:

- Unknown source value fails schema validation.
- Missing commit digest fails validation.
- Raw absolute path must not be required for identity.
- Missing schema or producer metadata fails validation.

Non-Functional Testing:

- Performance: evidence projection should be index-backed.
- Security: evidence JSON must not leak canonical payload bytes.
- Accessibility: field names must be lower camel or snake case consistently
  according to existing Echo CLI convention.

## Slice 17: [Echo] Generic External-App Recovery Gate Command

### 1. Feature Overview & Objectives:

Problem statement: jedit needs a stable way to call Echo recovery proof from CI.
Echo should expose a generic command or test-slice that exercises external-app
durability without knowing jedit nouns.

Target user/audience: release engineers, jedit CI, and external app authors.

Success metrics:

- One command emits generic recovery gate JSON.
- Command accepts generic ids/digests or fixture input.
- Command cannot mutate trusted runtime state in read-only mode.

### 2. Scope Definition:

In Scope:

- Add or document generic recovery gate command/test-slice.
- Ensure JSON includes posture, causal chain, commit evidence, and certificate.
- Make command usable by sibling repo scripts.

Out of Scope:

- jedit editor-specific wrapper.
- Full process-kill orchestration.
- WSC packaging.

### 3. Detailed User Stories:

- US1: As jedit CI, I want a boring Echo command that returns generic recovery
  evidence so that jedit can map it externally.
- US2: As Echo, I want the command to stay read-only unless explicitly running
  a test fixture.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                     | When                            | Then                                                       |
| ----- | ----------------------------------------- | ------------------------------- | ---------------------------------------------------------- |
| US1   | A WAL root and submission id are supplied | The command runs                | JSON includes posture, commit evidence, and chain status.  |
| US2   | Read-only mode is selected                | The WAL has an uncommitted tail | Command reports would-truncate and does not rewrite files. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario             | Fixture/Input      | Expected Result         |
| -------------------- | ------------------ | ----------------------- |
| Known submission     | WAL root plus id   | Full generic report.    |
| Unknown submission   | WAL root plus id   | `not_found`.            |
| Dirty tail read-only | WAL root with tail | Would-truncate posture. |

Happy Path Testing:

1. Generate fixture WAL root.
2. Run command with submission id and digest.
3. Validate JSON against schema.

Negative/Edge Case Testing:

- Missing WAL root returns typed IO/config error.
- Bad JSON output path fails before recovery.
- Read-only mode does not truncate.

Non-Functional Testing:

- Performance: command should complete under one second on small fixture roots.
- Security: command must not expose trusted runtime control.
- Accessibility: command output should be line-stable for CI logs.

## Slice 18: [jedit] Echo Recovery Port Interface

### 1. Feature Overview & Objectives:

Problem statement: jedit must not depend conceptually on WAL internals. It
needs an app-owned recovery port that asks Echo for generic posture.

Target user/audience: jedit application code and adapter implementers.

Success metrics:

- `EchoRecoveryPort` or equivalent exists in jedit ports.
- Port methods use generic Echo ids/digests and return generic posture payloads.
- Application code does not import Echo CLI/process details.

### 2. Scope Definition:

In Scope:

- Define jedit port interface.
- Add TypeScript types for request/response.
- Add fake adapter fixture for unit tests.

Out of Scope:

- Real Echo command wiring.
- UI rendering.
- Text export.

### 3. Detailed User Stories:

- US1: As jedit application code, I want a recovery port so that I can ask for
  Echo posture without knowing WAL details.
- US2: As a test author, I want a fake recovery adapter so that mapping logic is
  deterministic.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                     | When                       | Then                                                |
| ----- | ----------------------------------------- | -------------------------- | --------------------------------------------------- |
| US1   | jedit has submission id and digest        | It calls the recovery port | It receives generic Echo posture data.              |
| US1   | Application code imports recovery support | Type checking runs         | It imports the port, not CLI or filesystem modules. |
| US2   | Fake adapter is configured with posture   | Test queries port          | Fake returns deterministic posture payload.         |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario             | Fixture/Input              | Expected Result                           |
| -------------------- | -------------------------- | ----------------------------------------- |
| Type contract        | `tsc`                      | Port types compile.                       |
| Fake posture         | `accepted_pending` fixture | Port returns fixture payload.             |
| Forbidden dependency | app source scan            | No direct CLI/process import in app code. |

Happy Path Testing:

1. Instantiate fake recovery adapter.
2. Query a known submission.
3. Assert generic posture payload.

Negative/Edge Case Testing:

- Missing submission id returns validation error.
- Unknown posture label fails type/schema guard.
- Adapter timeout maps to recovery-unavailable posture.

Non-Functional Testing:

- Performance: fake adapter tests complete without spawning processes.
- Security: port exposes no scheduler/tick methods.
- Accessibility: errors include field names for test logs.

## Slice 19: [jedit] Echo Recovery Adapter Implementation

### 1. Feature Overview & Objectives:

Problem statement: The jedit port needs a concrete adapter that can call Echo's
generic recovery surface while keeping process/path details adapter-private.

Target user/audience: jedit CI, agents, and integration test authors.

Success metrics:

- Adapter can call Echo CLI or library surface through a path/process adapter.
- App code remains path-blind.
- Adapter returns typed generic posture response.

### 2. Scope Definition:

In Scope:

- Implement adapter behind `EchoRecoveryPort`.
- Keep sibling Echo checkout path behind configuration.
- Add integration test with mocked process output.

Out of Scope:

- Requiring Echo checkout for default unit tests.
- UI status display.
- Raw WAL parsing in jedit.

### 3. Detailed User Stories:

- US1: As jedit CI, I want the adapter to call Echo recovery evidence so that
  the release gate checks real Echo truth.
- US2: As jedit app code, I want filesystem/process details hidden so that the
  app remains hexagonal.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                | When                    | Then                                             |
| ----- | ------------------------------------ | ----------------------- | ------------------------------------------------ |
| US1   | Adapter receives Echo command config | It queries posture      | It parses generic Echo JSON into typed response. |
| US2   | App code uses recovery               | Imports are checked     | App code depends only on the port.               |
| US2   | Echo command fails                   | Adapter handles failure | It returns typed unavailable/error posture.      |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario       | Fixture/Input     | Expected Result                       |
| -------------- | ----------------- | ------------------------------------- |
| Mocked success | Echo JSON fixture | Typed posture response.               |
| Mocked failure | Non-zero exit     | Recovery unavailable with diagnostic. |
| Malformed JSON | Invalid output    | Typed parse obstruction.              |

Happy Path Testing:

1. Provide mocked Echo command output.
2. Call adapter.
3. Assert parsed posture, evidence, and chain fields.

Negative/Edge Case Testing:

- Timeout returns typed adapter failure.
- Missing executable returns configuration error.
- Adapter must not retry mutation commands.

Non-Functional Testing:

- Performance: mocked adapter tests avoid real process spawn.
- Security: command args must avoid shell interpolation.
- Accessibility: diagnostics include command name but not secrets.

## Slice 20: [jedit] Generic-To-Editor Posture Mapping

### 1. Feature Overview & Objectives:

Problem statement: Echo returns generic posture. jedit must map it into editor
language without teaching Echo editor nouns.

Target user/audience: jedit application code and UI/report consumers.

Success metrics:

- Mapping covers every Echo posture.
- Unknown posture fails closed.
- Mapping is documented as jedit-owned.

### 2. Scope Definition:

In Scope:

- Implement mapping from Echo posture to jedit edit status.
- Add exhaustive tests.
- Update jedit docs.

Out of Scope:

- Changing Echo posture labels.
- UI styling.
- Product notification policy.

### 3. Detailed User Stories:

- US1: As jedit, I want `accepted_pending` mapped to `edit_pending` so that
  users see editor language.
- US2: As a maintainer, I want mapping exhaustiveness so that new Echo postures
  cannot be silently ignored.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                      | When              | Then                                           |
| ----- | -------------------------- | ----------------- | ---------------------------------------------- |
| US1   | Echo returns `applied`     | jedit maps status | Result is `edit_applied`.                      |
| US1   | Echo returns `obstructed`  | jedit maps status | Result is `edit_blocked` with reason.          |
| US2   | Echo returns unknown label | Mapping runs      | jedit returns typed unsupported-posture error. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario   | Fixture/Input          | Expected Result             |
| ---------- | ---------------------- | --------------------------- |
| Pending    | `accepted_pending`     | `edit_pending`.             |
| Applied    | `applied`              | `edit_applied`.             |
| Rejected   | `rejected`             | `edit_rejected`.            |
| Incomplete | `incomplete_evidence`  | `edit_recovery_incomplete`. |
| Corrupt    | `corrupt_or_untrusted` | `echo_recovery_error`.      |

Happy Path Testing:

1. Feed each known posture fixture into mapper.
2. Assert exact editor status.
3. Assert evidence fields are preserved for reports.

Negative/Edge Case Testing:

- Unknown posture label fails closed.
- Missing reason for obstructed posture returns validation error.
- Corrupt/untrusted never maps to applied.

Non-Functional Testing:

- Performance: mapping is pure and O(1).
- Security: mapper does not expose raw Echo payloads.
- Accessibility: status labels are understandable in JSON reports.

## Slice 21: [jedit] Stable Edit Submission Identity

### 1. Feature Overview & Objectives:

Problem statement: Retry safety depends on stable app-side submission identity.
jedit needs deterministic operation ids and envelope digests for edit intents.

Target user/audience: jedit adapter authors and recovery gate tests.

Success metrics:

- Edit submissions include stable client operation id.
- Envelope digest is recorded in witness/recovery report.
- Repeated same operation reuses identity only for retry, not for new edits.

### 2. Scope Definition:

In Scope:

- Define jedit client operation id rules.
- Attach id/digest to TextBufferSessionPort edit submission.
- Add tests for retry versus intentional repeated edit.

Out of Scope:

- Full collaborative user identity.
- Undo/redo semantic model.
- Echo id generation changes.

### 3. Detailed User Stories:

- US1: As jedit, I want stable edit identity so that crash retry can correlate
  with Echo recovery.
- US2: As a user, I want two intentional identical edits to remain two edits
  when submitted with distinct operation ids.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                           | When                 | Then                                                                |
| ----- | ----------------------------------------------- | -------------------- | ------------------------------------------------------------------- |
| US1   | A retry reuses client operation id and envelope | jedit submits        | Echo receives same submission identity.                             |
| US2   | User performs the same edit twice intentionally | jedit submits both   | The submissions use distinct operation ids.                         |
| US2   | Same id with changed envelope is attempted      | jedit preflight runs | jedit rejects before sending or Echo rejects as protocol violation. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario              | Fixture/Input         | Expected Result              |
| --------------------- | --------------------- | ---------------------------- |
| Retry same edit       | Same operation id     | Same submission identity.    |
| Intentional duplicate | New operation id      | New submission.              |
| Mutated retry         | Same id, changed vars | Validation/protocol failure. |

Happy Path Testing:

1. Create edit intent.
2. Generate submission identity.
3. Retry with same persisted identity.
4. Assert identity stable.

Negative/Edge Case Testing:

- Missing operation id in release-gate mode fails.
- Non-deterministic ids in replay fixture fail.
- Id collision with different envelope fails.

Non-Functional Testing:

- Performance: id derivation should not hash large text more than necessary.
- Security: operation ids should not include raw file paths unless policy allows.
- Accessibility: report includes both id and digest.

## Slice 22: [jedit] Recovery Evidence Report Fields

### 1. Feature Overview & Objectives:

Problem statement: jedit's report must carry the evidence needed to prove Echo
authority: submission, envelope, contract identity, receipt, reading, and
recovery posture.

Target user/audience: reviewers, CI, and release notes authors.

Success metrics:

- Report includes submission id, envelope digest, contract identity, Echo
  posture, receipt evidence, reading evidence, and source-of-truth marker.
- Report includes `schema_version`, producer metadata, and compatibility
  metadata.
- Report explicitly states whether durable replay is available.
- Report states `source_of_truth: "echo"` only when all gate conditions are
  satisfied; otherwise it reports `incomplete`, `unknown`, or
  `local_fallback_detected`.
- Report schema is tested.

### 2. Scope Definition:

In Scope:

- Extend jedit JSON report schema.
- Add fixtures for present, pending, and unavailable recovery evidence.
- Ensure fields are generated through ports/adapters.
- Include reading source/authority, evidence health, tripwire status, and
  source-of-truth conclusion.

Out of Scope:

- Pretty HTML report.
- Human receipt card.
- WSC export.

### 3. Detailed User Stories:

- US1: As a reviewer, I want the jedit report to show the complete Echo evidence
  chain so that the proof is inspectable.
- US2: As jedit, I want missing durable replay labeled honestly so that reports
  do not overclaim.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                                     | When                   | Then                                                         |
| ----- | --------------------------------------------------------- | ---------------------- | ------------------------------------------------------------ |
| US1   | jedit runs recovery gate                                  | JSON report is emitted | Required evidence fields are present.                        |
| US1   | Full evidence, no tripwire, and Echo recovery are present | JSON report is emitted | `source_of_truth` is `echo`.                                 |
| US2   | Echo recovery is unavailable                              | JSON report is emitted | Report says recovery unavailable, not applied.               |
| US2   | Receipt exists but reading is missing                     | JSON report is emitted | Report shows incomplete reading evidence.                    |
| US2   | Legacy fallback is detected                               | JSON report is emitted | Report fails the gate and records `local_fallback_detected`. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario             | Fixture/Input                | Expected Result                               |
| -------------------- | ---------------------------- | --------------------------------------------- |
| Full evidence        | Applied posture plus reading | Complete report.                              |
| Pending evidence     | Acceptance only              | Pending report.                               |
| Unavailable recovery | Adapter failure              | Recovery unavailable report.                  |
| Legacy fallback      | Tripwire event               | Failed report, no Echo source-of-truth claim. |

Happy Path Testing:

1. Run report builder with full fixture.
2. Validate JSON schema.
3. Assert source-of-truth is Echo.

Negative/Edge Case Testing:

- Missing envelope digest fails schema validation.
- Missing contract identity is allowed only when explicitly fixture-scoped.
- Unknown status fails report validation.
- `source_of_truth: "echo"` with missing accepted evidence, missing reading
  authority, or fired tripwire fails validation.

Non-Functional Testing:

- Performance: report generation should not run multiple Echo recovery queries
  for the same submission.
- Security: report redacts raw text payload unless explicitly test fixture.
- Accessibility: JSON is stable and human-readable with indentation option.

## Slice 23: [jedit] Recovered Bounded Reading Path

### 1. Feature Overview & Objectives:

Problem statement: After recovery, jedit must obtain bounded text readings from
Echo evidence, not from legacy local memory.

Target user/audience: jedit editor runtime, tests, and external app proof
reviewers.

Success metrics:

- Recovered reading request uses Echo basis/reading identity.
- Legacy direct memory read is not used in release-gate path.
- Missing reading evidence maps to editor recovery incomplete.

### 2. Scope Definition:

In Scope:

- Add recovery-aware bounded reading request path.
- Wire through TextBufferSessionPort or successor.
- Add tests with fake Echo recovery/reading adapter.

Out of Scope:

- Full viewport UX.
- Streaming reads.
- Graft structural highlighting.

### 3. Detailed User Stories:

- US1: As jedit, I want recovered text to come from Echo reading evidence so
  that Echo is the authority after restart.
- US2: As a maintainer, I want legacy memory reads blocked in this path so that
  fallback cannot hide missing Echo evidence.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                             | When                       | Then                                                      |
| ----- | ------------------------------------------------- | -------------------------- | --------------------------------------------------------- |
| US1   | Echo reports applied posture with reading basis   | jedit requests text window | jedit uses Echo reading path and returns bounded reading. |
| US2   | Legacy memory API is invoked in release-gate mode | The test runs              | The gate fails.                                           |
| US2   | Echo reading is missing                           | jedit requests text window | jedit returns recovery incomplete, not local fallback.    |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                | Fixture/Input           | Expected Result       |
| ----------------------- | ----------------------- | --------------------- |
| Applied with reading    | Echo reading fixture    | Bounded text reading. |
| Applied without reading | Missing reading fixture | Recovery incomplete.  |
| Legacy fallback attempt | Tripwire enabled        | Test failure.         |

Happy Path Testing:

1. Configure fake Echo reading response.
2. Query recovered text window.
3. Assert reading identity and bounded payload.

Negative/Edge Case Testing:

- Wrong basis token fails.
- Over-budget window fails with budget posture.
- Missing Echo adapter fails closed.

Non-Functional Testing:

- Performance: bounded reading should request only needed range.
- Security: no direct filesystem or legacy model access in app path.
- Accessibility: errors identify missing basis/reading, not generic failure.

## Slice 24: [jedit] Happy-Path Recovery Gate Scenario

### 1. Feature Overview & Objectives:

Problem statement: jedit needs a single scenario proving edit submission,
Echo decision, bounded reading, restart, and recovery status in the happy path.

Target user/audience: CI, reviewers, and future app teams copying the pattern.

Success metrics:

- Scenario submits one non-trivial edit.
- Recovery posture is applied.
- Bounded reading digest matches expected text.

### 2. Scope Definition:

In Scope:

- Add jedit recovery gate happy-path script/spec.
- Use app-owned ports and Echo recovery adapter.
- Emit report artifact.

Out of Scope:

- Crash matrix.
- Legacy deletion.
- Full UI automation.

### 3. Detailed User Stories:

- US1: As a reviewer, I want one command showing jedit can recover an applied
  edit from Echo so that the proof is repeatable.
- US2: As jedit, I want the scenario to use product ports so that it reflects
  real architecture.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                  | When                | Then                                                  |
| ----- | -------------------------------------- | ------------------- | ----------------------------------------------------- |
| US1   | Echo and jedit fixtures are configured | Scenario runs       | Report shows applied posture after restart.           |
| US1   | Bounded reading is queried             | Report is emitted   | Reading digest matches expected edited text.          |
| US2   | Scenario source is scanned             | Imports are checked | It uses ports/adapters, not direct runtime internals. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario      | Fixture/Input           | Expected Result         |
| ------------- | ----------------------- | ----------------------- |
| Single edit   | Replace non-empty range | Applied after recovery. |
| Bounded read  | Window around edit      | Expected digest.        |
| Report schema | JSON report             | Valid schema.           |

Happy Path Testing:

1. Start configured Echo host fixture.
2. Submit edit through jedit port.
3. Drain trusted host.
4. Restart/recover.
5. Query posture and reading.

Negative/Edge Case Testing:

- Missing Echo config skips only when marked optional; release gate fails.
- Unexpected pending posture fails happy path.
- Reading mismatch fails with digest details.

Non-Functional Testing:

- Performance: scenario should complete within CI budget, target under 10 s.
- Security: app path cannot invoke trusted lifecycle directly.
- Accessibility: report path printed clearly for CI artifacts.

## Slice 25: [jedit] Retry After Local Amnesia Scenario

### 1. Feature Overview & Objectives:

Problem statement: The strongest proof is that jedit can forget local state,
retry safely, and accept Echo's recovered answer.

Target user/audience: jedit QA and Echo release reviewers.

Success metrics:

- Scenario wipes local adapter/session memory after submit.
- Retry returns Echo duplicate/final posture.
- No legacy local model is used to restore text truth.

### 2. Scope Definition:

In Scope:

- Add "memory amnesia" gate mode.
- Persist only submission id/envelope digest allowed for retry.
- Assert recovery from Echo.

Out of Scope:

- Full user session persistence.
- Window layout recovery.
- Cursor/selection recovery.

### 3. Detailed User Stories:

- US1: As a QA engineer, I want to wipe jedit local memory after submit so that
  Echo authority is proven.
- US2: As a user, I want retry after crash to avoid duplicate edits.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                  | When                 | Then                                                                |
| ----- | -------------------------------------- | -------------------- | ------------------------------------------------------------------- |
| US1   | Local session memory is wiped          | jedit restarts       | jedit queries Echo for posture instead of local model.              |
| US2   | Same submission id/envelope is retried | Echo responds        | jedit maps duplicate/final posture without resubmitting a new edit. |
| US2   | Same id/different envelope is retried  | jedit/Echo validates | The flow fails as protocol violation.                               |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario               | Fixture/Input             | Expected Result               |
| ---------------------- | ------------------------- | ----------------------------- |
| Amnesia after accepted | Only id/digest kept       | Pending or applied from Echo. |
| Amnesia after applied  | Only id/digest kept       | Applied from Echo.            |
| Bad retry              | Same id, changed envelope | Protocol violation.           |

Happy Path Testing:

1. Submit edit.
2. Store id/digest only.
3. Destroy jedit session objects.
4. Restart adapters.
5. Query Echo and map status.

Negative/Edge Case Testing:

- If local text cache is read, tripwire fails.
- If retry creates new id, test fails.
- If Echo unavailable, report recovery unavailable.

Non-Functional Testing:

- Performance: amnesia scenario should not require full UI boot.
- Security: persisted retry token must not include raw text unless policy allows.
- Accessibility: report explains which local state was intentionally discarded.

## Slice 26: \[Echo]\[jedit] Crash Runner Harness

### 1. Feature Overview & Objectives:

Problem statement: Crash-window truth requires a runner that can kill and
restart Echo/jedit at deterministic boundaries.

Target user/audience: cross-repo QA, CI maintainers, and release reviewers.

Success metrics:

- Runner accepts canonical crashpoint names.
- Runner records before/after artifacts.
- Runner supports read-only recovery validation.

### 2. Scope Definition:

In Scope:

- Add runner script or test harness contract.
- Wire first dry-run mode with fake crashpoint hooks.
- Emit structured result per crashpoint.

Out of Scope:

- Full OS-level process supervision for every platform.
- Browser automation.
- Long-running stress tests.

### 3. Detailed User Stories:

- US1: As QA, I want one crash runner interface so that every crash window uses
  the same evidence format.
- US2: As CI, I want dry-run capability so that fixture logic can be tested
  without killing real processes.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                          | When            | Then                                                                       |
| ----- | ------------------------------ | --------------- | -------------------------------------------------------------------------- |
| US1   | A crashpoint name is supplied  | Runner executes | It emits crashpoint, pre-state, recovery posture, and report path.         |
| US2   | Dry-run mode is selected       | Runner executes | It simulates boundary and validates expected posture without process kill. |
| US2   | Unknown crashpoint is supplied | Runner starts   | It fails before running scenario.                                          |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                 | Fixture/Input | Expected Result              |
| ------------------------ | ------------- | ---------------------------- |
| Known crashpoint dry-run | Manifest name | Structured pass/fail result. |
| Unknown crashpoint       | Bad name      | Typed config error.          |
| Report emission          | Output dir    | JSON report file exists.     |

Happy Path Testing:

1. Run dry-run for all five crashpoints.
2. Assert reports generated.
3. Validate report schema.

Negative/Edge Case Testing:

- Missing Echo path returns config error.
- Output path unwritable returns IO error.
- Runner timeout produces timeout posture.

Non-Functional Testing:

- Performance: dry-run matrix completes under CI unit-test budget.
- Security: runner must not execute shell-interpolated user input.
- Accessibility: logs show crashpoint name and exact failing phase.

## Slice 27: \[Echo]\[jedit] Before-Accept-Response Crash Window

### 1. Feature Overview & Objectives:

Problem statement: If a crash occurs before the accepted response, Echo may or
may not have committed acceptance. The system must classify the outcome
deterministically.

Target user/audience: jedit recovery gate and Echo WAL reviewers.

Success metrics:

- Crash before commit yields not-found.
- Crash after commit before response yields duplicate-compatible posture.
- jedit maps both outcomes correctly.

### 2. Scope Definition:

In Scope:

- Implement crashpoint around acceptance response.
- Add jedit mapping assertions.
- Record transaction count and posture.

Out of Scope:

- Scheduler tick crash windows.
- User notification copy.
- Network transport simulation beyond response loss.

### 3. Detailed User Stories:

- US1: As jedit, I want to know whether the edit was accepted when the process
  died before response.
- US2: As Echo, I want acceptance truth to be based on WAL commit, not response
  delivery.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                         | When           | Then                                                |
| ----- | --------------------------------------------- | -------------- | --------------------------------------------------- |
| US1   | Crash occurs before WAL commit                | jedit recovers | Status maps from `not_found`.                       |
| US1   | Crash occurs after WAL commit before response | jedit retries  | Status maps from duplicate pending/decided posture. |
| US2   | Response delivery is lost                     | Echo recovers  | WAL commit decides acceptance truth.                |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                     | Fixture/Input             | Expected Result               |
| ---------------------------- | ------------------------- | ----------------------------- |
| Before commit                | Kill before commit marker | `not_found`.                  |
| After commit before response | Kill after commit fsync   | Duplicate-compatible posture. |
| Retry conflict               | Same id, changed envelope | Protocol violation.           |

Happy Path Testing:

1. Run crash runner at before-accept-response.
2. Restart.
3. Query posture.
4. Assert jedit mapped status.

Negative/Edge Case Testing:

- Lost response must not create second accepted edit.
- Missing retry token yields jedit recovery unknown, not local fallback.
- WAL root corruption yields recovery error.

Non-Functional Testing:

- Performance: crash window test should stay under integration timeout.
- Security: no app access to WAL append to force acceptance.
- Accessibility: report describes whether commit was observed.

## Slice 28: \[Echo]\[jedit] After-Accept-Before-Tick Crash Window

### 1. Feature Overview & Objectives:

Problem statement: Accepted-but-not-ticked work is valid history. After crash,
jedit must see pending/deciding status from Echo, not assume failure or apply.

Target user/audience: jedit users, adapters, and Echo scheduler reviewers.

Success metrics:

- Crash after acceptance before scheduler decision recovers pending.
- jedit maps pending without reading legacy text as applied.
- Later trusted host drain can decide pending work.

### 2. Scope Definition:

In Scope:

- Add crashpoint after accepted response and before tick.
- Assert recovered accepted-pending posture.
- Test optional later drain from pending state.

Out of Scope:

- Multi-user conflict policy.
- UI progress indicator.
- Background auto-retry queue.

### 3. Detailed User Stories:

- US1: As a user, I want accepted pending edits to survive restart so that
  acknowledged work is not lost.
- US2: As jedit, I want pending posture to avoid showing applied text without a
  scheduler receipt.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                         | When                         | Then                                                  |
| ----- | --------------------------------------------- | ---------------------------- | ----------------------------------------------------- |
| US1   | Edit is accepted and process dies before tick | Echo recovers                | Posture is `accepted_pending` or `accepted_deciding`. |
| US2   | jedit maps pending posture                    | UI/report status is produced | Status is pending, not applied.                       |
| US1   | Trusted host drains after recovery            | Scheduler decides            | Receipt is committed and posture becomes final.       |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                     | Fixture/Input             | Expected Result                 |
| ---------------------------- | ------------------------- | ------------------------------- |
| Accepted pending             | Kill before tick          | Pending posture.                |
| Pending then drain           | Restart and host drain    | Applied/rejected final posture. |
| Pending with missing payload | Missing envelope material | Incomplete evidence.            |

Happy Path Testing:

1. Submit edit.
2. Stop at after-accept-before-tick crashpoint.
3. Recover.
4. Query jedit status.

Negative/Edge Case Testing:

- jedit must not materialize edited text from local memory while pending.
- App must not command tick after restart.
- Missing accepted evidence returns incomplete, not applied.

Non-Functional Testing:

- Performance: pending recovery should be fast from indexes.
- Security: pending posture must not expose scheduler control.
- Accessibility: pending report should explain no receipt exists yet.

## Slice 29: \[Echo]\[jedit] After-Receipt-Before-Reading Crash Window

### 1. Feature Overview & Objectives:

Problem statement: A scheduler decision may be durable before a bounded reading
is materialized. jedit must recover applied/rejected decision while handling
missing or rederivable reading evidence honestly.

Target user/audience: jedit recovery gate and Echo observation maintainers.

Success metrics:

- Receipt recovers as final decision.
- Reading absence is classified separately.
- jedit does not use local memory to fill missing reading.

### 2. Scope Definition:

In Scope:

- Add crashpoint after receipt commit before reading evidence.
- Assert receipt recovery and reading incomplete/rederived posture.
- Add jedit report mapping.

Out of Scope:

- Full retained reading rederivation engine if not already available.
- UI rendering.
- Export.

### 3. Detailed User Stories:

- US1: As jedit, I want to know the edit applied even if reading evidence is not
  yet available.
- US2: As Echo, I want missing reading evidence to be explicit so that final
  decision and observation are not conflated.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                          | When             | Then                                                         |
| ----- | ------------------------------ | ---------------- | ------------------------------------------------------------ |
| US1   | Receipt committed before crash | Recovery runs    | Submission posture is applied/rejected according to receipt. |
| US2   | Reading evidence is absent     | Chain is queried | Chain reports missing or rederivable reading evidence.       |
| US1   | jedit requests text            | Reading missing  | jedit returns recovery incomplete, not local fallback.       |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                    | Fixture/Input                | Expected Result                        |
| --------------------------- | ---------------------------- | -------------------------------------- |
| Applied receipt no reading  | Crash after receipt          | Applied with reading incomplete.       |
| Rejected receipt no reading | Rejection receipt            | Rejected; no reading required.         |
| Rederivable reading         | Observer can rerun read-only | Reading rederived or marked available. |

Happy Path Testing:

1. Submit edit and commit receipt.
2. Crash before reading materialization.
3. Recover and query posture.
4. Query chain.

Negative/Edge Case Testing:

- Receipt corruption returns recovery error.
- Reading rederivation must not tick or mutate.
- jedit local fallback tripwire must remain enabled.

Non-Functional Testing:

- Performance: receipt recovery should not require reading payload.
- Security: rederivation respects observer rights/available local scope.
- Accessibility: report separates decision and reading sections.

## Slice 30: \[Echo]\[jedit] After-Reading-Before-Observe Crash Window

### 1. Feature Overview & Objectives:

Problem statement: Echo may have committed reading evidence before jedit sees
it. After restart, jedit should recover the reading from Echo evidence.

Target user/audience: jedit recovery gate and Echo QueryView maintainers.

Success metrics:

- Reading evidence survives restart.
- jedit maps recovered reading to editor-facing result.
- No local memory fallback occurs.

### 2. Scope Definition:

In Scope:

- Add crashpoint after reading evidence commit before jedit observation.
- Query recovered reading after restart.
- Record reading digest in report.

Out of Scope:

- Streaming observation.
- UI update scheduling.
- Full WSC bundling.

### 3. Detailed User Stories:

- US1: As jedit, I want observed text to be recoverable even if process died
  before app handled the observation.
- US2: As Echo, I want committed reading evidence to be enough for post-restart
  observation.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                         | When             | Then                                           |
| ----- | --------------------------------------------- | ---------------- | ---------------------------------------------- |
| US1   | Reading evidence committed before app observe | jedit restarts   | jedit obtains bounded reading from Echo.       |
| US2   | Reading evidence exists                       | Chain is queried | Chain is complete with reading id and digest.  |
| US1   | Local cache is empty                          | jedit observes   | Result still comes from Echo reading evidence. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                | Fixture/Input            | Expected Result            |
| ----------------------- | ------------------------ | -------------------------- |
| Reading committed       | Crash before app observe | Reading recovered.         |
| Reading payload missing | Envelope only            | Missing-retention posture. |
| Wrong vars digest       | Query mismatch           | No stale reading returned. |

Happy Path Testing:

1. Commit acceptance, receipt, and reading.
2. Crash before app observes.
3. Restart jedit.
4. Query reading through Echo port.

Negative/Edge Case Testing:

- Reading from wrong basis fails.
- Payload digest mismatch obstructs.
- Local memory tripwire catches fallback.

Non-Functional Testing:

- Performance: reading retrieval should use retained index.
- Security: reading respects redaction posture.
- Accessibility: report includes digest for comparison.

## Slice 31: \[Echo]\[jedit] After-Observe-Before-Local-Status Crash Window

### 1. Feature Overview & Objectives:

Problem statement: jedit may observe success but crash before recording local
status. On restart, Echo must remain the source of truth and jedit must
reconcile from Echo.

Target user/audience: jedit app maintainers and recovery reviewers.

Success metrics:

- Local status loss does not change recovered result.
- jedit maps Echo final posture after restart.
- No duplicate edit is submitted during reconciliation.

### 2. Scope Definition:

In Scope:

- Add crashpoint after app observe before local status write.
- Wipe local status and recover from Echo.
- Assert no resubmission unless explicit retry token is used safely.

Out of Scope:

- Full workspace session persistence.
- UI notification persistence.
- Undo history persistence.

### 3. Detailed User Stories:

- US1: As jedit, I want local status loss to reconcile from Echo so that the app
  does not invent truth.
- US2: As Echo, I want duplicate retries after observed success to return final
  posture, not reapply.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                                           | When                           | Then                                       |
| ----- | --------------------------------------------------------------- | ------------------------------ | ------------------------------------------ |
| US1   | jedit observed applied reading then crashed before local status | jedit restarts                 | It queries Echo and maps applied status.   |
| US2   | jedit retries same id after restart                             | Echo handles retry             | Echo returns already-applied posture.      |
| US1   | Local memory is empty                                           | jedit renders recovered status | It uses Echo reading/materialization path. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario             | Fixture/Input           | Expected Result                  |
| -------------------- | ----------------------- | -------------------------------- |
| Lost local status    | Applied evidence exists | Applied recovered.               |
| Retry after observed | Same id/envelope        | Already-applied posture.         |
| Missing reading      | Applied receipt only    | Applied plus incomplete reading. |

Happy Path Testing:

1. Run edit through observation.
2. Crash before status persistence.
3. Restart with local status wiped.
4. Query Echo posture and reading.

Negative/Edge Case Testing:

- Reconstructing from local memory fails tripwire.
- Retry with changed envelope fails.
- Missing Echo evidence yields recovery error, not applied.

Non-Functional Testing:

- Performance: reconciliation should not require full file scan.
- Security: local status cannot override Echo final posture.
- Accessibility: report should state local status was discarded.

## Slice 32: [jedit] Production Legacy Memory Static Guard

### 1. Feature Overview & Objectives:

Problem statement: jedit cannot be Echo-authoritative while production paths can
import direct legacy text mutation/read modules. A static guard must enforce the
port boundary.

Target user/audience: jedit maintainers and review agents.

Success metrics:

- Guard scans production source.
- Guard allows test/dev fixture imports by explicit allowlist.
- Sample forbidden import test fails.

### 2. Scope Definition:

In Scope:

- Add static guard script/test.
- Define allowlist for fixtures/adapters.
- Wire guard into jedit check or release-gate command.

Out of Scope:

- Deleting all legacy code.
- Blocking adapter-private caches.
- UI code refactor beyond imports.

### 3. Detailed User Stories:

- US1: As a reviewer, I want direct legacy memory imports to fail so that the
  Echo path cannot be bypassed silently.
- US2: As a test author, I want explicit fixture allowlists so that transition
  tests remain possible.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                              | When           | Then                                       |
| ----- | -------------------------------------------------- | -------------- | ------------------------------------------ |
| US1   | Production source imports forbidden legacy module  | Guard runs     | Guard fails with file path and import.     |
| US2   | Test fixture imports legacy module under allowlist | Guard runs     | Guard passes and records allowlist reason. |
| US1   | Release gate runs                                  | Guard executes | No production bypass is present.           |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                 | Fixture/Input             | Expected Result       |
| ------------------------ | ------------------------- | --------------------- |
| Forbidden import fixture | Synthetic production file | Guard fails.          |
| Allowed fixture import   | Test path                 | Guard passes.         |
| Real source scan         | `src/` production paths   | No forbidden imports. |

Happy Path Testing:

1. Run guard over current source.
2. Assert pass.
3. Run sample forbidden fixture.
4. Assert failure.

Negative/Edge Case Testing:

- Dynamic import of forbidden module fails.
- Re-export through barrel file fails.
- Similar but allowed names do not false-positive.

Non-Functional Testing:

- Performance: guard completes under one second.
- Security: guard cannot be disabled by environment in release-gate mode.
- Accessibility: guard output is plain text and CI-friendly.

## Slice 33: [jedit] Release-Gate Runtime Tripwire Mode

### 1. Feature Overview & Objectives:

Problem statement: Static guards miss runtime fallback. Release-gate mode must
fail loudly if production flow reads or mutates legacy memory instead of Echo.

Target user/audience: jedit QA and release reviewers.

Success metrics:

- `ECHO_POWERED_JEDIT_RELEASE_GATE=1` or equivalent tripwire mode exists.
- Direct legacy read/mutation in gated path throws typed error.
- Gate proves no tripwire fired.

### 2. Scope Definition:

In Scope:

- Add runtime tripwire hooks around legacy memory access points.
- Enable tripwire only in release-gate/test mode.
- Add tests proving both pass and fail behavior.

Out of Scope:

- Removing legacy modules.
- Production user-facing panic behavior.
- UI instrumentation.

### 3. Detailed User Stories:

- US1: As QA, I want runtime fallback to fail so that hidden local-memory reads
  cannot pass the gate.
- US2: As jedit developers, I want tripwires gated by environment so that
  existing tests can migrate intentionally.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                         | When                      | Then                                  |
| ----- | ----------------------------- | ------------------------- | ------------------------------------- |
| US1   | Release-gate mode is enabled  | Legacy direct read occurs | jedit throws typed tripwire error.    |
| US1   | Echo path runs cleanly        | Gate completes            | Report says no legacy tripwire fired. |
| US2   | Test fixture mode is explicit | Legacy fixture runs       | It can run outside release-gate mode. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario           | Fixture/Input     | Expected Result     |
| ------------------ | ----------------- | ------------------- |
| Clean Echo path    | Tripwire enabled  | No tripwire events. |
| Forbidden read     | Tripwire enabled  | Typed failure.      |
| Fixture local path | Tripwire disabled | Allowed.            |

Happy Path Testing:

1. Enable release-gate mode.
2. Run happy-path recovery scenario.
3. Assert zero tripwire events.

Negative/Edge Case Testing:

- Direct text mutation throws.
- Export from local buffer throws.
- Legacy save path throws.

Non-Functional Testing:

- Performance: tripwire overhead acceptable in tests; disabled in normal dev
  mode unless configured.
- Security: app cannot disable tripwire during release gate.
- Accessibility: errors identify forbidden operation and call site where possible.

## Slice 34: [jedit] Materialize Artifact From Recovered Causal Basis

### 1. Feature Overview & Objectives:

Problem statement: The proof should show jedit can materialize a text artifact
from Echo recovered causal basis, not from local memory.

Target user/audience: jedit users, export adapters, and release reviewers.

Success metrics:

- Materialization request names recovered basis/reading identity.
- Output artifact digest matches Echo reading digest.
- Local memory source is blocked in release-gate mode.

### 2. Scope Definition:

In Scope:

- Add materialization adapter path from recovered reading.
- Write artifact to temp output in test.
- Record artifact digest in report.

Out of Scope:

- Full "save file" UX.
- Git export.
- WSC portable archive.

### 3. Detailed User Stories:

- US1: As a user, I want jedit to materialize text from recovered Echo truth so
  that crash recovery can produce artifacts.
- US2: As Echo, I want materialization to be an app-owned read/export action,
  not Echo state mutation.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                          | When                        | Then                                                  |
| ----- | ------------------------------ | --------------------------- | ----------------------------------------------------- |
| US1   | Echo recovered applied reading | jedit materializes artifact | File bytes match recovered reading digest.            |
| US2   | Materialization runs           | Echo state is inspected     | No Echo mutation occurs from export.                  |
| US1   | Local memory is unavailable    | Materialization runs        | Output still comes from Echo reading or fails closed. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario        | Fixture/Input | Expected Result                        |
| --------------- | ------------- | -------------------------------------- |
| Applied reading | Output path   | Artifact written with expected digest. |
| Missing reading | Output path   | Materialization blocked.               |
| Read-only Echo  | Export action | No Echo mutation.                      |

Happy Path Testing:

1. Recover applied edit.
2. Query bounded/full-enough reading for materialization.
3. Write temp artifact.
4. Verify digest.

Negative/Edge Case Testing:

- Missing output directory returns typed adapter error.
- Digest mismatch fails.
- Local memory export tripwire catches fallback.

Non-Functional Testing:

- Performance: materialization writes streaming where possible.
- Security: output path must be adapter-controlled and avoid path traversal.
- Accessibility: report states artifact path and digest.

## Slice 35: \[Echo]\[jedit] Side-Effect Authorization Boundary

### 1. Feature Overview & Objectives:

Problem statement: Materialization is an external side effect. It must happen
only after committed Echo authorization and should be idempotent.

Target user/audience: Echo/jedit integration engineers and storage reviewers.

Success metrics:

- Materialization requires committed receipt/reading evidence.
- Side-effect intent or adapter authorization is recorded.
- Existing artifact replay checks digest before rewrite.

### 2. Scope Definition:

In Scope:

- Define first jedit materialization authorization rule.
- Add idempotent temp-write/rename/digest verification.
- Report materialization observed/not observed posture.

Out of Scope:

- General side-effect outbox implementation for all apps.
- Distributed filesystem semantics.
- User save workflow.

### 3. Detailed User Stories:

- US1: As jedit, I want export side effects fenced by Echo evidence so that files
  cannot change ahead of history.
- US2: As recovery, I want existing artifact detection so that retry does not
  rewrite unnecessarily.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                        | When                         | Then                                                                                  |
| ----- | -------------------------------------------- | ---------------------------- | ------------------------------------------------------------------------------------- |
| US1   | No committed receipt/reading evidence exists | Materialization is requested | jedit refuses to write artifact.                                                      |
| US1   | Evidence exists                              | Materialization runs         | Artifact is written via temp file, fsync/rename where supported, and digest verified. |
| US2   | Artifact already matches expected digest     | Retry runs                   | Adapter records observed/matched posture without rewriting.                           |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                     | Fixture/Input             | Expected Result                          |
| ---------------------------- | ------------------------- | ---------------------------------------- |
| Authorized write             | Evidence plus output path | Verified artifact.                       |
| Unauthorized write           | No evidence               | Refusal.                                 |
| Existing matching artifact   | Prewritten file           | Observed/matched posture.                |
| Existing mismatched artifact | Wrong file                | Obstruction or overwrite policy refusal. |

Happy Path Testing:

1. Recover applied reading.
2. Request materialization.
3. Verify temp-write/rename path.
4. Verify digest and report posture.

Negative/Edge Case Testing:

- Output path traversal rejected.
- Partial write cleaned or obstructed.
- Echo unavailable blocks materialization.

Non-Functional Testing:

- Performance: digest verification scales linearly with artifact size.
- Security: no untrusted path writes outside configured root.
- Accessibility: materialization errors cite authorization/evidence issue.

## Slice 36: [Echo] No-Editor-Nouns Trusted-Runtime Guard

### 1. Feature Overview & Objectives:

Problem statement: Echo must stay generic while supporting jedit. A guard should
fail if trusted runtime code starts importing editor concepts.

Target user/audience: Echo maintainers and review agents.

Success metrics:

- Guard scans production Echo crate source for forbidden app/editor nouns.
- Allowlist covers docs/tests/fixtures only.
- Guard is linked from BEARING and release gate.

### 2. Scope Definition:

In Scope:

- Expand or tune `scripts/check-no-app-nouns-in-core.sh`.
- Add forbidden terms relevant to jedit boundaries.
- Add fixture proving guard catches violations.

Out of Scope:

- Removing legitimate generic terms that collide with prose.
- Scanning external app repos.
- Blocking generated app fixtures in tests.

### 3. Detailed User Stories:

- US1: As Echo, I want a mechanical guard so that runtime code does not learn
  jedit nouns.
- US2: As a reviewer, I want failures to show exact file/term so that cleanup is
  fast.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                                 | When              | Then                                                               |
| ----- | ----------------------------------------------------- | ----------------- | ------------------------------------------------------------------ |
| US1   | Production core source contains forbidden editor noun | Guard runs        | Guard fails with file and term.                                    |
| US1   | Test fixture contains app-shaped noun                 | Guard runs        | Guard passes if path is explicitly allowed.                        |
| US2   | Guard fails                                           | Output is printed | It names remediation boundary: move noun to app/generated adapter. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario             | Fixture/Input              | Expected Result |
| -------------------- | -------------------------- | --------------- |
| Production violation | Synthetic source line      | Guard fails.    |
| Fixture allowance    | Test fixture path          | Guard passes.   |
| Current source       | `crates/` production paths | Guard passes.   |

Happy Path Testing:

1. Run guard over current production source.
2. Assert pass.
3. Run negative fixture.
4. Assert failure.

Negative/Edge Case Testing:

- Case variants are caught.
- Comment-only violations are treated according to configured policy.
- False positives can be documented in allowlist with reason.

Non-Functional Testing:

- Performance: guard completes quickly enough for pre-push.
- Security: guard cannot be bypassed by generated production file placement.
- Accessibility: output uses plain text and no color-only meaning.

## Slice 37: \[Echo]\[jedit] Malicious Adapter Negative Suite

### 1. Feature Overview & Objectives:

Problem statement: The boundary must resist cheating, not just support the happy
path. A malicious adapter suite should attempt forbidden operations.

Target user/audience: security reviewers, Echo maintainers, and app adapter
authors.

Success metrics:

- Attempts to tick, append WAL, recover runtime, install package, forge receipt,
  invent reading, and spoof contract identity fail.
- Failures are typed and do not mutate state.
- Suite runs in CI or release gate.

### 2. Scope Definition:

In Scope:

- Add negative fixtures in Echo and/or jedit.
- Cover app-facing and trusted-host boundary separation.
- Report authority violation outcomes.

Out of Scope:

- Penetration testing external OS surfaces.
- Browser sandbox hardening.
- Network auth.

### 3. Detailed User Stories:

- US1: As Echo, I want malicious app adapters to fail when they request trusted
  authority.
- US2: As jedit, I want boundary tests proving the editor cannot accidentally
  depend on privileged control.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                                | When                | Then                                                        |
| ----- | ------------------------------------ | ------------------- | ----------------------------------------------------------- |
| US1   | App-facing adapter attempts to tick  | API is called       | Operation is impossible by type or returns authority error. |
| US1   | Adapter tries to forge receipt       | Report is validated | Forged evidence is rejected.                                |
| US2   | jedit app path tries package install | Test runs           | Authority violation is reported and no install occurs.      |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario           | Fixture/Input        | Expected Result                     |
| ------------------ | -------------------- | ----------------------------------- |
| App tick attempt   | App-safe client      | Compile failure or authority error. |
| WAL append attempt | App-safe client      | No method/capability.               |
| Receipt forge      | Fake receipt id      | Validation error.                   |
| Contract spoof     | Wrong package digest | Install/read failure.               |

Happy Path Testing:

1. Run malicious adapter suite.
2. Assert every forbidden operation fails.
3. Assert runtime state unchanged.

Negative/Edge Case Testing:

- Partial forged evidence cannot be accepted.
- Same submission id different payload fails.
- Trusted-host fixture still works when using host API.

Non-Functional Testing:

- Performance: suite should be targeted enough for PR gate.
- Security: no forbidden operation should require unsafe code to test.
- Accessibility: each failure message names violated authority boundary.

## Slice 38: \[Echo]\[jedit] JSON Causal Durability Report

### 1. Feature Overview & Objectives:

Problem statement: The release gate needs one machine-readable artifact proving
the full chain: edit, acceptance, receipt, reading, restart, recovery, and
jedit mapping.

Target user/audience: CI, release reviewers, and future app teams.

Success metrics:

- JSON report includes every required section.
- Schema validation passes.
- Report says `source_of_truth: "echo"` only when all gate conditions pass.
- Report includes schema version, producer identity, producer version,
  compatibility metadata, durability mode, and reading source/authority.

### 2. Scope Definition:

In Scope:

- Define report schema.
- Aggregate Echo and jedit evidence into one artifact.
- Add pass/fail summary.
- Validate source-of-truth conclusion from evidence fields rather than trusting
  a precomputed string.

Out of Scope:

- Human receipt card.
- Web dashboard.
- Full release notes.

### 3. Detailed User Stories:

- US1: As CI, I want a JSON report so that gate status is machine-verifiable.
- US2: As a reviewer, I want the report to show why the proof passed or failed.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                              | When              | Then                                                                                                                                 |
| ----- | ---------------------------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| US1   | Full recovery gate passes          | Report is emitted | `source_of_truth` is `echo` and all required evidence sections exist.                                                                |
| US1   | Report claims Echo source of truth | Validator runs    | Validator recomputes the claim from accepted evidence, scheduler ownership, reading authority, tripwire status, and recovery source. |
| US2   | Reading evidence is missing        | Report is emitted | Gate fails or reports incomplete according to scenario expectation.                                                                  |
| US2   | Legacy tripwire fires              | Report is emitted | Gate fails and names the forbidden legacy access.                                                                                    |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario        | Fixture/Input                           | Expected Result       |
| --------------- | --------------------------------------- | --------------------- |
| Full pass       | Happy path                              | Valid pass report.    |
| Pending crash   | After accept before tick                | Valid pending report. |
| Missing reading | Receipt only                            | Incomplete report.    |
| Legacy bypass   | Tripwire event                          | Failed report.        |
| Overclaim       | Missing evidence plus Echo source claim | Schema/gate failure.  |

Happy Path Testing:

1. Run end-to-end gate.
2. Validate report schema.
3. Assert evidence chain and digest fields.

Negative/Edge Case Testing:

- Missing required section fails schema.
- Contradictory posture fields fail validation.
- Source-of-truth cannot be Echo if local fallback occurred.
- Unknown schema version fails closed unless compatibility metadata permits it.

Non-Functional Testing:

- Performance: report aggregation should not rerun scenarios.
- Security: report redacts payload unless fixture marks it public.
- Accessibility: JSON field names should be stable and documented.

## Slice 39: \[Echo]\[jedit] Human Causal Receipt Card

### 1. Feature Overview & Objectives:

Problem statement: Humans need a compact proof summary without reading raw JSON.
The receipt card should summarize the same evidence without becoming authority.

Target user/audience: reviewers, release notes authors, and demo operators.

Success metrics:

- Card is generated from JSON report only.
- Card includes submission, decision, reading, recovery, and legacy-bypass status.
- Card cannot pass if JSON report fails.

### 2. Scope Definition:

In Scope:

- Add human-readable Markdown/text receipt card generator.
- Source it from validated JSON report.
- Add snapshot tests.

Out of Scope:

- Replacing JSON report.
- Web UI.
- User-facing application copy.

### 3. Detailed User Stories:

- US1: As a reviewer, I want a concise receipt card so that I can see the proof
  result quickly.
- US2: As CI, I want the card generated from JSON so that humans and machines
  do not diverge.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                        | When                 | Then                                                                          |
| ----- | ---------------------------- | -------------------- | ----------------------------------------------------------------------------- |
| US1   | JSON report passes           | Card is generated    | Card shows PASS, submission, receipt, reading, recovery, and source-of-truth. |
| US2   | JSON report fails validation | Card generation runs | Card generation fails or marks report invalid.                                |
| US1   | Legacy tripwire fired        | Card is generated    | Card shows failure and legacy-bypass reason.                                  |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario     | Fixture/Input    | Expected Result        |
| ------------ | ---------------- | ---------------------- |
| Passing JSON | Full report      | PASS card snapshot.    |
| Pending JSON | Pending scenario | PENDING/expected card. |
| Invalid JSON | Missing fields   | Generation failure.    |
| Failed gate  | Tripwire report  | FAIL card.             |

Happy Path Testing:

1. Generate JSON report.
2. Generate card.
3. Snapshot card.

Negative/Edge Case Testing:

- Card must not recompute status independently.
- Unknown posture renders as invalid.
- Missing digest renders as failed validation.

Non-Functional Testing:

- Performance: card generation should be near-instant.
- Security: card redacts payload consistently with JSON.
- Accessibility: card is plain Markdown/text with no color-only signal.

## Slice 40: \[Echo]\[jedit] Aggregate Gate, Docs, And Drift Audit

### 1. Feature Overview & Objectives:

Problem statement: The forty-slice effort must end in an aggregate gate and
updated docs, not scattered tests. Reviewers need one command and one doctrine
check.

Target user/audience: Echo/jedit maintainers, release reviewers, and future app
teams.

Success metrics:

- Aggregate gate runs Echo checks, jedit checks, crash windows, tripwires, JSON
  report, and receipt card.
- BEARING and jedit docs reflect current truth.
- Drift audit confirms Echo stays generic and jedit owns editor semantics.

### 2. Scope Definition:

In Scope:

- Add aggregate gate command or documented procedure.
- Update BEARING, jedit end-to-end docs, WorkItems, and release-gate docs.
- Run app-noun and legacy-bypass guards.

Out of Scope:

- Full v0.1.0 release.
- Continuum transport.
- Complete editor product polish.

### 3. Detailed User Stories:

- US1: As a maintainer, I want one aggregate gate so that I know whether jedit is
  genuinely Echo-authoritative.
- US2: As a future app team, I want docs to show the pattern so that Graft,
  Think, and other apps do not copy transitional hacks.

### 4. Acceptance Criteria (BDD Format):

| Story | Given                           | When                     | Then                                                                |
| ----- | ------------------------------- | ------------------------ | ------------------------------------------------------------------- |
| US1   | All forty slices are complete   | Aggregate gate runs      | It passes and emits JSON report plus receipt card.                  |
| US1   | Any required crash window fails | Aggregate gate runs      | It fails and names the failing slice/window.                        |
| US2   | Docs are read after completion  | The reader follows links | Echo/jedit authority boundary and remaining non-goals are explicit. |

### 5. Detailed Test Plan:

Test Scenarios:

| Scenario                | Fixture/Input                  | Expected Result |
| ----------------------- | ------------------------------ | --------------- |
| Full aggregate pass     | Clean Echo and jedit checkouts | Gate passes.    |
| Echo app noun violation | Synthetic violation            | Gate fails.     |
| jedit legacy bypass     | Synthetic violation            | Gate fails.     |
| Missing crash report    | Remove artifact                | Gate fails.     |

Happy Path Testing:

1. Run Echo unit/test-slice gates.
2. Run jedit check and recovery gate.
3. Run crash-window matrix.
4. Generate JSON report and receipt card.
5. Run docs/dead-ref validation.

Negative/Edge Case Testing:

- Pending Echo posture must be allowed only in crash windows that expect pending.
- Source-of-truth must not be Echo if jedit used legacy fallback.
- Docs must not claim full WSC, Continuum, or complete editor release.

Non-Functional Testing:

- Performance: aggregate gate can be slower than PR smoke, but must be bounded
  and documented for release use.
- Security: aggregate gate verifies app-safe and host-only boundaries.
- Accessibility: summary output is plain text with artifact paths.

## Completion Bar

The forty-slice plan is complete when this sentence is true and executable:

```text
A production jedit edit submitted through the app-owned port is durably
accepted by Echo, safely retryable without duplicate application, decided only
by Echo's trusted scheduler, observed only through Echo bounded reading
evidence, recoverable after restart from Echo WAL history, mapped by jedit into
editor-facing status without Echo learning editor nouns, and materializable
from the recovered causal basis without reading legacy memory.
```
