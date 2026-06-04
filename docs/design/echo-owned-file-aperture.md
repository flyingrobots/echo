<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo-Owned File Aperture

Status: active design note.  
Scope: shared Echo standard artifact for host-file observation,
reconciliation, projection, mutation, and materialization.

## Decision Summary

The file aperture belongs in Echo as a standard Echo-owned contract/runtime
artifact. Wesley may describe and generate the contract shape, but Wesley does
not own file semantics. Jedit and WARP DRIVE are client membranes over the same
Echo file aperture. They may provide host capabilities and user interfaces, but
they must not maintain causal file history.

This artifact is not an Echo kernel primitive. Echo core remains substrate
generic and app-noun-clean. The file aperture is a standard optic hosted by
Echo: it uses Echo admission, WAL/WSC retention, receipts, reading envelopes,
witnesses, and materialization posture to make host-file interactions lawful.

## Sponsored Humans

A Jedit user wants to open any file from disk, see exactly the bytes that are
there, edit normally, and save normally, without knowing that Echo is observing,
admitting, diffing, retaining, and materializing causal file history.

A WARP DRIVE user wants ordinary POSIX tools such as `cat`, `vim`, `rg`, and
build tools to see files, while Echo remains the authority behind the projected
bytes and every basis-aware write.

## Sponsored Agents

An agent needs a machine-readable file aperture contract so it can inspect why
a file has particular bytes, replay or reconstruct a retained coordinate, and
distinguish admitted host observations from materializations without scraping
Jedit UI state or WARP DRIVE FUSE internals.

## Hill

By the end of the file aperture cycle, Jedit and WARP DRIVE can both open a
host file through Echo, receive a witnessed file projection, submit changes
against an explicit basis, and inspect receipts proving whether Echo admitted,
obstructed, or materialized the file state.

## Current Truth

Echo already has the generic runtime ingredients:

- witnessed submission ingress;
- scheduler-owned tick receipts;
- receipt correlation;
- retained material references;
- `ReadingEnvelope`-backed QueryView observations;
- WAL and WSC storage boundaries for recoverable causal evidence.

Jedit currently has product pressure for opening arbitrary host files and
rendering text in an editor. It must not become a second causal ledger.

WARP DRIVE already proves a read-path slice: Echo can produce normal file-like
bytes through an observation/projection payload, and WARP DRIVE can expose
those bytes through POSIX reads. Its current FUSE scaffold is not the shared
domain contract.

## Problem

Opening a host file is usually called a read. Saving a host file is usually
called a write. Those words are wrong at the Echo boundary.

Inside Echo, both actions can advance causal history:

- Opening a host file may discover bytes Echo has never admitted.
- Opening a known host file may discover that another program changed it.
- Saving a file must admit the desired content against a basis before any host
  materialization can be called successful.
- A host write must be followed by observation or verification before Echo can
  claim the external material matches the causal projection.

Therefore both user-level reads and user-level writes contain causal write
phases. The Echo vocabulary must name the actual boundary acts.

## Boundary Vocabulary

Use these names inside Echo designs and APIs:

- **Host observation:** external host material enters Echo causal history.
- **Host snapshot:** the observed bytes, digest, stat posture, path evidence,
  platform identity, and capability witness for host material.
- **File site:** Echo's durable identity for a file-like host artifact.
- **File projection:** a bounded Echo reading of file content or directory
  membership at a causal coordinate.
- **Content intent:** Echo's canonical mutation shape after diffing an observed
  or proposed byte sequence against a basis.
- **Host materialization:** Echo-authorized attempt to write an admitted
  projection back to a host path.
- **Materialization verification:** follow-up host observation proving whether
  the external bytes match the admitted Echo projection.

Avoid treating POSIX `read` and `write` as Echo ontology. They are client
membrane operations.

## Core Invariants

- Echo owns file causality inside the file aperture.
- The application defines vocabulary and host affordances; it does not manage
  causal history.
- A host file path is evidence, not authority.
- A host file read may be an Echo admission.
- A host file save is admission plus materialization plus verification.
- Echo owns accepting a host snapshot, diffing it against causal basis, and
  decomposing the result into canonical content intents.
- A cached file projection is an acceleration over Echo truth, not authority.
- If retained file material is missing, redacted, encrypted-unavailable,
  corrupt, or obstructed, the information is unavailable through Echo. Clients
  must not reconstruct it from private app logs.
- Jedit and WARP DRIVE consume the same Echo file aperture. Neither project
  owns the shared file semantics.
- Echo core stays app-noun-clean. File aperture vocabulary may live in an
  Echo-owned standard artifact, not in the substrate kernel as privileged
  mutable filesystem state.

## User Experience Shape

The successful user experience remains ordinary:

```text
open file -> contents appear
edit -> text changes
save -> saved
external edit -> current disk bytes appear or a clear conflict is reported
```

The Echo sequence for opening a host file is:

```text
user selects path
-> client asks Echo file aperture to observe host material
-> host capability supplies bytes, digest, stat posture, and path evidence
-> Echo maps or creates a file site
-> Echo compares observed material with retained causal history
-> Echo admits initial import, no-change observation, or external-change intent
-> Echo returns a witnessed file projection
-> client renders the projection
```

The Echo sequence for saving a host file is:

```text
user saves
-> client submits desired content or delta against a basis
-> Echo admits canonical content intents or obstructs stale/invalid basis
-> Echo authorizes host materialization for the admitted projection
-> host capability writes bytes
-> Echo observes/verifies host bytes
-> Echo records materialization receipt or obstruction
-> client reports saved only if verification posture permits it
```

Normal clients should translate Echo posture into product language. The history
or inspector surface may expose receipts and witnesses directly.

## Runtime Contract Sketch

The exact Wesley surface is future work, but the Echo artifact should include
operations with this shape:

```text
acceptHostFileObservation(input) -> HostFileObservationReceipt
reconcileHostFileObservation(input) -> FileReconciliationReceipt
projectFileContent(input) -> FileContentReading
proposeFileContent(input) -> FileContentIntentReceipt
materializeFileProjection(input) -> FileMaterializationReceipt
explainFileState(input) -> FileProvenanceReading
```

The contract should expose generic file-aperture types:

```text
FileSiteId
HostFileIdentity
HostFileObservation
HostFileFingerprint
FileContentDigest
FileContentProjection
DirectoryProjection
FileBasisToken
ExternalChangeReceipt
ContentIntentReceipt
MaterializationReceipt
FileObstructionReason
```

Jedit-facing editor nouns such as buffer, cursor, selection, vim mode, panel,
or tab do not belong in this shared Echo artifact. Those remain application
vocabulary above the file aperture.

WARP DRIVE-facing POSIX nouns such as inode, file handle, FUSE request, errno,
or `.warp` synthetic path also do not belong in the shared Echo artifact. Those
remain membrane vocabulary below the user tool surface.

## Data And State Model

The file aperture should distinguish these authorities:

| Concept                | Authority                   | Notes                                                          |
| ---------------------- | --------------------------- | -------------------------------------------------------------- |
| Host path string       | Host/client evidence        | Useful to locate material, never the causal authority.         |
| Host file bytes        | Host observation material   | Must be admitted before a client can render it as Echo-backed. |
| File site identity     | Echo                        | Durable coordinate for file-like history.                      |
| Content digest         | Echo-retained evidence      | Names bytes, but does not by itself prove semantic coordinate. |
| File projection        | Echo reading                | Bounded reading over a basis and aperture.                     |
| Basis token            | Echo                        | Required for lawful mutations and stale-basis detection.       |
| Materialization effect | Echo-authorized host effect | Not successful until verified or honestly obstructed.          |

Initial import, external edit, user edit, and save verification should all flow
through Echo admission and retention. They differ by cause and direction, not
by whether they matter causally.

## Diff Ownership

Echo owns the diff from observed or proposed bytes to canonical content
intents.

Clients may supply helpful hints, such as a text edit range or a known previous
reading id, but Echo decides the admitted mutation shape. For text files, Echo
may choose range edits, line patches, rope chunks, or whole-file replacement
according to policy. For binary files, Echo may choose chunk replacement or
whole-blob replacement. The caller does not own causal canonicalization.

External host edits and Jedit edits should converge into the same retained
content history shape once admitted.

## Accessibility Posture

Rendered clients must not expose Echo jargon during happy-path file open and
save. When an obstruction affects the user, clients should translate it into
clear product language while preserving agent-readable receipts.

Examples:

```text
The file changed on disk. Jedit reopened the current disk contents.
```

```text
Could not save because the file changed since this buffer was opened.
```

```text
Saved, but verification failed: the file on disk does not match the saved content.
```

## Localization Posture

The Echo artifact should return stable obstruction codes and structured facts,
not user-facing English as the authority. Jedit and WARP DRIVE own localized
copy for their surfaces.

## Agent Inspectability

An agent must be able to inspect:

- the file site;
- the host observation receipt;
- the basis token used for a content intent;
- whether a host snapshot became an initial import, no-change observation, or
  external-change admission;
- the materialization receipt;
- the verification digest;
- the obstruction reason when reconstruction or save is unavailable.

This should be possible without scraping pixels, terminal prose, or app-owned
private caches.

## Non-Goals

- Do not implement a FUSE mount in Echo.
- Do not make Echo core a filesystem runtime.
- Do not put Jedit editor nouns in Echo core.
- Do not make WARP DRIVE's fixture tree or inode scaffolding the shared
  contract.
- Do not let applications keep private causal file logs as fallback authority.
- Do not guarantee reconstruction after Echo retention policy redacts, prunes,
  encrypts, or obstructs required support material.

## Tests To Write First

- Opening an unknown host file admits an initial import before returning a file
  projection.
- Opening a known unchanged host file returns an Echo projection and records
  honest no-change observation posture.
- Opening a known changed host file admits an external-change transition by
  diffing observed bytes against the causal basis.
- Saving with a current basis admits content intents, materializes the
  projection, verifies host digest, and records a materialization receipt.
- Saving with a stale basis returns a typed obstruction and does not claim
  saved.
- Materialization write succeeds but verification digest differs returns a
  materialization obstruction.
- Missing retained evidence prevents reconstruction instead of falling back to
  host bytes or app logs.

## Acceptance Criteria

- Jedit can open an arbitrary host file through Echo and render the exact bytes
  after Echo admits or reconciles the observation.
- Jedit can save through Echo and report saved only after admitted projection
  materialization is verified.
- WARP DRIVE can consume the same Echo file projection and content intent
  contract without owning file causality.
- Echo reports explainable receipts for initial import, external change,
  content admission, obstruction, materialization, and verification.
- No Jedit or WARP DRIVE implementation nouns enter Echo production core.

## Validation Plan

Early validation should be contract-level before client polish:

```text
cargo test -p warp-core --test file_aperture_tests
cargo test -p warp-cli --test file_aperture_cli_tests
cargo xtask dind run
```

Client follow-through should later prove the same semantic cases through Jedit
and WARP DRIVE harnesses.

## Playback / Witness

The first useful witness should be:

```text
1. Create a temporary host file with "one".
2. Ask Echo to open it through the file aperture.
3. Verify Echo admits initial import and returns "one".
4. Mutate the host file externally to "two".
5. Ask Echo to open it again.
6. Verify Echo admits external change and returns "two".
7. Submit a save to "three" against the current basis.
8. Verify Echo materializes "three" and records matching host digest.
```

Jedit should eventually run this witness without showing Echo terminology in
the happy path. WARP DRIVE should eventually run the same witness through
normal POSIX reads and writes.

## Risks

- Path evidence can be mistaken for durable identity. Mitigation: make file
  site identity explicit and treat paths as host observations.
- Diff canonicalization can become editor-specific. Mitigation: Echo owns
  canonical content intents; editor hints remain optional evidence.
- Whole-file replacement can be wasteful. Mitigation: permit policy-selected
  diff granularity without changing the external contract.
- Happy-path UX can leak causal jargon. Mitigation: clients translate
  structured posture into product copy and keep receipts inspectable elsewhere.
- A client can accidentally keep a private fallback ledger. Mitigation: tests
  must prove missing Echo evidence obstructs reconstruction.

## Follow-On Work

- Define the Wesley contract artifact for the file aperture.
- Decide whether the implementation crate is named `echo-file-aperture`,
  `echo-fs-runtime`, or another standard-artifact name.
- Add a no-app-nouns guard for file aperture production code that allows
  generic file vocabulary but rejects Jedit/WARP DRIVE implementation nouns.
- Build shared conformance fixtures consumed by Echo, Jedit, and WARP DRIVE.
- Add an explanation surface equivalent to WARP DRIVE's proposed
  `/.warp/why/<path>` and Jedit's history drawer.

## Retrospective

Not yet implemented. This note records the architectural target before the
contract and runtime slices begin.
