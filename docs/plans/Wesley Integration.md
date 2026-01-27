<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

Yeah, you’re _way_ off… but in a useful direction. **Don’t use a submodule.** Submodules are the “permanent foot-gun” of Git: easy to forget to update, easy to get into a detached-head mess, and they turn every clone into a tiny onboarding tax. You’re building a deterministic engine; don’t bolt nondeterministic repo UX onto it.

### **What I recommend instead (best → acceptable)**

#### **1)**

#### **Vendor Wesley outputs into Echo (recommended)**

Treat Wesley like protoc: **a generator**, not a dependency.

- Echo repo contains:
    - crates/ttd-protocol-rs/ (generated Rust)
    - packages/ttd-protocol-ts/ (generated TS)
    - crates/ttd-manifest/ (compiled manifests)
- Echo also contains a tiny provenance file:
    - docs/wesley/VERSION (or wesley.lock) that records:
        - Wesley repo URL
        - commit SHA
        - schema_hash
        - generator version

Then add one command in Echo:

- cargo xtask wesley:sync (or just wesley-sync)
    - runs Wesley (from wherever it lives)
    - copies outputs into Echo
    - writes wesley.lock
    - optionally verifies schema_hash matches what Echo expects

**Result:** Echo builds/tests without needing Wesley present. CI is clean. Contributors don’t need two repos unless they’re _changing the schema/compiler_.

#### **2)**

#### **If you really want “Wesley inside Echo” temporarily: use git subtree, not submodule**

Subtree gives you the “it lives in this repo” experience without the submodule pain.

- You can vendor Wesley under tools/wesley/
- Pull updates with git subtree pull …
- CI and clones “just work”

It’s not as pure as separate repos, but it’s dramatically less annoying than submodules.

#### **3)**

#### **Monorepo / workspace member (later, when Wesley is Rust)**

Once Wesley becomes a Rust crate:

- Put it in the Echo workspace
- Drive generation via xtask / build.rs (I’d still prefer xtask so builds don’t surprise-regenerate)

### **The rule of thumb**

If **Echo needs Wesley at runtime** → dependency.

If **Echo needs Wesley to produce artifacts** → generator.

Wesley is a generator. So **commit outputs + record provenance**.

### **Concrete “do this next”**

1. In Echo, create:

- crates/ttd-protocol-rs/
- packages/ttd-protocol-ts/
- crates/ttd-manifest/
- docs/wesley/wesley.lock

1. Add an Echo task:

- xtask wesley:sync that:
    - runs Wesley
    - writes outputs into those paths
    - writes wesley.lock containing the Wesley SHA + schema_hash

1. Add a CI check:

- “Generated outputs are up to date” (fails if git diff after regen)

That gives you tight integration, perfect reproducibility, and zero submodule misery. Submodules are for when you hate your future self.
