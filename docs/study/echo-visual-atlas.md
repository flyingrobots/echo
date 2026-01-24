<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Visual Atlas

> Standalone diagrams for understanding Echo's architecture.
> These diagrams complement the main guide "What Makes Echo Tick?"

---

## 1. The Complete Tick Pipeline

```mermaid
flowchart TB
    subgraph PHASE1["Phase 1: BEGIN"]
        B1[engine.begin]
        B2[Increment tx_counter]
        B3[Add to live_txs]
        B4[Return TxId]
        B1 --> B2 --> B3 --> B4
    end

    subgraph PHASE2["Phase 2: APPLY (0..N times)"]
        A1[engine.apply]
        A2{Matcher?}
        A3[Compute Footprint]
        A4[Create PendingRewrite]
        A5[Enqueue to Scheduler]
        A6[NoMatch]
        A1 --> A2
        A2 -->|true| A3 --> A4 --> A5
        A2 -->|false| A6
    end

    subgraph PHASE3["Phase 3: COMMIT"]
        subgraph DRAIN["3a. Drain"]
            D1[Radix sort pending]
            D2[Canonical order]
        end
        subgraph RESERVE["3b. Reserve"]
            R1[For each rewrite]
            R2{Footprint conflict?}
            R3[Accept]
            R4[Reject + witness]
            R1 --> R2
            R2 -->|no| R3
            R2 -->|yes| R4
        end
        subgraph EXECUTE["3c. Execute"]
            E1[For each accepted]
            E2[Call executor]
            E3[Emit to TickDelta]
            E1 --> E2 --> E3
        end
        subgraph MERGE["3d. Merge"]
            M1[Collect all deltas]
            M2[Sort by key+origin]
            M3[Dedupe/detect conflicts]
            M1 --> M2 --> M3
        end
        subgraph FINALIZE["3e. Finalize"]
            F1[Apply ops to state]
            F2[Update indexes]
            F1 --> F2
        end
        DRAIN --> RESERVE --> EXECUTE --> MERGE --> FINALIZE
    end

    subgraph PHASE4["Phase 4: HASH"]
        H1[BFS reachable nodes]
        H2[Canonical encode]
        H3[BLAKE3 state_root]
        H4[BLAKE3 patch_digest]
        H5[Compute commit_hash]
        H1 --> H2 --> H3 --> H4 --> H5
    end

    subgraph PHASE5["Phase 5: RECORD"]
        REC1[Append Snapshot]
        REC2[Append Receipt]
        REC3[Append Patch]
        REC1 --> REC2 --> REC3
    end

    PHASE1 --> PHASE2 --> PHASE3 --> PHASE4 --> PHASE5
```

---

## 2. BOAW Parallel Execution Model

```mermaid
flowchart LR
    subgraph INPUT["Input"]
        I[ExecItems<br/>n items]
    end

    subgraph PARTITION["Partition Phase"]
        P[partition_into_shards]
        S0[Shard 0]
        S1[Shard 1]
        S2[...]
        S255[Shard 255]
        P --> S0
        P --> S1
        P --> S2
        P --> S255
    end

    subgraph EXECUTE["Execute Phase (Parallel)"]
        W0[Worker 0<br/>TickDelta]
        W1[Worker 1<br/>TickDelta]
        W2[Worker 2<br/>TickDelta]
        WN[Worker N<br/>TickDelta]
    end

    subgraph STEAL["Work Stealing"]
        AC[AtomicUsize<br/>next_shard]
        AC -.->|fetch_add| W0
        AC -.->|fetch_add| W1
        AC -.->|fetch_add| W2
        AC -.->|fetch_add| WN
    end

    subgraph MERGE["Merge Phase"]
        MG[merge_deltas]
        SORT[Sort by key+origin]
        DEDUP[Dedupe identical]
        MG --> SORT --> DEDUP
    end

    subgraph OUTPUT["Output"]
        O[Canonical Ops<br/>deterministic]
    end

    I --> P
    S0 --> W0
    S1 --> W1
    S2 --> W2
    S255 --> WN
    W0 --> MG
    W1 --> MG
    W2 --> MG
    WN --> MG
    DEDUP --> O
```

---

## 3. Virtual Shard Routing

```mermaid
flowchart TD
    subgraph NODEID["NodeId (32 bytes)"]
        B0["byte 0"]
        B1["byte 1"]
        B2["byte 2"]
        B3["byte 3"]
        B4["byte 4"]
        B5["byte 5"]
        B6["byte 6"]
        B7["byte 7"]
        REST["bytes 8-31<br/>(ignored)"]
    end

    subgraph EXTRACT["Extract First 8 Bytes"]
        LE["u64::from_le_bytes<br/>[b0,b1,b2,b3,b4,b5,b6,b7]"]
    end

    subgraph MASK["Apply Shard Mask"]
        AND["val & 0xFF<br/>(NUM_SHARDS - 1)"]
    end

    subgraph RESULT["Shard ID"]
        SID["0..255"]
    end

    B0 --> LE
    B1 --> LE
    B2 --> LE
    B3 --> LE
    B4 --> LE
    B5 --> LE
    B6 --> LE
    B7 --> LE
    LE --> AND --> SID
```

### Test Vectors (Frozen Protocol)

| Input (first 8 bytes) | LE u64               | Shard      |
| --------------------- | -------------------- | ---------- |
| `0xDEADBEEFCAFEBABE`  | `0xBEBAFECAEFBEADDE` | 190 (0xBE) |
| `0x0000000000000000`  | `0x0000000000000000` | 0          |
| `0x2A00000000000000`  | `0x000000000000002A` | 42         |
| `0xFFFFFFFFFFFFFFFF`  | `0xFFFFFFFFFFFFFFFF` | 255        |

---

## 4. Two-Plane WARP Architecture

```mermaid
graph TB
    subgraph SKELETON["Skeleton Plane (Structure)"]
        direction TB
        N1["Node A<br/>id: 0x1234"]
        N2["Node B<br/>id: 0x5678"]
        N3["Node C<br/>id: 0x9ABC"]

        N1 -->|"edge:link<br/>id: 0xE001"| N2
        N1 -->|"edge:child<br/>id: 0xE002"| N3
        N2 -->|"edge:ref<br/>id: 0xE003"| N3
    end

    subgraph ALPHA["Attachment Plane (α)"]
        direction TB
        A1["N1.α['title']<br/>Atom{string, 'Home'}"]
        A2["N2.α['url']<br/>Atom{string, '/page/b'}"]
        A3["N3.α['body']<br/>Atom{html, '&lt;p&gt;...&lt;/p&gt;'}"]
        A4["N3.α['portal']<br/>Descend('child-instance')"]
    end

    N1 -.- A1
    N2 -.- A2
    N3 -.- A3
    N3 -.- A4

    subgraph DESCENDED["Descended Instance"]
        direction TB
        C1["Child Root<br/>id: 0xCCC1"]
        C2["Child Node<br/>id: 0xCCC2"]
        C1 --> C2
    end

    A4 -.->|"Descend pointer"| C1
```

---

## 5. GraphView Contract Enforcement

```mermaid
flowchart TD
    subgraph EXECUTOR["Executor Function"]
        EX["fn executor(view: GraphView, scope: &NodeId, delta: &mut TickDelta)"]
    end

    subgraph READ["Read Path (GraphView)"]
        R1["view.node(id)"]
        R2["view.edges_from(id)"]
        R3["view.attachment(id, key)"]
        R4["view.has_edge(id)"]

        R1 --> GS
        R2 --> GS
        R3 --> GS
        R4 --> GS
    end

    subgraph GS["GraphStore (Immutable)"]
        NODES["nodes: BTreeMap"]
        EDGES["edges_from: BTreeMap"]
        ATTACH["attachments: BTreeMap"]
    end

    subgraph WRITE["Write Path (TickDelta)"]
        W1["delta.emit(UpsertNode)"]
        W2["delta.emit(UpsertEdge)"]
        W3["delta.emit(SetAttachment)"]
        W4["delta.emit(DeleteNode)"]

        W1 --> OPS
        W2 --> OPS
        W3 --> OPS
        W4 --> OPS
    end

    subgraph OPS["Accumulated Ops"]
        OPLIST["Vec&lt;(WarpOp, OpOrigin)&gt;"]
    end

    EX --> READ
    EX --> WRITE

    style GS fill:#e8f5e9
    style OPS fill:#fff3e0
```

---

## 6. State Root Hash Computation

```mermaid
flowchart TD
    subgraph BFS["1. Deterministic BFS"]
        START["Start at root"]
        VISIT["Visit reachable nodes"]
        DESCEND["Follow Descend() attachments"]
        COLLECT["Collect reachable set"]
        START --> VISIT --> DESCEND --> COLLECT
    end

    subgraph ENCODE["2. Canonical Encoding"]
        subgraph INSTANCE["Per Instance (BTreeMap order)"]
            IH["warp_id header"]
            subgraph NODE["Per Node (ascending NodeId)"]
                NH["node_id[32]"]
                NT["node_type[32]"]
                subgraph EDGE["Per Edge (ascending EdgeId)"]
                    EH["edge_id[32]"]
                    ET["edge_type[32]"]
                    ED["to_node[32]"]
                end
                subgraph ATTACH["Per Attachment"]
                    AK["key_len[8] + key"]
                    AT["type_id[32]"]
                    AV["value_len[8] + value"]
                end
            end
        end
    end

    subgraph HASH["3. BLAKE3 Digest"]
        STREAM["Byte stream"]
        DIGEST["state_root[32]"]
        STREAM --> DIGEST
    end

    BFS --> ENCODE --> HASH
```

---

## 7. Commit Hash v2 Structure

```mermaid
flowchart LR
    subgraph INPUTS["Commit Hash Inputs"]
        V["version[4]<br/>protocol tag"]
        P["parents[]<br/>parent hashes"]
        SR["state_root[32]<br/>graph hash"]
        PD["patch_digest[32]<br/>ops hash"]
        PI["policy_id[4]<br/>aion policy"]
    end

    subgraph CONCAT["Concatenation"]
        BYTES["version || parents || state_root || patch_digest || policy_id"]
    end

    subgraph OUTPUT["Output"]
        CH["commit_hash[32]<br/>BLAKE3"]
    end

    V --> BYTES
    P --> BYTES
    SR --> BYTES
    PD --> BYTES
    PI --> BYTES
    BYTES --> CH
```

---

## 8. WSC Snapshot Format

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                        WSC SNAPSHOT FILE                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ HEADER (fixed size)                                                 │ │
│  │ ┌──────────┬──────────┬──────────┬──────────┬──────────┐          │ │
│  │ │  magic   │ version  │ node_cnt │ edge_cnt │ offsets  │          │ │
│  │ │  8 bytes │ 8 bytes  │ 8 bytes  │ 8 bytes  │ 8×N bytes│          │ │
│  │ └──────────┴──────────┴──────────┴──────────┴──────────┘          │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ NODES TABLE (sorted by NodeId, 8-byte aligned)                      │ │
│  │ ┌─────────────────┬─────────────────┬─────────────────┐            │ │
│  │ │    NodeRow      │    NodeRow      │    NodeRow      │  ...       │ │
│  │ │    64 bytes     │    64 bytes     │    64 bytes     │            │ │
│  │ │ [id:32][type:32]│ [id:32][type:32]│ [id:32][type:32]│            │ │
│  │ └─────────────────┴─────────────────┴─────────────────┘            │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ EDGES TABLE (sorted by EdgeId, 8-byte aligned)                      │ │
│  │ ┌─────────────────────────┬─────────────────────────┐              │ │
│  │ │       EdgeRow           │       EdgeRow           │  ...         │ │
│  │ │       128 bytes         │       128 bytes         │              │ │
│  │ │[id:32][from:32][to:32]  │[id:32][from:32][to:32]  │              │ │
│  │ │[type:32]                │[type:32]                │              │ │
│  │ └─────────────────────────┴─────────────────────────┘              │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ OUT_INDEX (per-node ranges into out_edges)                          │ │
│  │ ┌──────────────┬──────────────┬──────────────┐                     │ │
│  │ │    Range     │    Range     │    Range     │  ...                │ │
│  │ │   16 bytes   │   16 bytes   │   16 bytes   │                     │ │
│  │ │[start:8][len:8]│[start:8][len:8]│[start:8][len:8]│                │ │
│  │ └──────────────┴──────────────┴──────────────┘                     │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ ATTACHMENT INDEX (per-slot ranges)                                  │ │
│  │ Similar structure to OUT_INDEX                                      │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │ BLOB ARENA (variable-length payloads)                               │ │
│  │ ┌─────────────────────────────────────────────────────────────┐    │ │
│  │ │ [payload bytes...] [payload bytes...] [payload bytes...] ...│    │ │
│  │ └─────────────────────────────────────────────────────────────┘    │ │
│  │ Referenced by (offset: u64, length: u64) tuples                     │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 9. Footprint Independence Check

```mermaid
flowchart TD
    subgraph REWRITE1["Rewrite A"]
        R1_READ["reads: {N1, N2}"]
        R1_WRITE["writes: {N3}"]
    end

    subgraph REWRITE2["Rewrite B"]
        R2_READ["reads: {N4, N5}"]
        R2_WRITE["writes: {N6}"]
    end

    subgraph REWRITE3["Rewrite C"]
        R3_READ["reads: {N1, N3}"]
        R3_WRITE["writes: {N7}"]
    end

    subgraph CHECK["Independence Check"]
        C1{{"A ∩ B"}}
        C2{{"A ∩ C"}}
        C3{{"B ∩ C"}}
    end

    subgraph RESULT["Results"]
        OK1["A || B: OK<br/>(no overlap)"]
        CONFLICT["A || C: CONFLICT<br/>(A.write ∩ C.read = {N3})"]
        OK2["B || C: OK<br/>(no overlap)"]
    end

    R1_WRITE --> C1
    R2_WRITE --> C1
    R1_WRITE --> C2
    R3_READ --> C2
    R2_WRITE --> C3
    R3_WRITE --> C3

    C1 --> OK1
    C2 --> CONFLICT
    C3 --> OK2

    style CONFLICT fill:#ffcdd2
    style OK1 fill:#c8e6c9
    style OK2 fill:#c8e6c9
```

---

## 9b. FootprintGuard Enforcement Flow

```mermaid
flowchart TD
    EXEC["execute_item_enforced()"]
    SNAP["ops_before = delta.ops_len()"]
    CATCH["catch_unwind(executor)"]
    SCAN["FOR op IN delta.ops()[ops_before..]"]
    CHECK["check_op(op, footprint, kind)"]
    VIOL{"Violation?"}
    PANIC{"Executor panicked?"}
    ERR["Err(FootprintViolation)"]
    RESUME["resume_unwind(payload)"]
    OK["Ok(())"]

    EXEC --> SNAP --> CATCH --> SCAN --> CHECK --> VIOL
    VIOL -->|Yes| ERR
    VIOL -->|No| PANIC
    PANIC -->|Yes| RESUME
    PANIC -->|No| OK

    style ERR fill:#ffcdd2
    style RESUME fill:#fff9c4
    style OK fill:#c8e6c9
```

**Key:** When footprint enforcement is active (`cfg(debug_assertions)` or
`footprint_enforce_release` feature), every `ExecItem` execution is wrapped
by `execute_item_enforced()`. The guard validates all newly-emitted ops
against the declared footprint. Write violations take precedence over
executor panics—ensuring the developer always sees the root cause.

---

## 10. Complete Data Flow: Intent to Render

```mermaid
sequenceDiagram
    autonumber
    participant U as User
    participant V as Viewer
    participant H as Session Hub
    participant E as Engine
    participant S as Scheduler
    participant B as BOAW
    participant G as GraphStore
    participant W as WSC

    U->>V: Click action
    V->>V: Encode intent bytes
    V->>H: ingest_intent(bytes)
    H->>E: forward intent

    Note over E: Phase 1: BEGIN
    E->>E: begin() → TxId

    Note over E: Intent Processing
    E->>E: dispatch_next_intent(tx)
    E->>G: GraphView lookup
    G-->>E: intent data

    Note over E: Phase 2: APPLY
    E->>S: apply(tx, rule, scope)
    S->>G: matcher(view, scope)
    G-->>S: match result
    S->>S: compute footprint
    S->>S: enqueue PendingRewrite

    Note over E: Phase 3: COMMIT
    E->>S: commit(tx)
    S->>S: radix sort (drain)
    S->>S: independence check (reserve)

    Note over B: Parallel Execution
    S->>B: execute_parallel(items)
    B->>B: partition into shards
    par Worker 0
        B->>G: read via GraphView
        G-->>B: data
        B->>B: emit to TickDelta
    and Worker 1
        B->>G: read via GraphView
        G-->>B: data
        B->>B: emit to TickDelta
    and Worker N
        B->>G: read via GraphView
        G-->>B: data
        B->>B: emit to TickDelta
    end
    B->>B: merge_deltas (canonical)
    B-->>S: merged ops

    S->>G: apply ops

    Note over E: Phase 4: HASH
    E->>G: compute state_root
    G-->>E: hash
    E->>E: compute commit_hash

    Note over E: Phase 5: RECORD
    E->>W: store snapshot
    E->>E: append to history

    Note over H: Emit to Tools
    E->>H: WarpDiff
    H->>V: WarpFrame

    Note over V: Apply & Render
    V->>V: apply_op (each op)
    V->>V: verify state_hash
    V->>V: render frame
    V->>U: Display result
```

---

## 11. Viewer Event Loop

```mermaid
flowchart TD
    subgraph FRAME["Frame Loop"]
        START[frame start]

        subgraph DRAIN["1. Drain Session"]
            DN[drain_notifications]
            DF[drain_frames]
        end

        subgraph PROCESS["2. Process Frames"]
            PF[process_frames]
            SNAP{Snapshot?}
            DIFF{Diff?}
            APPLY[apply_op each]
            VERIFY[verify hash]
        end

        subgraph EVENTS["3. Handle Events"]
            UE[apply_ui_event]
            REDUCE[reduce pure]
            EFFECTS[run effects]
        end

        subgraph RENDER["4. Render"]
            MATCH{screen?}
            TITLE[draw_title]
            VIEW[draw_view]
            HUD[draw_hud]
        end

        END[frame end]

        START --> DRAIN
        DN --> DF
        DF --> PROCESS
        PF --> SNAP
        SNAP -->|yes| APPLY
        PF --> DIFF
        DIFF -->|yes| APPLY
        APPLY --> VERIFY
        VERIFY --> EVENTS
        UE --> REDUCE
        REDUCE --> EFFECTS
        EFFECTS --> RENDER
        MATCH -->|Title| TITLE
        MATCH -->|View| VIEW
        VIEW --> HUD
        TITLE --> END
        HUD --> END
    end
```

---

_Visual Atlas generated 2026-01-18. Use alongside "What Makes Echo Tick?" for complete understanding._
