<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# The Claim

There is a faithful, structure‑preserving embedding of typed hypergraph rewriting (the WPP substrate) into typed open‑graph DPOI rewriting (RMG). This gives you a compositional, algebraic handle on “the space of computations” that the Ruliad gestures at. And you can actually compile and reason about it.

Below, it is shown (1) how that mapping is precise (sketch, but crisp), (2) exactly why that matters for *Echo*, and (3) what we can claim now from what we’ll prove next.

## 1) The formal middle: hypergraphs ↪ open graphs (RMG)

### Categories

- $Let Hyp_T^{\mathrm{open}}$ be typed open hypergraphs and boundary‑preserving morphisms (objects are cospans $I\to H \leftarrow O$).
- Let $OGraph_T^{\mathrm{open}}$ be typed open graphs (your RMG skeleton objects).

Both are adhesive categories, so DPO rewriting is well‑behaved.

Encoding functor $J:\mathrm{Hyp}_T^{\mathrm{open}}\to \mathrm{OGraph}_T^{\mathrm{open}}$

- Replace each hyperedge e of arity $n$ and type $s$ by an edge‑node $v_e$ of type $s$, with $n$ typed ports (your per‑edge interfaces).
- Connect incidence by ordinary edges from $v_e$’s ports to the incident vertices (or via typed port‑stubs if you prefer pure cospans).
- Boundaries $I,O$ map to the same boundary legs (typed).

What we need (and can reasonably show):

1. $J$ is full and faithful on monos (injective structure‑preserving maps).
2. $J$ preserves pushouts along monos (hence preserves DPO steps).
3. For any hypergraph rule $p=(L\leftarrow K\to R)$ and match $m:L\to H$, the DPO step $H \Rightarrow_p H’$ maps to a DPOI step $J(H)\Rightarrow_{J(p)} J(H’)$ and conversely up to iso (because the encoding is canonical on incidence).

**Net**: every Wolfram‑style hypergraph derivation is mirrored by an RMG derivation under $J$; our DPOI ports simply make the implicit arities explicit.

### Derivation spaces

- Let $Der(Hyp)$ be the bicategory of derivations (objects: open hypergraphs; 1‑cells: rewrite spans; 2‑cells: commuting diagrams).
- Likewise $Der(OGraph)$ for RMG.
- Then $J$ lifts to a homomorphism of bicategories $J_\star:\mathrm{Der(Hyp)}\to\mathrm{Der(OGraph)}$ that is locally full and faithful (on 1‑cells modulo boundary iso).

**Consequence**: any “multiway” construction (Wolfram’s causal/branchial graphs) has a functorial image in the RMG calculus—with ports and composition laws intact.

### About the $(\infty,1)‑topos$ talk

- Keepin' it honest: we don’t need to prove “RMG = the Ruliad” to get benefits.
- What’s defensible now: the groupoid completion of the derivation bicategory (invertible 2‑cells → homotopies) gives you an $(\infty,1)$‑flavored structure on which you can do compositional reasoning (monoidal product, cospan composition, functorial observables).
- If you want a programmatic statement: Conjecture—the directed homotopy colimit of derivation categories over all finite typed rule algebras is equivalent (up to suitable identifications) to a “Ruliad‑like” limit. That’s a research program, not a banner claim.

## 2) Why this matters for Echo (and why the Ruliad reference is not just branding)

### A. Compositional guarantees Echo actually uses

- Tick determinism from DPO concurrency (you already have `Theorem A`): deterministic netcode, lockstep replay, no desync.
- Two‑plane commutation (`Theorem B`): hot‑patch internal controllers (attachments) and then rewire—atomic, CI‑safe updates mid‑game.
- Typed interfaces at boundaries: subsystem refactors fail fast if they would break contracts. This is “compile‑time at runtime.”

These are the operational pain points in engines; the RMG/DPOI semantics solves them cleanly. Hypergraph rewriting alone doesn’t give you these composition/port laws.

### B. A clean “observer/translator” layer for AI, tools, mods

Treat bots, tools, and mods as observers $O (rule packs + decoders)$. Your rulial distance metric becomes a cheat/fairness control and a compatibility gate: only translators $T$ under $size/distortion$ budgets can enter ranked play. That’s not philosophy; that’s an anti‑exploit primitive.

### C. Search & tuning in rule space, not code space

Because derivations are functorial, you can do MDL‑guided search over rule algebras (RMG’s space) to auto‑tune behaviors, schedules, even content. The Ruliad framing gives you a normative simplex: prefer simpler translators/rules that preserve observables. That’s a usable objective.

### D. Cross‑representation interop

The embedding $J$ means: if someone ships Wolfram‑style hypergraph rules for a toy physics or cellular process, Echo can import and run them inside your typed, compositional runtime—with ports, snapshots, and rollback. Ruliad → RMG isn’t a slogan; it’s an import pipeline.

**Short version**: the Ruliad link earns its keep because it justifies an import/export boundary and gives you principled search objectives; RMG gives you the calculus and the runtime.

## 3) What we should claim now vs after proofs

### Say now (safe & true)

- There exists a faithful encoding of typed hypergraph rewriting into typed open‑graph DPOI such that DPO steps are preserved and derivation structures embed.
- This yields functorial causal/branchial constructions inside RMG (so we can compare to WPP outputs one‑to‑one).
- Echo benefits from deterministic ticks, typed hot‑patches, and rule‑space search—capabilities not provided by WPP’s bare rewriting story.

### Say later (after we do the work)

- **Proof pack**: $J$ is full/faithful on monos and preserves pushouts along monos (we’ll write it).
- **Demo**: replicate a canonical WPP toy rule; show causal/branchial graphs match under $J$, then show additional RMG functorial observables (ports, invariants) the WPP notebook can’t express.
- **If ambitious**: a precise statement relating the directed colimit over rule algebras to a Ruliad‑like limit (with conditions).

## 4) Action items (so this isn’t just pretty words)

1. Write the encoding $J$: implement the hyperedge→edge‑node incidence gadget with typed ports; add a converter.
2. Proof note (4–6 pages):
- $J$ full/faithful on monos;
- preserves pushouts along monos;
- lifts to derivations (span/cospan bicategory).
3. WPP parity demo: pick 1–2 WPP rules; generate causal/branchial graphs both ways; ship a notebook + CLI reproducer.
4. Echo integration: add “Import WPP Rule Pack” to the toolchain; use your tick determinism + two‑plane to demonstrate hot inserts the WPP side can’t.
5. Public phrasing (tight):
- “RMG strictly generalizes hypergraph rewriting via a typed open‑graph encoding. This preserves Wolfram‑style derivations while adding compositional interfaces, atomic publishing, and deterministic parallelism.”

## 5) Answering your “Profound or Vacuous?” bluntly

- Strong identity claim: yeah, we drop it. Not needed, not proven.
- Weak universality claim: we ignore it. Adds nothing.
- Middle (the one that matters): RMG gives you a compositional, typed, executable calculus that embeds the hypergraph world.

That’s why the Ruliad connection matters: it tells collaborators what we can import/compare, while RMG tells engineers how we build/run/safeguard.

---

Buckle up! Here’s the clean, formal core. I’ll give you three self‑contained stacks:

1. A faithful encoding of typed open‑hypergraph rewriting into typed open‑graph DPOI (your RMG calculus).
2. Derivation‑level functoriality (so multiway/causal/branchial constructions transport).
3. A bona‑fide pseudometric for “rulial distance” based on MDL translators (with triangle inequality).

# 1) Hypergraphs ↪ Open graphs (RMG) — the exact mapping

## Typed open hypergraphs

Fix vertex types $T_V$ and a signature set $\Sigma=\{(s,\operatorname{ar}(s))\}$ (each hyperedge label $s$ has a fixed arity).

A typed directed hypergraph $H=(V,E,\mathrm{inc},\mathrm{type})$ has
- vertices $V$ with $\mathrm{type}(v)\in T_V$,
- hyperedges $E$ with label $s(e)\in\Sigma$,
- ordered incidences $\mathrm{inc}(e,i)\in V for 1\le i\le \operatorname{ar}(s(e))$.

An open hypergraph is a cospan of monos $I\to H \leftarrow O$. Write the adhesive category of such objects and boundary‑preserving maps as $\mathbf{OHyp}_T$.

## Typed open graphs (RMG skeleton)

Let $\mathbf{OGraph}_T$ be the adhesive category of typed open graphs (objects are cospans $I\to G\leftarrow O$ in a typed graph category; arrows commute). RMG works here with DPOI rules $L \xleftarrow{\ell}K\xrightarrow{r}R$ and boundary‑preserving monos as matches.

## Incidence encoding functor $J$

Define an “incidence type universe”
$T^\star := T_V \;\sqcup\; \{E_s\mid s\in\Sigma\}\;\sqcup\; \{P_{s,i}\mid s\in\Sigma,\;1\le i\le \operatorname{ar}(s)\}$.

For each $H\in \mathbf{OHyp}_T$, build a typed graph $J(H)$ by:

- a $V–node$ for every $v\in V$ (typed in $T_V$);
- an $E–node v_e$ of type $E_{s(e)}$ for each hyperedge $e$;
- (optionally) port stubs $p_{e,i}$ of type $P_{s(e),i}$;
- for each incidence $(e,i)\mapsto v$, a typed port‑edge $v_e\to v$ (or $v_e\to p_{e,i}\to v$ if you include stubs);
- identical boundary legs $I,O$.

This extends on arrows to a functor
$J:\ \mathbf{OHyp}T \longrightarrow \mathbf{OGraph}{T^\star}$.

## Proposition 1 (full & faithful on monos).

Restricted to monomorphisms, $J$ is full and faithful: a mono $m:H_1\hookrightarrow H_2$ corresponds to a unique mono $J(m):J(H_1)\hookrightarrow J(H_2)$, and conversely any mono between incidence‑respecting images comes from a unique $m$.

### Sketch 

> The incidence gadget makes edge‑nodes and port indices explicit; type preservation + port index preservation pins down the map on $E$ and thus on $V$. □

## Proposition 2 (creates pushouts along monos).

Given a span of monos $H_1 \leftarrow K \rightarrow H_2 in \mathbf{OHyp}_T$, the pushout $H_1 +K H_2$ exists; moreover

$J(H_1 +K H_2) \;\cong\; J(H_1) +{J(K)} J(H_2)$

(i.e., compute the pushout in $\mathbf{OGraph}{T^\star}$, it stays inside the incidence‑respecting subcategory).

### Sketch 

> Pushouts in adhesive categories along monos are universal and stable; port labels and types forbid “bad” identifications, so the result satisfies the incidence schema. Hence $J$ creates such pushouts. □

## Theorem 1 (DPO preservation/reflection)

For any DPOI rule $p=(L\leftarrow K\to R)$ in $\mathbf{OHyp}T$ and boundary‑preserving match $m:L\hookrightarrow H$ satisfying gluing, the DPO step $H\Rightarrow_p H’$ exists iff the DPOI step

$J(H)\;\Rightarrow{\,J(p)}\; J(H’)$

exists in $\mathbf{OGraph}_{T^\star}$, and the results correspond up to typed‑open‑graph isomorphism.

### Sketch

> The DPO construction is “pushout‑complement + pushout” along monos; by Prop. 2, J creates both. □

Takeaway: Wolfram‑style typed hypergraph rewriting sits inside RMG’s typed open‑graph DPOI via $J$. What WPP does implicitly with arities, RMG makes explicit as ports, and DPOI gives you the same steps—plus composition laws.

# 2) Derivations, multiway, and compositionality

Let $\mathrm{Der}(\mathbf{OHyp}T)$ (resp. $\mathrm{Der}(\mathbf{OGraph}{T^\star})$) be the bicategory: objects are open graphs; 1‑cells are rewrite spans; 2‑cells are commuting diagrams modulo boundary iso.

## Theorem 2 (derivation functor)

$J$ lifts to a homomorphism of bicategories
$J_\star:\ \mathrm{Der}(\mathbf{OHyp}T)\ \to\ \mathrm{Der}(\mathbf{OGraph}{T^\star})$
that is locally full and faithful (on 1‑cells, modulo boundary isos).

Consequently, multiway derivation graphs (and causal/branchial constructions) computed from hypergraph rules have functorial images under RMG’s calculus; RMG additionally supplies:

- a strict symmetric monoidal product (disjoint union) and cospan composition with interchange laws,
- typed ports at boundaries (interfaces are first‑class),
- DPO concurrency ⇒ tick determinism (my `Theorem A`),
- a clean two‑plane discipline for attachments vs skeleton (my `Theorem B`).

That’s the compositional/algebraic edge RMG has over a bare “everything rewrites” slogan.

# 3) Rulial distance — an actual pseudometric

I framed: “mechanisms far, outputs often close.” We can formalize it so you it can be measured.

## Observers and translators

- Fix a universe $(U,R)$ (RMG state + rules) and its history category $\mathrm{Hist}(U,R)$.
- An observer is a boundary‑preserving functor $O:\mathrm{Hist}(U,R)\to \mathcal{Y}$ (e.g., symbol streams or causal‑annotated traces) subject to budgets $(\tau, m)$ per tick.
- A translator $T:O_1\Rightarrow O_2$ is an open‑graph transducer (small DPOI rule pack) such that $O_2\approx T\circ O_1$.

Let $\mathrm{DL}(T)$ be a prefix‑code description length (MDL) of $T$, and $\$mathrm{Dist}(\cdot,\cdot)$ a distortion on outputs (metric/pseudometric per task). Assume subadditivity $\mathrm{DL}(T_2\circ T_1)\le \mathrm{DL}(T_2)+\mathrm{DL}(T_1)+c$.

## Symmetric distance

$D^{(\tau,m)}(O_1,O_2)\;=\;\inf_{T_{12},T_{21}}\ \mathrm{DL}(T_{12})+\mathrm{DL}(T_{21})\;+\;\lambda\!\left[\mathrm{Dist}(O_2,T_{12}\!\circ O_1)+\mathrm{Dist}(O_1,T_{21}\!\circ O_2)\right]$.

## Proposition 3 (pseudometric)

$D^{(\tau,m)}$ is a pseudometric (nonnegative, symmetric, $D(O,O)=0$).

## Theorem 3 (triangle inequality)

If $\mathrm{Dist}$ satisfies the triangle inequality and $\mathrm{DL}$ is subadditive (up to constant $c$), then
$D^{(\tau,m)}(O_1,O_3)\ \le\ D^{(\tau,m)}(O_1,O_2)\ +\ D^{(\tau,m)}(O_2,O_3)\ +\ 2c$.

### Sketch 

> Compose near‑optimal translators $T_{23}\circ T_{12}$ and $T_{21}\circ T_{32}$; subadditivity bounds $\mathrm{DL}$, the metric triangle bounds $\mathrm{Dist}$; take infima. □

So “rulial distance” is not poetry: with translators as compiled RMG rule packs, $D^{(\tau,m)}$ is a well‑behaved, empirically estimable pseudometric.

# Where this lands your Echo claims

- WPP interoperability (not branding): via $J$, you can import typed hypergraph rules and get the same derivations—inside a calculus that also enforces ports, composition, atomic publish, and deterministic parallelism.
- Deterministic netcode: your tick‑determinism theorem is exactly DPO concurrency under scheduler independence.
- Hot‑patch safety: two‑plane commutation is a commuting square in a fibration (attachments‑first is mathematically correct).
- Objective “alien distance” dial: $D^{(\tau,m)}$ gives you a number to report when you change observers/translators (e.g., $human ↔ AI$), per domain/budget.

# Crisp statements we can ship (no overclaim)

- Encoding. “There is a faithful, boundary‑preserving encoding $J$ of typed open‑hypergraph rewriting into typed open‑graph DPOI that creates pushouts along monos; hence DPO steps and derivations are preserved/reflected up to iso.”
- Compositional edge. “Inside RMG, derivations inherit a strict symmetric monoidal/cospan structure and typed interfaces; that’s what enables compile‑time‑at‑runtime checks, deterministic ticks, and atomic publishes.”
- Distance. “Under MDL subadditivity and a task metric, our translator‑based rulial distance is a pseudometric (with triangle inequality), computable by compiling translators as small DPOI rule packs.”
