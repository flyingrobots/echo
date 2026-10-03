<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Causal Receipt History

Causal receipt history is Echo's bounded, application-facing reading of
admitted transition evidence. It lets an application traverse the receipts that
remain reachable at an explicit worldline basis without maintaining a
process-local history authority.

The history projection does not define undo semantics. An application contract
decides which transitions are invertible and how to derive an inverse. Echo
retains and projects the exact receipts, submissions, causal parents, and
contract evidence required to make that decision honestly.

## Authority

The projection is reconstructed from:

- recovered receipt-correlation records;
- witnessed submission records;
- retained canonical ingress envelopes;
- worldline provenance at the requested basis;
- typed causal-parent roles retained by ingress;
- installed-contract evidence attached to each admitted transition.

An in-memory map may index these facts after recovery, but it is not an
independent source of truth. Deleting and rebuilding that index must not change
the returned history.

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

Echo sorts receipt coordinates canonically before applying traversal direction.
A continuation cursor binds:

- worldline;
- basis frontier;
- direction;
- exact last receipt returned.

The cursor is exclusive. Reusing it continues immediately after that receipt.
A cursor minted for a different worldline, basis, or direction is rejected.
A cursor whose exact receipt is no longer available is also rejected; Echo does
not silently skip the missing evidence.

Because the basis is fixed, history admitted after that basis does not alter the
page sequence. Repeating the same traversal after WAL recovery produces the
same entries and continuation boundaries.

## Obstructions

The query returns `CausalReceiptHistoryObstruction` when:

- the page size is outside the fixed bound;
- the worldline is unavailable;
- the basis exceeds the recovered frontier;
- provenance cannot establish a nonzero basis;
- the cursor scope or exact cursor receipt is invalid;
- duplicate correlations claim one exact receipt;
- receipt fields disagree with their exact coordinate;
- witnessed submission records or envelopes are missing;
- submission, envelope, and correlation ingress identities disagree;
- envelope parents disagree with retained correlation parents;
- a cited parent receipt is unavailable;
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

See [/topics/ContractInverseAdmission](/topics/ContractInverseAdmission) for
inverse derivation and admission, and [/topics/WAL](/topics/WAL) for the durable
carrier and recovery boundary.
