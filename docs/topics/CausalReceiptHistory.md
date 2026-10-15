<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Causal Receipt History

Causal receipt history is Echo's bounded, application-facing reading of
admitted transition evidence. It lets an application traverse the receipts that
remain reachable at an explicit worldline basis without maintaining a
process-local history authority.

Which submissions a tick decided is owned by
[submission attribution](SubmissionAttribution.md): the history walks it and
checks every other source against it.

The history projection does not define undo semantics. An application contract
decides which transitions are invertible and how to derive an inverse. Echo
retains and projects the exact receipts, submissions, causal parents, and
contract evidence required to make that decision honestly.

## Authority

The projection is reconstructed from:

- each retained tick's committed attribution section, which names the
  submissions that tick decided;
- recovered receipt-correlation records;
- witnessed submission records;
- retained canonical ingress envelopes;
- worldline provenance at the requested basis;
- typed causal-parent roles retained by ingress;
- installed-contract evidence attached to each admitted transition.

An in-memory map may index these facts after recovery, but it is not an
independent source of truth. Deleting and rebuilding that index must not change
the returned history. For an attributed tick, a correlation the attribution
does not name, or an attributed ticketed submission with no correlation, is an
obstruction rather than a shorter page.

The query does not:

- stage ingress;
- tick the scheduler;
- append to the WAL;
- mutate application state;
- choose an undo target;
- make a transition reversible;
- grant receipt or retention authority.

## Request

`TrustedRuntimeApp::causal_receipt_history` accepts a
`CausalReceiptHistoryRequest` containing:

- one `WorldlineId`;
- an explicit `basis_frontier_tick` at or before the recovered frontier;
- `OldestFirst` or `NewestFirst` traversal;
- a page size from 1 through
  `MAX_CAUSAL_RECEIPT_HISTORY_PAGE_SIZE`;
- an optional Echo-issued continuation cursor.

The basis is a frontier coordinate. Basis zero names the replay base. Basis N
includes admitted receipt correlations whose transition advanced the worldline
to a frontier no later than N.

## Entries

Each `CausalReceiptHistoryEntry` exposes:

- the exact `CausalTickReceiptRef`;
- the witnessed submission id;
- installed-contract evidence, when present;
- the canonical set of cited parent receipts;
- an optional typed `ContractInverseDerivation`.

An ordinary transition has no inverse derivation. A contract inverse names both
the exact transition selected for inversion and the receipt set used as its
current admission basis. Echo recovers this distinction from typed retained
ingress-parent roles rather than from a process-local request map.

## Pagination

The query walks retained ticks in the requested direction, not the correlation
index. Within one tick, submissions are in submission order, which is also the
order of their receipt coordinates, so traversal order matches canonical
receipt order.

One page has three separate limits:

- at most `page_size` returned entries;
- at most `MAX_CAUSAL_RECEIPT_HISTORY_INSPECTED_TICKS` inspected ticks;
- at most `MAX_CAUSAL_RECEIPT_HISTORY_INSPECTED_RECORDS` inspected attribution
  records or legacy correlations.

Each inspected record costs a fixed number of index lookups and at most
`MAX_CAUSAL_RECEIPT_HISTORY_PARENTS` parent checks, so these limits bound a
page's work, not only its output. A long run of ticks with no submissions costs
several pages; a page that spends its inspection budget may return no entries
and still continue.

A continuation cursor binds:

- worldline;
- basis frontier;
- direction;
- the next tick to inspect;
- within that tick, the last submission already returned.

A cursor can therefore resume inside a tick. A cursor minted for a different
worldline, basis, or direction is rejected, and so is one naming a tick at or
beyond the basis or a submission its tick does not account for. Each page makes
forward progress, so a traversal always ends.

Because the basis is fixed, history admitted after that basis does not alter the
page sequence. Repeating the same traversal after WAL recovery produces the
same entries and continuation boundaries.

## Completeness

Each page reports the ticks it inspected, `inspected` (first and last, in
traversal order), and a `CausalReceiptHistoryCompleteness` claim about exactly
those ticks under the pinned basis:

- `Established` when every inspected tick committed an attribution section, so
  every submission those ticks decided was returned or lies ahead of the
  cursor;
- `UnestablishedForLegacyTicks { first_legacy_tick }` when an inspected tick
  was written before attribution existed. Its retained correlations are
  returned, but Echo cannot prove none is missing.

The claim never extends beyond `inspected`. A traversal is complete only if
every one of its pages is `Established`, so legacy uncertainty cannot be hidden
by splitting a traversal into pages.

## Obstructions

The query returns `CausalReceiptHistoryObstruction` when:

- the page size is outside the fixed bound;
- the worldline is unavailable;
- the basis exceeds the recovered frontier;
- provenance cannot establish a nonzero basis;
- the cursor scope or position is invalid;
- retained provenance holds no transition for a tick before the basis;
- an attributed tick's indexed correlation is not named by its attribution;
- an attributed, ticketed submission has no receipt correlation (lost
  correlation material);
- an attributed submission was never ticketed, so it has no receipt identity;
- duplicate correlations claim one exact receipt;
- an indexed receipt has no correlation record, or its fields disagree with
  its exact coordinate;
- retained provenance does not hold the transition a receipt names, or holds
  it with a different commit or receipt content (a retention gap), for any
  returned receipt or the cursor's receipt;
- witnessed submission records or envelopes are missing;
- submission, envelope, and correlation ingress identities disagree;
- envelope parents disagree with retained correlation parents;
- a cited parent receipt is unavailable, or retained provenance does not hold
  its transition as named;
- an entry cites more than `MAX_CAUSAL_RECEIPT_HISTORY_PARENTS` causal parents;
- typed inverse derivation is missing or ambiguous.

These conditions are obstructions, not empty pages. Returning less history than
the retained evidence claims would make downstream provenance appear valid when
it is not.

## Undo And Redo

An application that implements undo should:

1. Read causal receipt history at the current worldline basis.
2. Apply its contract-owned policy to choose an invertible transition.
3. Ask the installed contract to derive the inverse from retained evidence.
4. Submit that inverse through the normal WAL-acknowledged admission path.
5. Treat the resulting receipt as a new forward causal transition.

Redo follows the same law by inverting the admitted inverse transition. Neither
operation moves a hidden cursor backward or removes history. Process restart
must not change which transition the application selects for the same basis and
policy.

See [contract inverse admission](ContractInverseAdmission.md) for
inverse derivation and admission, and [WAL](WAL.md) for the durable
carrier and recovery boundary.
