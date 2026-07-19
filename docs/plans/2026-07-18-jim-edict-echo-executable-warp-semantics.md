<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Jim × Edict × Echo: Executable WARP Semantics Inventory

- **Status:** Historical discovery reconciled by
  [ADR 0023](../adr/0023-admitted-executable-operation-packages.md)
- **Date:** 2026-07-18
- **Implementation status:** Campaign 1 has not started

ADR 0023 is authoritative where this source inventory's earlier hypotheses
differ from the accepted executable-operation boundary. The accepted campaign
is recorded in section 13; the inventory of broader Jim and Echo capabilities
does not place those capabilities in scope for the first vertical.

## Source basis

This report is based on source code, not repository documentation, at these
authoritative repository heads:

- [Jedit `c70e12d73b4b`](https://github.com/flyingrobots/jedit/commit/c70e12d73b4b00bc92412bab67e1761f7dd22f82)
- [Echo `6615d3a97731`](https://github.com/flyingrobots/echo/commit/6615d3a97731a076fb4945bb6da083e82f55710d)
- [Edict `da5da887c1fa`](https://github.com/flyingrobots/edict/commit/da5da887c1fa089a3f82f4d29d0799eb6e155f31)

Historical commits in this report are immutable evidence coordinates. They are
not permission to assume that current repository heads remain unchanged.

## Executive finding

WARP is the right candidate for Echo's executable machine, but it is not yet
the executable machine implemented in the repositories.

The source currently contains:

```text
Edict meaning
→ Target IR naming a target intrinsic
→ generated registration/helper metadata
→ host-supplied Rust executor + footprint callbacks
→ TickDelta
→ Echo receipt
```

It does not yet contain:

```text
Edict meaning
→ canonical executable WARP program
→ Echo-owned WARP interpreter
→ TickDelta
→ Echo receipt
```

This is not a documentation discrepancy. It identifies the missing
architectural center.

Jim's Echo-facing needs fall into five jurisdictions:

```text
Application mutations, authored by Jedit in Edict
├── CreateBufferWorldline
├── ReplaceRange
│   ├── Insert
│   └── Delete
└── DeclareCheckpoint

Bounded observations, authored as optics
├── TextWindow
├── CausalLineDiff
├── WhyRange
└── FullTextSnapshot / export materialization

Capability crossings
├── Read file / import bytes
└── Write file / export bytes

Causal-topology operations, owned by Echo kernel law
├── Observe historical coordinate
├── Fork strand
├── Braid membership/lifecycle
├── Compare or plan settlement
└── Settle suffix as import, conflict, or lawful plurality

Runtime judgments, exclusively Echo-owned
├── submit
├── admit
├── schedule
├── evaluate/apply
├── commit
├── emit outcome evidence
├── persist
└── recover
```

This is one algebra with multiple jurisdictions.

Two prerequisites must be resolved before `ReplaceRange` can honestly become
executable Edict meaning:

1. Jedit must select and bind one canonical text-graph schema and migration
   posture; package generation or admission must refuse a missing or mismatched
   declaration.
2. Echo does not yet have a serialized DPO-program interpreter with
   deterministic self-validation and a structurally separate target-verifier
   path. Independently implemented evidence is a separate, stronger grade.

## 1. What Jim executes through Echo today

The current GraphQL/Wesley corridor defines exactly three mutations and one
query:

- `createBufferWorldline`
- `replaceRangeAsTick`
- `declareCheckpoint`
- `textWindow`

Source:
[jedit/contracts/jedit/echo-text.graphql#L1-L71@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/contracts/jedit/echo-text.graphql#L1-L71)

The TypeScript host port exposes precisely the same four operations.

Sources:
[jedit/src/ports/echo-text-contract-host.ts#L9-L21@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/ports/echo-text-contract-host.ts#L9-L21),
[jedit/src/ports/echo-text-contract-host.ts#L140-L145@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/ports/echo-text-contract-host.ts#L140-L145)

### 1.1 Open / `CreateBufferWorldline`

Open is presently a composite operation:

1. Jedit reads the file through its local `EditorFilePort`.
2. The loaded text becomes `initialText`.
3. Jedit calls the Echo host.
4. If the buffer already exists, the native host returns its current snapshot,
   without a creation receipt.
5. Otherwise, it submits `CreateBufferWorldline`.
6. Jedit subsequently performs a `TextWindow` observation.

The pre-Echo file read occurs here:
[jedit/src/app/workspace/workspace-text-open-basis.ts#L55-L80@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/workspace-text-open-basis.ts#L55-L80)

The create-then-observe composition occurs here:
[jedit/src/app/workspace/workspace-text-commands.ts#L203-L234@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/workspace-text-commands.ts#L203-L234)

The native existing-or-create behavior occurs here:
[jedit/native/jedit-echo-host/src/host.rs#L101-L134@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/host.rs#L101-L134)

The actual create planner:

- refuses an existing buffer;
- builds a persistent rope from initial UTF-8;
- creates an initial head;
- writes the mutable buffer record pointing at that head.

Source:
[jedit/native/jedit-echo-host/src/rope.rs#L346-L383@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope.rs#L346-L383)

The correct future expression is:

```text
file-read capability
→ witnessed bytes or missing-file result
→ CreateBufferWorldline application operation
→ TextWindow optic
```

The file-read capability provides bytes. It must not define buffer semantics.

### 1.2 Insert / Replace / Delete

These are one semantic operation, not three runtime operations:

```text
Insert(start, text)
= ReplaceRange(start, start, text)

Delete(start, end)
= ReplaceRange(start, end, "")

Replace(start, end, text)
= ReplaceRange(start, end, text)
```

The adapter performs exactly these reductions:
[jedit/src/adapters/workspace-production-text-session.ts#L69-L84@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/adapters/workspace-production-text-session.ts#L69-L84)

The product command layer independently presents all three edit kinds.

Sources:
[jedit/src/app/workspace/workspace-text-commands.ts#L46-L58@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/workspace-text-commands.ts#L46-L58),
[jedit/src/app/workspace/workspace-text-commands.ts#L282-L305@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/workspace-text-commands.ts#L282-L305)

The current handwritten `ReplaceRange` law:

1. Resolves the buffer's canonical head.
2. Refuses a stale supplied basis.
3. Refuses malformed or out-of-bounds ranges.
4. Reads the replaced bytes.
5. Refuses exact no-ops.
6. Checked-increments head sequence and buffer version.
7. Splits the persistent rope at both range boundaries.
8. Builds a rope for the replacement text.
9. Deterministically joins and balances the resulting rope.
10. Creates a new immutable head.
11. Creates rewrite and diff facts.
12. Changes the buffer's canonical head and version.
13. Returns the exact read/write footprint and consequence.

Source:
[jedit/native/jedit-echo-host/src/rope.rs#L385-L475@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope.rs#L385-L475)

#### Persistent derivation, not destructive replacement

`ReplaceRange` does not delete old rope nodes.

Jim has a persistent rope. The implementation creates new content-addressed
nodes and a new head, reuses unaffected structure, and advances the buffer's
canonical-head reference. Old heads and old rope structure remain available as
historical support.

The emitted patch contains only node upserts and attachment writes. It contains
no node deletion:
[jedit/native/jedit-echo-host/src/rope.rs#L217-L237@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope.rs#L217-L237)

The future graph rewrite should therefore look like:

```text
preserve old head and old rope
preserve all reusable subtrees
add replacement blobs/leaves/branches
add new head
add rewrite/diff evidence
replace Buffer ─canonicalHead→ OldHead
     with Buffer ─canonicalHead→ NewHead
```

This is naturally expressible in DPO, but it is persistent derivation rather
than destructive replacement.

### 1.3 `DeclareCheckpoint`

Checkpoint declaration:

- resolves a requested retained head;
- verifies that the head belongs to the requested buffer;
- creates a content-addressed checkpoint proposition from worldline, head, and
  reason;
- does not advance the text head.

Source:
[jedit/native/jedit-echo-host/src/rope.rs#L309-L344@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope.rs#L309-L344)

This should be a small application-semantic graph write:

```text
Buffer ─contains→ BasisHead
+ DeclareCheckpoint invocation
→ Checkpoint ─worldline→ Buffer
             └─basis→ BasisHead
             + reason
```

It is not a text mutation.

The current source nevertheless wraps it in a `MutationPlan`, which is the false
vocabulary identified by Jedit issue #287:
[jedit/native/jedit-echo-host/src/rope.rs#L240-L256@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope.rs#L240-L256)

The underlying checkpoint reason domain contains:

- manual-save
- autosave
- retention-boundary
- export
- import

Source:
[jedit/native/jedit-echo-host/src/contract.rs#L175-L195@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/contract.rs#L175-L195)

The current product adapter maps only manual-save and autosave:
[jedit/src/adapters/workspace-production-text-session.ts#L43-L46@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/adapters/workspace-production-text-session.ts#L43-L46)

### 1.4 `TextWindow`

`TextWindow` is the one currently executing bounded observation. It:

- takes a buffer and explicit basis head;
- verifies buffer/head ownership;
- checks the byte range;
- applies `maxBytes`;
- traverses only relevant rope support;
- returns the largest complete UTF-8 prefix;
- reports supporting leaf/blob identities;
- computes line projections.

Sources:
[jedit/native/jedit-echo-host/src/rope/window.rs#L43-L102@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope/window.rs#L43-L102),
[jedit/native/jedit-echo-host/src/rope/window.rs#L129-L217@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope/window.rs#L129-L217)

This is already shaped like a bounded optic:

```text
basis
+ byte aperture
+ byte budget
→ text projection
+ exact supporting leaves/blobs
+ completeness posture
```

Its implementation is still a host-supplied observer closure, not executable
admitted semantics. The generated code accepts an arbitrary observing
function:
[jedit/native/jedit-echo-host/src/generated/contract.rs#L681-L753@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/generated/contract.rs#L681-L753)

## 2. What Jim wants but cannot currently execute

The broader `ProductionTextSession` asks for ten operations:

- open buffer
- insert
- replace
- delete
- multi-range edit
- checkpoint
- text window
- causal line diff
- export snapshot
- explain range

Source:
[jedit/src/app/workspace/production-text-session.ts#L196-L209@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/production-text-session.ts#L196-L209)

Only open, insert/replace/delete, checkpoint, and text window are connected.
Multi-range edit, causal line diff, export, and range explanation explicitly
fail closed with a message saying that the current Wesley corridor does not
implement them:
[jedit/src/adapters/workspace-production-text-session.ts#L40-L62@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/adapters/workspace-production-text-session.ts#L40-L62)

### 2.1 `MultiRangeEdit`

The input is an ordered collection of byte ranges and replacement strings:
[jedit/src/app/workspace/production-text-session.ts#L104-L114@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/production-text-session.ts#L104-L114)

It is declared but unsupported by the current adapter.

Future choices must be explicit:

- canonicalize ranges and perform one atomic bounded operation;
- lower it into a transaction containing multiple `ReplaceRange` applications;
- or refuse it as unsupported.

It must not become an accidental loop whose result depends on caller iteration
order or intermediate canonical-head movement.

### 2.2 `CausalLineDiff`

The requested optic takes:

- buffer;
- basis head;
- next head;
- maximum byte count;
- maximum line count;
- maximum rewrite count;
- maximum marker count.

Source:
[jedit/src/app/workspace/production-text-causal-line-diff.ts#L6-L24@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/production-text-causal-line-diff.ts#L6-L24)

The intended reading cites:

- inserted and deleted line counts;
- tick receipts;
- rewrite identities;
- diff identities;
- per-line markers and deletion markers;
- observer version.

Source:
[jedit/src/ports/text-authority-evidence.ts#L67-L95@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/ports/text-authority-evidence.ts#L67-L95)

This should be a bounded causal optic over retained `RopeRewrite` and `RopeDiff`
evidence. It is not a mutation and should not be recomputed from two privileged
snapshots.

### 2.3 `ExplainRange` / `:why`

The intended range explanation includes:

- exact queried basis and range;
- complete or partial coverage;
- continuation;
- fragments mapped to leaf/blob support;
- imported or rewrite origins;
- rewrite, diff, and tick receipt citations;
- related checkpoints;
- checkpoint-to-causal-anchor associations.

Source:
[jedit/src/ports/jedit-why-range.ts#L34-L128@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/ports/jedit-why-range.ts#L34-L128)

The UI computes a byte range around the cursor and requests `explainRange`. The
current adapter refuses it:
[jedit/src/app/workspace/workspace-why-range.ts#L70-L109@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/workspace-why-range.ts#L70-L109)

This is another bounded optic:

```text
basis head
+ byte range
+ evidence budget
→ origin fragments
+ rewrite/checkpoint evidence
+ partial/complete posture
+ continuation
```

### 2.4 Save / Export

Save is currently a three-crossing composition:

```text
exportSnapshot from Echo
→ local filesystem write
→ declare manual-save checkpoint
```

The export step invokes `exportSnapshot`, runs a materialization preflight, and
then directly calls `saveEditorFile`:
[jedit/src/app/workspace/workspace-text-commands.ts#L336-L368@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/workspace-text-commands.ts#L336-L368)

After successful export, the reducer schedules checkpoint declaration:
[jedit/src/app/workspace/workspace-text-runtime-state.ts#L317-L348@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/workspace-text-runtime-state.ts#L317-L348)

The current `exportSnapshot` session operation is unsupported, so the production
save chain cannot complete through the generated corridor.

The correct future decomposition is:

```text
FullText optic at explicit head
→ complete retained reading
→ FileWrite capability request
→ witnessed filesystem outcome
→ DeclareCheckpoint(manual-save or export)
```

The write capability performs the effect. It must not decide which text is
authoritative, which head was exported, or whether a checkpoint proposition is
lawful.

## 3. Jim's causal command vocabulary

Jim already presents these user-level commands:

- ttd
- strand
- braid

All three are deliberately unavailable.

Sources:
[jedit/src/app/workspace/workspace-command-catalog.ts#L88-L119@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/workspace-command-catalog.ts#L88-L119),
[jedit/src/app/workspace/command-line-dispatch.ts#L52-L100@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/command-line-dispatch.ts#L52-L100)

### 3.1 `ttd`

The command vocabulary supports:

- canonical head;
- current observer basis;
- previous tick;
- arbitrary tick or relative coordinate according to its usage.

It explicitly promises historical observation without moving canonical head.

Sources:
[jedit/src/app/workspace/workspace-command-catalog.ts#L88-L96@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/workspace-command-catalog.ts#L88-L96),
[jedit/src/app/workspace/workspace-command-catalog.ts#L143-L167@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/workspace-command-catalog.ts#L143-L167)

This is a kernel-resolved historical observation coordinate followed by normal
optics. It is not an application mutation.

### 3.2 `strand`

The UI vocabulary includes:

- list;
- create from the current observer basis;
- switch to main.

Source:
[jedit/src/app/workspace/workspace-command-catalog.ts#L169-L191@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/workspace-command-catalog.ts#L169-L191)

The source-backed mapping is:

- strand list → causal-topology optic;
- strand new from here → Echo kernel fork;
- strand switch → client/runtime routing posture, not an application graph
  rewrite.

Echo implements a strand as a speculative lane relation over a child worldline,
with an immutable exact fork basis. It explicitly says that a strand is not a
separate scheduler or substrate.

Sources:
[echo/crates/warp-core/src/strand.rs#L1-L27@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/strand.rs#L1-L27),
[echo/crates/warp-core/src/strand.rs#L75-L117@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/strand.rs#L75-L117)

Echo's `fork_strand`:

- replays the source at the requested historical tick;
- forks provenance;
- materializes the child frontier;
- creates child writer heads;
- registers the strand;
- rolls runtime and provenance back if anything fails.

Source:
[echo/crates/warp-core/src/coordinator.rs#L1751-L1848@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/coordinator.rs#L1751-L1848)

This is kernel causal law.

### 3.3 `braid`

The UI vocabulary includes:

- view;
- preview;
- admit.

Source:
[jedit/src/app/workspace/workspace-command-catalog.ts#L193-L215@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/workspace-command-catalog.ts#L193-L215)

Echo's actual braid is not a merge algorithm. It is an append-only coordination
log with these events:

- braid created;
- member woven;
- settlement finalized;
- plural braid collapsed.

Source:
[echo/crates/warp-core/src/braid.rs#L99-L128@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/braid.rs#L99-L128)

Its membership projections are explicitly read models, not admission
authority:
[echo/crates/warp-core/src/braid.rs#L130-L213@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/braid.rs#L130-L213)

Its fold validates lifecycle status, sequence, duplicate membership, disclosure
posture, settlement posture, and collapse witness:
[echo/crates/warp-core/src/braid.rs#L215-L335@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/braid.rs#L215-L335)

The accurate mapping is:

```text
braid view
→ observe braid event log and membership projection

braid preview
→ compare strand suffix + produce deterministic settlement plan

braid admit
→ execute a named Echo settlement policy
```

Echo's settlement code already separates those acts:

- compare is explicitly read-only inspection;
- `plan_with_policy` produces deterministic import/conflict/plural decisions;
- `settle_with_policy` appends corresponding causal consequences.

Sources:
[echo/crates/warp-core/src/settlement.rs#L700-L757@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/settlement.rs#L700-L757),
[echo/crates/warp-core/src/settlement.rs#L760-L956@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/settlement.rs#L760-L956),
[echo/crates/warp-core/src/settlement.rs#L958-L1057@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/settlement.rs#L958-L1057)

The settlement decisions are:

- import candidate;
- conflict artifact;
- plural alternative.

Source:
[echo/crates/warp-core/src/settlement.rs#L225-L293@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/settlement.rs#L225-L293)

#### Reserve settle for causal-topology settlement

The word settle should remain reserved for causal-topology settlement.

The source already gives it a specific meaning: settling a strand suffix into
its base worldline as imports, conflicts, or retained plurality under a named
policy. Reusing settle to mean closing the disposition of every ordinary
invocation would introduce ambiguity.

For ordinary submitted work, use a term such as:

- `recordOutcome`;
- `finalizeDisposition`;
- `commitOutcome`.

Retain `settleStrand`, `settleBraid`, and `SettlementPolicy` for the causal operation
already implemented.

## 4. Latent concepts that are not Echo operations yet

### 4.1 Undo and redo

Undo and redo appear only as command/durability vocabulary and help text. There
is no `ProductionTextSession` or Echo operation for them.

Source:
[jedit/src/app/workspace/workspace-buffer-durability.ts#L44-L105@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/app/workspace/workspace-buffer-durability.ts#L44-L105)

Future undo should not silently rewind or rewrite causal history. It should be
one of:

- a new admitted forward replacement;
- an explicitly authored inverse operation;
- a compensation;
- a new strand from an earlier basis.

The exact law remains unimplemented.

### 4.2 Point anchors

Jedit has a local deterministic point-anchor transformation contract with
left/right bias and replacement-delta behavior:
[jedit/src/domain/anchor-transform-contract.ts#L7-L136@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/domain/anchor-transform-contract.ts#L7-L136)

This is currently a local pure utility, not an Echo operation.

Jedit also models a separate `RopeCheckpointAnchored` association containing
checkpoint, causal-anchor fact, and receipt identities:
[jedit/src/domain/graph-rope-types.ts#L250-L258@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/domain/graph-rope-types.ts#L250-L258)

This supports the distinction:

```text
declare checkpoint
≠
anchor checkpoint to a causal coordinate
```

They should remain different propositions and operations.

### 4.3 Structural maintenance

The TypeScript domain models:

- split leaf;
- merge leaves;
- rotate left;
- rotate right;
- rebalance branch.

Source:
[jedit/src/domain/graph-rope-types.ts#L199-L219@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/domain/graph-rope-types.ts#L199-L219)

The native rope performs splitting and deterministic height balancing
internally, but its admitted native fact inventory does not contain
`RopeStructuralMaintenance` and the planner emits no such facts.

Sources:
[jedit/native/jedit-echo-host/src/records.rs#L7-L14@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/records.rs#L7-L14),
[jedit/native/jedit-echo-host/src/rope/tree.rs#L101-L193@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope/tree.rs#L101-L193)

A future design must decide whether balancing steps are:

- internal interpreter trace;
- retained structural-maintenance facts;
- or implied by the canonical rope-construction law.

That decision affects receipts, replay, and `:why`.

## 5. Jim's graph data model

### 5.1 There is not currently one canonical Jim text graph

The TypeScript semantic model contains ten fact kinds:

1. `BufferWorldline`
2. `RopeHead`
3. `RopeBranch`
4. `RopeLeaf`
5. `TextBlob`
6. `RopeRewrite`
7. `RopeDiff`
8. `RopeStructuralMaintenance`
9. `RopeCheckpoint`
10. `RopeCheckpointAnchored`

Sources:
[jedit/src/domain/graph-rope-types.ts#L12-L21@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/domain/graph-rope-types.ts#L12-L21),
[jedit/src/domain/graph-rope-types.ts#L85-L270@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/domain/graph-rope-types.ts#L85-L270)

The native runtime contains only eight:

1. buffer
2. blob
3. leaf
4. branch
5. head
6. rewrite
7. diff
8. checkpoint

Source:
[jedit/native/jedit-echo-host/src/records.rs#L7-L14@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/records.rs#L7-L14)

These are not merely two serializations of identical propositions.

| Concept           | TypeScript semantic model                                        | Native runtime model                                                         |
| ----------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Buffer            | Creation receipt and initial head                                | Buffer key, projection path, mutable canonical head, version                 |
| Head              | Worldline, root, basis, receipt, byte/line metrics, content hash | Buffer, optional basis/root, byte/UTF-16/line metrics, root digest, sequence |
| Rewrite           | Receipt, replacement blob, diff identity, full range             | Buffer, basis/next head, range, inserted length                              |
| Diff              | Ordered equal/delete/insert spans                                | Aggregate range and inserted/deleted lengths                                 |
| Maintenance       | Explicit modeled facts                                           | Absent                                                                       |
| Checkpoint anchor | Explicit modeled fact                                            | Absent                                                                       |

Sources:
[jedit/src/domain/graph-rope-types.ts#L85-L270@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/domain/graph-rope-types.ts#L85-L270),
[jedit/native/jedit-echo-host/src/records.rs#L25-L106@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/records.rs#L25-L106)

ADR 0023 turns this discovery into a Jedit-owned admission prerequisite. Before
authoritative `.edict` source can produce an admissible real package, Jedit must
publish one versioned declaration that selects the canonical proposition,
identity, and codec law; classifies the TypeScript-only maintenance and anchor
facts; and states the compatibility or migration posture for the native JSON
model. The package, lawpack, program, and public schemas must close over that
exact declaration or fail with a typed schema-closure refusal. Edict and Echo
must not select a side of this semantic fork.

### 5.2 Current graph relationships are opaque JSON fields

Every native fact is serialized to JSON. Content-addressed identities hash
those JSON bytes. On read, the entire node attachment is decoded into a Rust
struct:
[jedit/native/jedit-echo-host/src/records.rs#L123-L173@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/records.rs#L123-L173)

The planner emits:

- an Echo node with a type;
- one atom attachment containing the JSON fact.

It emits no Echo edges for:

- head → root;
- branch → left/right;
- leaf → blob;
- rewrite → basis/next;
- checkpoint → head.

Source:
[jedit/native/jedit-echo-host/src/rope.rs#L217-L237@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope.rs#L217-L237)

The domain is conceptually graph-shaped, but the current WARP graph sees each
Jim fact largely as an opaque attributed node. A true DPO interpreter cannot
match Jim's semantic relationships if they exist only inside host-decoded
JSON.

### 5.3 A plausible canonical graph shape

After freezing the authoritative propositions, relationships could be
represented structurally:

```text
BufferWorldline
├─ canonicalHead ───────────────→ RopeHead
├─ initialHead ─────────────────→ RopeHead
└─ attributes: bufferKey, projectionPath, version

RopeHead
├─ worldline ───────────────────→ BufferWorldline
├─ basis ───────────────────────→ RopeHead?
├─ root ────────────────────────→ RopeBranch | RopeLeaf
└─ attributes:
   byteLength, utf16Length, lineCount, sequence, rootDigest

RopeBranch
├─ left ────────────────────────→ RopeBranch | RopeLeaf
├─ right ───────────────────────→ RopeBranch | RopeLeaf
└─ attributes:
   byteLength, utf16Length, lineBreaks, height

RopeLeaf
├─ blob ────────────────────────→ TextBlob
└─ attributes:
   byteStart, byteLength, utf16Length, lineBreaks

TextBlob
└─ attributes:
   digest, canonical bytes or retained-content reference

RopeRewrite
├─ worldline ───────────────────→ BufferWorldline
├─ basis ───────────────────────→ RopeHead
├─ next ────────────────────────→ RopeHead
├─ replacement ────────────────→ TextBlob
├─ diff ────────────────────────→ RopeDiff
└─ attributes:
   startByte, endByte, receipt/evidence references

RopeCheckpoint
├─ worldline ───────────────────→ BufferWorldline
├─ basis ───────────────────────→ RopeHead
└─ attributes: reason

RopeCheckpointAnchored
├─ checkpoint ──────────────────→ RopeCheckpoint
└─ attributes:
   causalAnchorId, anchorFactId, anchorReceiptId
```

Scalar fields can remain in canonical typed attachment records. Semantic
relationships should be typed edges, or the future WARP pattern algebra must
provide equally canonical typed-field matching. They cannot remain dependent
on arbitrary Rust JSON decoding.

Echo's store already has typed nodes, outbound/inbound edge indexes, and
node/edge attachment planes:
[echo/crates/warp-core/src/graph.rs#L29-L64@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/graph.rs#L29-L64)

## 6. How ReplaceRange becomes a WARP program

A single static DPO span is not enough for arbitrary ReplaceRange.

The current operation performs variable-depth traversal and recursive
rebuilding:

- leaves are capped at 4,096 UTF-8 bytes;
- leaf boundaries preserve UTF-8;
- splitting descends recursively by subtree metrics;
- joining recursively rebalances by height;
- hashes and aggregate metrics are recomputed.

Sources:
[jedit/native/jedit-echo-host/src/rope/tree.rs#L9-L44@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope/tree.rs#L9-L44),
[jedit/native/jedit-echo-host/src/rope/tree.rs#L69-L193@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope/tree.rs#L69-L193)

The executable unit therefore needs to be a bounded transactional WARP
program, not merely one rule:

```text
EchoOperationProgramV1: ReplaceRange executable meaning
│
├─ bind invocation
│  ├─ buffer
│  ├─ explicit basis
│  ├─ start: u64
│  ├─ end: u64
│  └─ replacement: bytes
│
├─ validate
│  ├─ operation/schema identity
│  ├─ basis belongs to buffer
│  ├─ basis is canonical where required
│  ├─ start ≤ end ≤ byteLength
│  ├─ UTF-8 boundaries
│  ├─ no arithmetic overflow
│  └─ no-op law
│
├─ traverse/split
│  ├─ deterministic worklist
│  ├─ bounded rope descent
│  └─ exact read support
│
├─ construct
│  ├─ replacement blobs and leaves
│  ├─ shared persistent subtrees
│  ├─ deterministic joins/rebalancing
│  └─ aggregate metrics and hashes
│
├─ derive consequence
│  ├─ new RopeHead
│  ├─ RopeRewrite
│  ├─ RopeDiff
│  └─ new canonical-head relation
│
├─ derive exact footprint
│
└─ atomically prepare one TickPatch or one obstruction
```

The intermediate traversal/work graph must not leak into authoritative history
step by step. It is private evaluation state. The entire successful program
produces one atomic tick consequence.

The current planner already has the correct high-level separation:

```text
read-only source
→ MutationPlan with exact reads/writes
→ footprint
→ one emitted TickDelta
```

Sources:
[jedit/native/jedit-echo-host/src/rope.rs#L79-L180@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope.rs#L79-L180),
[jedit/native/jedit-echo-host/src/rope.rs#L187-L237@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope.rs#L187-L237)

That separation should survive. The handwritten Rust function determining the
plan must not.

### 6.1 Closed attachment algebra

The WARP interpreter needs a small, versioned, deterministic intrinsic profile
containing operations such as:

- checked `u64` arithmetic and comparison;
- canonical byte length;
- bounded byte slicing;
- UTF-8 validation and boundary checking;
- UTF-16 code-unit count;
- line-break count;
- canonical digest/content identity;
- height comparison and maximum;
- canonical ordered rule/work-item selection.

It should not contain:

```text
jedit_replace_range(...)
```

That would be the callback architecture renamed intrinsic.

Generic intrinsics must be individually specified, bounded, and Echo-owned. The
Jedit lawpack composes them into rope semantics.

## 7. The source proves the callback gap

Echo's current `RewriteRule` contains:

- a native matcher function pointer;
- a native executor function pointer;
- a native footprint function pointer.

Its `PatternGraph` is only a vector of type identifiers, and the source says that
the left pattern is currently unused.

Sources:
[echo/crates/warp-core/src/rule.rs#L11-L50@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/rule.rs#L11-L50),
[echo/crates/warp-core/src/rule.rs#L73-L103@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/rule.rs#L73-L103)

Jedit's generated mutation code constructs an empty `PatternGraph` and accepts
host-supplied executor and footprint functions:
[jedit/native/jedit-echo-host/src/generated/contract.rs#L990-L1033@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/generated/contract.rs#L990-L1033)

Jedit binds those callbacks to the handwritten planner.

Sources:
[jedit/native/jedit-echo-host/src/contract.rs#L34-L77@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/contract.rs#L34-L77),
[jedit/native/jedit-echo-host/src/contract.rs#L111-L142@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/contract.rs#L111-L142)

The provider-native Edict helper currently does the same thing with stronger
identity evidence. Its generated API asks for `ProviderMutationHooksV1` and says
it is constructing a package proposal from an explicit host
executor/footprint binding:
[echo/crates/echo-edict-provider-lowerer/src/lib.rs#L1238-L1283@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/echo-edict-provider-lowerer/src/lib.rs#L1238-L1283)

Echo's provider type states the limitation directly: identity equality detects
accidental cross-binding but does not prove that arbitrary Rust callbacks
semantically implement the claims. The hooks contain the executor and footprint
function pointers:
[echo/crates/warp-core/src/provider_contract.rs#L76-L126@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/provider_contract.rs#L76-L126)

This is the architectural defect, stated directly in source.

## 8. What Edict currently lowers

Edict's generic Target IR currently contains:

- operation profile;
- input constraints;
- evaluation budget;
- requirements;
- ordered steps;
- each step's semantic effect and target intrinsic;
- result expression.

Source:
[edict/crates/edict-syntax/src/target_ir.rs#L175-L214@da5da887c1fa](https://github.com/flyingrobots/edict/blob/da5da887c1fa089a3f82f4d29d0799eb6e155f31/crates/edict-syntax/src/target_ir.rs#L175-L214)

Core already has fixed-width integer values represented by width plus canonical
textual value, record/field/call expressions, predicates, requirements,
effects, obstruction maps, and budgets:
[edict/crates/edict-syntax/src/core_ir.rs#L118-L234@da5da887c1fa](https://github.com/flyingrobots/edict/blob/da5da887c1fa089a3f82f4d29d0799eb6e155f31/crates/edict-syntax/src/core_ir.rs#L118-L234)

This is a useful semantic foundation, but the current Echo-specific lowerer is
deliberately fixture-specific:

- one Core coordinate, `a.b@1`;
- one operation, `a.b@1.t`;
- one effect, `target.replace`;
- one target intrinsic, `echo.dpo@1.replace`;
- one obstruction.

Source:
[echo/crates/echo-edict-provider-lowerer/src/lib.rs#L23-L60@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/echo-edict-provider-lowerer/src/lib.rs#L23-L60)

It accepts exactly one intent and one effect step. It refuses authored optics.

Sources:
[echo/crates/echo-edict-provider-lowerer/src/lib.rs#L1580-L1690@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/echo-edict-provider-lowerer/src/lib.rs#L1580-L1690),
[echo/crates/echo-edict-provider-lowerer/tests/lowerer_contract.rs#L1263-L1293@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/echo-edict-provider-lowerer/tests/lowerer_contract.rs#L1263-L1293)

The existing `echo.span-ir/v1` must not yet be described as a complete executable
DPO program. Today it is a bounded effect/intrinsic plan plus identity evidence.

## 9. The accepted first-version executable object

[ADR 0023](../adr/0023-admitted-executable-operation-packages.md) retires
`WarpProgramV1` and `SemanticPackage` as normative first-version names. They
were discovery hypotheses, not compatibility aliases. The accepted public
container is `ExecutableOperationPackageV1`; its subordinate target-relative
executable artifact is `EchoOperationProgramV1`. A future portable WARP program
or broader semantic package needs a separate compatibility decision and cannot
silently alias either first-version identity.

The admitted package owns the public contract:

```text
ExecutableOperationPackageV1
├── public operation coordinate
├── invocation, output, and obstruction schemas
├── explicit basis contract
├── budget and declared-footprint contracts
├── authority and invocation-admission requirements
├── result and obstruction interpretation
├── Edict source/Core/target semantic closure
├── Jedit canonical text-schema declaration
├── exact Jedit lawpack coordinate and digest
├── exact EchoOperationProgramV1 bytes and digest
├── evaluator ABI and intrinsic-profile requirement
├── provenance and verifier evidence grades
└── package root and digests
```

The bound program supplies only executable meaning:

```text
EchoOperationProgramV1
├── invocation-to-graph binding checked against the package
├── typed graph and attachment schema identities
├── closed declarative DPO rewrite/evaluation rules
│   ├── L
│   ├── K
│   ├── R
│   ├── positive application conditions
│   └── negative application conditions
├── bounded deterministic control and match-selection laws
├── private working-state schema
├── attachment intrinsic profile
├── executable footprint derivation
├── static resource requirements or ceilings
├── typed result and obstruction construction checked against package schemas
└── evaluator ABI/version
```

An operation or schema coordinate carried inside program bytes is a consistency
claim, not a contract declaration. A program digest does not confer operation
identity, invocability, authority, or installation eligibility. Echo starts
installation and invocation lookup from an admitted operation-package identity
and follows its closed binding to the program. An application package does not
install itself, and Echo cannot install or invoke a naked program digest.

The broader optic and causal-policy ideas inventoried earlier are intentionally
non-normative here. `TextWindow`, capability execution, and causal-topology
migration are outside this vertical.

## 10. Receipt binding

Current provider-native receipt evidence binds:

- installed package;
- package reference/root;
- operation ID and coordinate;
- Target IR identity;
- scheduler rule ID.

Source:
[echo/crates/warp-core/src/provider_contract.rs#L1181-L1262@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/provider_contract.rs#L1181-L1262)

Provider-v1 evidence does not bind:

- executable operation-program digest;
- evaluator ABI/version;
- attachment intrinsic profile;
- deterministic program control law;
- executed rule/match trace or canonical patch identity.

This is consistent with the present callback architecture because no admitted
executable artifact exists to bind.

New executable-operation evidence must bind these common identities:

```text
admitted public operation coordinate, package root, and package-admission evidence
Echo-owned installed-operation identity
exact submission/invocation and canonical input digest
caller-authority and Echo invocation-admission evidence
Jedit canonical text-schema declaration and lawpack digests
EchoOperationProgramV1 digest
evaluator ABI/version
intrinsic profile digest
EchoOperationEvaluationBasisV1 bytes and digest
delegated and consumed budgets
declared and actual footprint/support digests
private execution/trace identity
scheduler-rule and ordered composition-member identities
```

It must then bind exactly one terminal variant:

```text
Committed
├── committed composite TickPatch digest
├── typed result identity
└── resulting frontier identity

NotCommitted
├── typed evaluation obstruction or commit-ineligibility identity
├── canonical outcome digest
└── canonical no-parent-patch evidence
```

`NotCommitted` never carries a committed patch digest. The exact executed
program digest is required but remains subordinate to the admitted operation
package. A full rule trace payload may be optional, but its retained identity is
not: recovery must be able to attribute the execution evidence without
rerunning ambient application code.

## 11. Additional constraints exposed by the source

### 11.1 Fixed-width coordinates remain unresolved end to end

There are currently three coordinate representations:

- native Rust uses `u64`;
- GraphQL/Wesley narrows values through `Int`/`i32`;
- TypeScript exposes branded structures whose value is `number`.

The GraphQL narrowing is explicit:
[jedit/contracts/jedit/echo-text.graphql#L41-L60@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/contracts/jedit/echo-text.graphql#L41-L60)

The native host converts `u64` to `i32` and refuses larger values:
[jedit/native/jedit-echo-host/src/host.rs#L420-L427@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/host.rs#L420-L427)

The domain coordinate wrapper holds a TypeScript `number`:
[jedit/src/domain/graph-rope-types.ts#L36-L59@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/domain/graph-rope-types.ts#L36-L59)

Removing GraphQL Int is necessary but insufficient. The generated JavaScript
client also needs an exact representation, likely `bigint` or a canonical
fixed-width wrapper rather than an unrestricted `number`.

### 11.2 `ReplaceRange`'s explicit basis is supplied by the host

The GraphQL operation carries `basisHeadId`, but the TypeScript host request does
not:
[jedit/src/ports/echo-text-contract-host.ts#L40-L45@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/ports/echo-text-contract-host.ts#L40-L45)

The native host snapshots the current canonical head and inserts it into the
generated intent:
[jedit/native/jedit-echo-host/src/host.rs#L136-L167@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/host.rs#L136-L167)

In the future law, Jim's canonical command should name its basis explicitly.
Ambient host lookup must not silently determine which historical coordinate the
operation meant.

### 11.3 Jim's native runtime is single-worldline

The native host creates one hard-coded worldline and one `AcceptAll` default
writer:
[jedit/native/jedit-echo-host/src/host.rs#L375-L409@c70e12d73b4b](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/host.rs#L375-L409)

The `ttd`, `strand`, and `braid` vocabulary is disconnected both at the operation
layer and at the host-session model. A future Jim/Echo boundary must carry
explicit worldline and causal coordinates.

### 11.4 Tick patches are already the correct commit boundary

Echo defines a tick patch as a replayable prescriptive delta sufficient to
reconstruct a tick without rerunning matching or scheduling:
[echo/crates/warp-core/src/tick_patch.rs#L3-L13@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/tick_patch.rs#L3-L13)

Its canonical operation vocabulary includes node, edge, attachment, and
warp-instance changes:
[echo/crates/warp-core/src/tick_patch.rs#L95-L165@6615d3a97731](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/tick_patch.rs#L95-L165)

The new evaluator does not need a second commit format. It needs to derive a
lawful `TickPatch` from an admitted package's subordinate
`EchoOperationProgramV1`.

## 12. Constitutional laws

### Executable Semantics Law

Every parent-visible consequence committed by Echo must be derivable solely
from:

- Jedit-owned application meaning preserved and lowered through an admitted
  Edict operation package;
- that package's exact subordinate `EchoOperationProgramV1`;
- Echo-owned runtime state;
- versioned deterministic Echo intrinsics;
- explicitly admitted capability results.

Ambient host code, callbacks, plugins, or handwritten application
implementations must not determine semantic outcomes.

Evidence for this law must use ADR 0023's exact grades: deterministic
self-validation, structurally separate verification, or independently
implemented conformance evidence. A separate crate or call path does not, by
itself, establish implementation independence.

### Application Meaning Law

Jedit owns application semantics. Edict preserves, canonicalizes, and lowers
that authored meaning. Echo admits packages and invocations, installs admitted
executable meaning, evaluates and schedules it, and commits parent-visible
consequences.

A generated client may construct canonical input and submit it. It must not
know how to perform `ReplaceRange`.

### Capability Law

Executable semantics may request explicitly named capabilities. Echo owns
capability admission. Capabilities perform effects but do not define
application-semantic consequence.

### Jurisdiction Law

The transformation algebra may be uniform. Authority is not.

- Echo kernel rules own runtime judgment.
- Echo causal rules own worldline and settlement topology.
- Jedit-owned, Edict-authored application rules define declarative application
  meaning; they do not confer Echo admission or commitment authority.
- Edict preserves and lowers application meaning; it does not prove runtime
  execution.
- Echo admits invocations and owns evaluation, scheduling, and commitment of
  their consequences.
- Future Edict-authored optics may define bounded read plans, but optics are not
  part of this campaign.
- Capabilities own only their named effect crossing.

### Atomic Program Law

A multi-rule WARP program may use private deterministic evaluation state, but a
successful application operation commits one atomic `TickPatch`. Intermediate
work states do not become accidental application history.

### Evidence Law

Evidence from one crossing cannot stand in for another.

Package identity, operation identity, Target IR identity, scheduler rule
identity, executable program identity, capability evidence, and committed
consequence are distinct propositions and must be bound explicitly where
required.

## 13. Accepted bounded campaign

ADR 0023 resolves the architecture question raised by this inventory. Campaign
1 implementation has not started. Work proceeds as four separately reviewed
and explicitly merged stages:

1. add only the minimal Edict operation prerequisites: exact fixed-width
   values, explicit basis, Jedit lawpack-resource binding through Core and
   target identity, and generated operation-facing package/client artifacts;
2. add Echo's hook-free bounded-operation evaluator with one tiny generic
   `EchoOperationProgramV1`, exact invocation admission, private evaluation,
   budget and footprint enforcement, exact-basis commit, typed terminal
   evidence, WAL, and recovery;
3. add the Jedit-owned canonical text-schema declaration, real `ReplaceRange`
   lawpack resource and program, plus a finite independently implemented
   differential oracle corpus whose limits are stated honestly; and
4. invoke the generated real operation from current Jim, prove the legacy
   replacement route unreachable, and cut over `ReplaceRange` only.

The first accepted witness is exactly:

```text
real Jim command
→ explicit basis-bearing generated invocation
→ Echo-admitted Edict-authored operation
→ Echo-owned deterministic evaluation
→ one Echo-committed buffer consequence or typed no-patch outcome
→ execution evidence binding the admitted executable semantics
```

This campaign does not add checkpoint migration, `TextWindow`, save or other
capability execution, Cyber Kitten runtime machinery, observer routing, durable
child lanes or child-local ticks, wormholes, holograms, Continuum transport, or
fork/braid/settlement migration. Those remain useful inventory, not acceptance
witnesses for this vertical.

## Final assessment

WARP algebra should be Echo's executable semantics, provided WARP program means
a closed, typed, deterministic, resource-bounded, transactional
graph-transformation program—not merely one DPO span and not a named host
intrinsic.

Fork, braid, and settlement belong to the same broad algebraic universe, but
they already operate under Echo's causal-kernel jurisdiction. They must not
become ordinary Jedit-authored application rules.

Only after the admitted executable-operation contract and Jedit-owned canonical
text-schema declaration close exactly should the real Jedit
`ReplaceRange.edict` package become admissible. Otherwise, Edict would be asked
to compile into a target that still has no complete executable constitutional
meaning, or would canonize an unresolved Jedit schema fork.
