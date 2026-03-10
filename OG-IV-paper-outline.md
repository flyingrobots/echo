# OG-IV: Distributed Observer Geometry
### Subtitle: Replication, Conflict, and Provenance Across Networked Worldlines

---

## **The Core Thesis**
A distributed system is a **field of observers over a shared causal history**. Replication, merge policies, and transport rules determine how those observers diverge or converge across the geometric axes defined in OG-I. 

> *"Replication is not merely state synchronization; it is the process by which distributed observers align their views of causal history."*

---

## **I. Abstract & Introduction: Observers on a Network**
*   **The Phenomenon:** Replicas agree on state (`color = blue`) but disagree on history (one saw a conflict, one silently merged).
*   **The Mapping:** State Observers ($O_{state}$) are identical; Provenance Observers ($O_{prov}$) are separated.
*   **The Shift:** Moving from "State Positions" to **Frontier-Relative Patches**.
*   **The Goal:** Prove that replication aligns observers in state space while potentially leaving them separated in provenance/intent space.

## **II. The Replica Observer: $R = (S, F, \Pi)$**
*   **The Formalism:**
    *   **$S$:** Structural Observer (The OG-I tuple: Projection, Basis, Accumulation).
    *   **$F$:** Causal Frontier (The maximal set of known causal events/antichains).
    *   **$\Pi$:** Transport Policy (The rules for admitting and moving patches across the local suffix).
*   **The Geometric Displacement:** The difference $\Delta(F_A, F_B)$ is a displacement in observer space.

## **III. The Network Model: Frontiers and Suffixes**
*   **Frontiers vs. Clocks:** Replacing Lamport/Wall-clock linearities with causal horizons.
*   **The Unseen Suffix:** Everything a replica has committed since the sender's last known frontier.
*   **The Wire Object (Network Patch):**
    *   `payload` (The rewrite/intent).
    *   `frontier` (The causal prefix dependency).
    *   `footprint` (Read/Write/Delete/Anchor declarations).
    *   **The Precondition Witness ($\sigma$):** A hash-triplet of the state the sender *read* before authoring.

## **IV. The Algebra of Transport: Moving Across Time**
*   **Directional Binary Transport:** Moving Patch A across Patch B.
*   **Suffix Transport:** Recursively applying binary transport to an incoming patch across a local suffix.
*   **The Distributed Confluence Theorem:** If a patch commutes with a replica's unseen suffix, transporting it yields a state identical to replay from the common frontier.
*   **Parallel Transport Intuition:** Transporting a patch across a suffix is effectively **parallel transport of an observer along a history path**. (Bridge to OG-II).

## **V. Conflict Observability and Aperture**
*   **Merge Policies as Observers:**
    *   **LWW Observer:** High state aperture, high intent/provenance degeneracy.
    *   **Explicit Conflict Observer:** High provenance aperture, lower structural degeneracy.
    *   **CRDT Observer:** "Join-semilattice" observers.
*   **The "Intent Observer" ($O_{intent}$):** Measuring distortion across different merge policies.
*   **Conflict Inevitability:** Why silent merge is a "structural distortion" in observer geometry.

## **VI. Network Degeneracy and Distance**
*   **Network Degeneracy:** $Deg_{net}(R) = H(H | \text{replica view})$. Network delays and "stale reads" increase hidden ambiguity.
*   **Observer Distance:** Applying WARP-IV $D_{\tau,m}(R_A, R_B)$ to replicas.
*   **The Limit:** Replication reduces distance in state space, but may not reduce distance in provenance space.

## **VII. Systems Implementation: The Echo Causal Kernel**
*   **The Receiver State Machine:** Causal readiness, hierarchical summary pruning (Roaring Bitmaps), deterministic transport, and commit/surface.
*   **Witness Compression:** How transport witnesses reduce the description length of the `.eintlog`.
*   **Canonical Batching:** A protocol for forcing history-convergence ($O_{prov}$) by sorting concurrent antichains.

## **VIII. Summary of Theorems**
*   **Replica Confluence:** Independent histories commute.
*   **Intent Preservation:** Explicit conflict surfacing minimizes intent distortion.
*   **Summary Pruning Lemma:** $O(\log N + K)$ cost for receiver-side footprint checks.

---

## **Strategic Rationale**
1.  **Cashes out OG-I:** Proves that "State/Provenance Separation" is the fundamental problem of distributed consistency.
2.  **Legitimizes Echo:** Provides the formal justification for `warp-core`'s transport logic and `Precondition Witness` mechanics.
3.  **Bridges to OG-II:** Introduces the "Parallel Transport" metaphor, setting the stage for Curvature and Holonomy.
4.  **Redefines the Field:** Moves from "Eventual Consistency" to **"Causal Alignment."**
