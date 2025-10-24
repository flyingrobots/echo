# Branch Merge Conflict Playbook

Merging timelines is where Echo’s temporal sandbox shines. This playbook defines how we detect, surface, and resolve conflicts when combining branch diffs.

---

## Conflict Types

1. **Component Value Conflict**
   - Same entity & component modified differently in both branches.

2. **Structural Conflict**
   - One branch deletes entity/component the other modifies.

3. **Order Conflict**
   - Sequencing-sensitive actions (e.g., timeline events) reordered.

4. **Resource Conflict**
   - Shared resources (inventory counts, singleton states) diverge.

---

## Detection Pipeline

1. Identify lowest common ancestor node `L`.
2. Collect diffs `Δα` (from `L` to branch α head) and `Δβ` (to branch β head).
3. For each entity/component touched:
   - Compare mutation timestamps (relative order from diff metadata).
   - If both branches modify same slot [=> conflict].
4. For deletions vs modifications, flag structural conflict.
5. Accumulate conflict records for resolution stage.

Conflict record structure:
```ts
interface MergeConflict {
  entityId: EntityHandle;
  componentType: number | null; // null for entity-level conflict
  type: "value" | "structural" | "order" | "resource";
  branchA: DiffEntry;
  branchB: DiffEntry;
}
```

---

## Resolution Strategies

1. **Manual Selection (Default)**
   - Present conflicts in inspector; designer chooses branch (A wins, B wins, custom).
   - Record decision for determinism (stored in merge log).

2. **Policy-Based**
   - Rules such as "prefer branch with higher Aion" or "prefer lower entropy".
   - Configurable via merge options.

3. **Blend** (future)
   - For numeric components, allow interpolation (requires designer script).

4. **Retry**
   - Abort merge, spawn new branch to rework conflicts.

---

## Tooling Flow

- Merge UI displays conflict list with filters (type, component, branch).
- Each conflict shows diffs side-by-side, include context (timeline notes, metadata).
- Decisions appended to merge log (`MergeDecision[]`) for replay.
- After resolving all conflicts, system applies merged diff sequentially and commits new node.

---

## Automation Hooks

- `merge.resolve(conflictId, strategy)` API for scripting/automation.
- Optional "auto-resolve" pass using policy (e.g., prefer branch A) before manual review.
- Notifications when unresolved conflicts remain.

---

## Open Questions

- Should we support collaborative resolution (multiple designers editing simultaneously)?
- How to visualize conflicts across nested branches (merge of merges)?
- Do we need plugin points for domain-specific merge strategies (e.g., level geometry vs inventory)?
- How to integrate paradox detection (if merge would introduce paradox, block and prompt user).

