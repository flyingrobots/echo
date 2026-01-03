<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Echo, Explained Like You’re Not a Programmer

This is the gentlest on‑ramp into Echo.
It assumes you **don’t** know (or don’t want) programming concepts yet.

It’s written as a **spiral**:
we explain an idea simply, then we loop back later and explain the same idea with a bit more precision.

If you only read one page before clicking around the docs site, read this.

---

## A One‑Sentence Summary

Echo is a way to run a simulation so reliably that:

- you can **replay** it later and get the exact same result,
- you can **prove** what happened (and when),
- and multiple computers can **stay in sync** without “close enough” guesswork.

---

## Spiral Level 0: A Story (No Jargon)

Imagine a tabletop game with:

- **pieces** (characters, doors, items, UI widgets, planets, anything),
- **connections** (“is holding”, “is inside”, “is targeting”, “depends on”),
- and **rules** (“if X is true, do Y”).

Echo treats the world like a **relationship map**:
what exists, and how things relate.

Time moves forward in steps (“ticks”).
Each tick, Echo applies rules in a careful order so the outcome is consistent.

That’s it.

---

## Spiral Level 1: The Same Story, With a Few Concrete Words

### 1) What is “state”?

**State** means “everything that’s true right now”.

Echo models state as:

- a **graph**: a set of things and the connections between them, and
- **attachments**: the data that sits on those things (numbers, small payloads, tags).

You can think of the graph as the *shape of the world*, and attachments as the *details*.

### 2) What is a “tick”?

A **tick** is one step of time in the simulation.

On every tick:

1) Echo looks at the world.
2) It asks which rules apply.
3) It applies a set of changes.

### 3) What is a “rewrite rule”?

A **rewrite** is “change the world’s relationship map in a controlled way”.

Example (informal):

- If a character is holding a key and is near a locked door,
  the rule might rewrite the world so the door becomes unlocked.

This is why you’ll see the word “WARP” in Echo docs:
it’s the name of this graph‑rewrite style of simulation.

---

## Spiral Level 2: Why Echo Cares So Much About Determinism

Most simulations are *approximately the same* across machines.
Echo aims for **exactly the same**.

Echo cares because it unlocks things you can’t safely build otherwise:

- **Replays that don’t drift**: a replay that “mostly matches” is not a replay you can trust.
- **Debugging you can believe**: “why did this happen?” must have a stable answer.
- **Networking without vibes**: when machines disagree, you want a clean “desync” signal, not a mystery.
- **Verification**: tools/peers can validate history.

Echo is intentionally strict about sources of randomness and “it depends” behavior.

---

## Spiral Level 3: Hashes (A Simple Mental Model)

Echo uses **hashes** as a compact fingerprint of the world.

You don’t need to know cryptography to understand the role hashes play here:

- If two worlds are identical, their fingerprints match.
- If anything differs (even a tiny bit), the fingerprints diverge.

This is how tools can quickly answer:

- “Are we still in sync?”
- “Did something change?”
- “Which change caused the divergence?”

---

## Spiral Level 4: Two Planes (The “No Hidden Edges” Idea)

This is a core Echo idea, and it’s worth learning early.

Echo separates the world into:

- **Structure**: the relationship map (the graph)
- **Data**: the payload details (attachments)

The key rule is:

> If a piece of information affects what rules apply, or whether two rules conflict,
> it must be visible in the structure (or visible attachment identity), not buried inside opaque bytes.

Echo calls this the “no hidden edges” law.

If you want the formal version, see:
[/warp-two-plane-law](/warp-two-plane-law)

---

## Spiral Level 5: Where to Go Next (Pick Your Path)

### If you want the next gentle step (still low‑jargon)

- Start Here (recommended paths + how the docs are organized): [/guide/start-here](/guide/start-here)

### If you want the “core mental model” (still newcomer-friendly, but deeper)

- WARP primer: [/guide/warp-primer](/guide/warp-primer)

### If you want to run something concrete

- WARP View Protocol demo: [/guide/wvp-demo](/guide/wvp-demo)
- Collision tour: [/guide/collision-tour](/guide/collision-tour)

---

## A Note for Contributors

This page intentionally avoids implementation terms.
If you’re writing or editing docs, try to preserve this gradient:

- this page explains *why Echo exists* and *what it feels like*,
- the WARP primer explains the model more precisely,
- the specs define the contractual boundaries.
