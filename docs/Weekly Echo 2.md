<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley Project - Exhaustive 8-Day Summary

Period: January 18-26, 2026
Commits: 26
Lines Changed: +8,020 / -968 (net +7,052)
Files Touched: 81

---

1. TTD Protocol Compiler (Major Feature)

What Was Built
┌────────────────────┬────────────────────────────┬───────┬──────────────────────────────────────────────────────────────────────────────────┐
│ Component │ Files │ Tests │ Description │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ Directive Parser │ directives.mjs │ 29 │ Parse 14 new @wes\__directives for channels, ops, rules, invariants │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ AST Model │ ast.mjs │ 20 │ TTD-specific abstract syntax tree nodes │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ Schema Extractor │ extractor.mjs │ 17 │ Extract channels/ops/rules from parsed GraphQL SDL │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ Schema Hasher │ hasher.mjs │ 22 │ Deterministic content-addressed hashing ("schema_hash is the universe identity") │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ Manifest Generator │ manifest.mjs │ 17 │ Generate schema.json, manifest.json, contracts.json │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ Validation Engine │ validation.mjs │ 42 │ Validate channels, ops, rules, invariants for consistency │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ Invariant Lexer │ invariants/lexer.mjs │ - │ Tokenize invariant expression language │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ Invariant Parser │ invariants/parser.mjs │ - │ Parse forall x in Y: expr quantified predicates │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ Bytecode Compiler │ invariants/golden.mjs │ - │ Compile expressions to verification bytecode │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ VM Runtime │ invariants/vm.mjs │ - │ Execute bytecode to verify state invariants │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ Verifier Generator │ invariants/verifier.mjs │ - │ Generate TypeScript verification code │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ Obligations Spec │ invariants/obligations.mjs │ - │ Compile obligation specifications │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ TypeScript Codegen │ codegen/ts-_.mjs │ 28 │ Generate types, Zod validators, registries │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ CLI Command │ compile-ttd │ - │ wesley compile-ttd schema.graphql --out-dir ttd-out/ │
├────────────────────┼────────────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────┤
│ Integration Tests │ test/integration/\*.mjs │ 43 │ E2E pipeline, determinism, rejection tests │
└────────────────────┴────────────────────────────┴───────┴──────────────────────────────────────────────────────────────────────────────────┘
Total: 321 tests passing across 11 test files

Why This Matters

1. Enables Echo Time Travel Debugger - Wesley now produces the protocol definitions that Echo needs to prove determinism in distributed systems
2. Content-Addressed Protocols - Every protocol version gets a unique SHA-256 hash. Two systems with different hashes are incompatible by design. This catches schema drift before it causes
   runtime failures.
3. Determinism Guarantees - Same SDL input always produces identical output bytes. Critical for reproducible builds and distributed system coordination.
4. Formal Verification Foundation - The invariant compiler and VM enable runtime verification of state invariants:
   @wes_invariant(
   name: "value_bounded",
   expr: "forall c in Counter: c.value >= 0 && c.value <= 1000000"
   )
5. Plugin Architecture - Rust codegen intentionally delegated to Echo repo, keeping Wesley focused on SDL->IR transformation

---

1. Holmes Security Hardening

What Was Fixed
┌───────────────────────────────┬───────────────────────────────────────────────────────────────────────────┬────────────────────────────────────────────────────────────────────────────┐
│ Fix │ Risk Mitigated │ Impact │
├───────────────────────────────┼───────────────────────────────────────────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────┤
│ Command injection prevention │ Malicious ref names could execute arbitrary shell commands │ Added isValidGitRef() validator + switched to execFileSync with args array │
├───────────────────────────────┼───────────────────────────────────────────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────┤
│ Git failure vs. zero activity │ Git errors silently reported as "0 commits", misleading plateau detection │ git.log() now returns null on error vs '' for genuine no-activity │
├───────────────────────────────┼───────────────────────────────────────────────────────────────────────────┼────────────────────────────────────────────────────────────────────────────┤
│ Test determinism │ Flaky tests due to git activity in test environment │ Disabled git activity during CLI tests │
└───────────────────────────────┴───────────────────────────────────────────────────────────────────────────┴────────────────────────────────────────────────────────────────────────────┘
Why This Matters

1. Security - The git port now validates all ref inputs against a strict allowlist before execution. No shell interpolation possible.
2. Reliability - Holmes reports now accurately distinguish between "git unavailable" and "no commits in time window". This prevents false plateau detection when git is misconfigured.
3. CI Stability - Tests no longer depend on actual git state, making CI runs reproducible.

---

1. Documentation & Planning
   ┌────────────────────────────────────────┬─────────────────────────────────────────────────┐
   │ Document │ Purpose │
   ├────────────────────────────────────────┼─────────────────────────────────────────────────┤
   │ docs/features/ttd-protocol-compiler.md │ User-facing feature documentation with examples │
   ├────────────────────────────────────────┼─────────────────────────────────────────────────┤
   │ docs/plans/ttd-protocol-compiler.md │ Implementation plan with phase breakdown │
   ├────────────────────────────────────────┼─────────────────────────────────────────────────┤
   │ meta/progress.json │ Updated package status matrix │
   └────────────────────────────────────────┴─────────────────────────────────────────────────┘

---

Current Project Status

Package Health
┌───────────────────────┬───────┬─────────────┬────────┬───────────────────────────┐
│ Package │ Stage │ Progress │ Tests │ Notes │
├───────────────────────┼───────┼─────────────┼────────┼───────────────────────────┤
│ @wesley/core │ MVP │ 0% -> Alpha │ ✅ │ TTD module added │
├───────────────────────┼───────┼─────────────┼────────┼───────────────────────────┤
│ @wesley/cli │ MVP │ 20% -> Alpha │ ✅ │ compile-ttd command added │
├───────────────────────┼───────┼─────────────┼────────┼───────────────────────────┤
│ @wesley/generator-ttd │ MVP │ NEW │ 321 ✅ │ Created this week │
├───────────────────────┼───────┼─────────────┼────────┼───────────────────────────┤
│ @wesley/holmes │ MVP │ 20% -> Alpha │ 31 ✅ │ Security hardened │
└───────────────────────┴───────┴─────────────┴────────┴───────────────────────────┘
Test Coverage
┌───────────────────────────┬───────┐
│ Area │ Tests │
├───────────────────────────┼───────┤
│ TTD Protocol Compiler │ 321 │
├───────────────────────────┼───────┤
│ Holmes │ 31 │
├───────────────────────────┼───────┤
│ Core (existing) │ ~400+ │
├───────────────────────────┼───────┤
│ Total new tests this week │ ~350 │
└───────────────────────────┴───────┘
PR Status
┌──────────┬───────────┬───────────────────────────────────┐
│ PR │ Status │ Description │
├──────────┼───────────┼───────────────────────────────────┤
│ #368 │ ✅ Merged │ TTD Protocol Compiler - Phase 1 │
├──────────┼───────────┼───────────────────────────────────┤
│ #369 │ ✅ Merged │ Holmes CLI test determinism │
├──────────┼───────────┼───────────────────────────────────┤
│ #375 │ 🟡 Open │ Holmes git adapter security fixes │
├──────────┼───────────┼───────────────────────────────────┤
│ #370-374 │ 🟡 Open │ Dependabot dependency updates │
└──────────┴───────────┴───────────────────────────────────┘

---

Next 7 Days - Expected Work

Immediate (This Week)
┌───────────┬──────────────────────────────────┬─────────────────────┐
│ Priority │ Task │ Est. Effort │
├───────────┼──────────────────────────────────┼─────────────────────┤
│ 🔴 High │ Merge PR #375 (Holmes fixes) │ Done pending review │
├───────────┼──────────────────────────────────┼─────────────────────┤
│ 🔴 High │ Echo integration validation │ 1-2 days │
├───────────┼──────────────────────────────────┼─────────────────────┤
│ 🟡 Medium │ Review/merge Dependabot PRs │ 1 hour │
├───────────┼──────────────────────────────────┼─────────────────────┤
│ 🟡 Medium │ Cross-repo TTD integration tests │ 1 day │
└───────────┴──────────────────────────────────┴─────────────────────┘
In-Flight (Roadmap Items)
┌───────────────────┬────────────┬─────────────────────────────────────────┐
│ Area │ Issue(s) │ Description │
├───────────────────┼────────────┼─────────────────────────────────────────┤
│ QIR Phase C │ #159, #160 │ --ops wiring, op->QIR translator │
├───────────────────┼────────────┼─────────────────────────────────────────┤
│ RLS Phase 2 │ #116 │ Extended policies, pgTAP coverage │
├───────────────────┼────────────┼─────────────────────────────────────────┤
│ Evidence & HOLMES │ #183, #184 │ Score refinements, schema hash emission │
└───────────────────┴────────────┴─────────────────────────────────────────┘
Backlog Candidates
┌───────┬───────────────────────────────────────┬────────┐
│ Issue │ Description │ Effort │
├───────┼───────────────────────────────────────┼────────┤
│ #315 │ Function SECURITY and SET search_path │ 1-2h │
├───────┼───────────────────────────────────────┼────────┤
│ #314 │ Emit operation registry.json │ 1-2h │
├───────┼───────────────────────────────────────┼────────┤
│ #311 │ Add ops coverage to evidence/scores │ 2-3h │
├───────┼───────────────────────────────────────┼────────┤
│ #353 │ HOLMES risk vectors for drops/renames │ 2-3h │
└───────┴───────────────────────────────────────┴────────┘

---

Summary

This was a major feature week. The TTD Protocol Compiler represents significant new capability - 7,000+ net lines of code, 321 new tests, and a complete SDL->protocol compilation pipeline.
Combined with Holmes security hardening, the project is materially more capable and robust than 8 days ago.

Next week will focus on integration validation (making sure Echo can actually consume the output) and continuing the roadmap items (QIR, RLS, Evidence).

# Flyingrobots.dev Update (Part 1)

I see. The shiplog.md and GIT-MIND-REPORT.md are documentation about other projects in the "inspo" folder. Let me check if TTD is perhaps a typo or acronym I should clarify with the user.
But first, let me see if there are any planning documents or if there's context about the project plan that might mention TTD.

Let me also look at the actual website components and structure to understand what has been built.

⏺ I couldn't find "TTD" in the codebase. Let me check the Home.vue and understand the current project state, then ask you to clarify what TTD refers to.

⏺ Read(/Users/james/git/james-website/src/views/Home.vue)
⎿  Read 426 lines
⎿  PostToolUse:Callback hook succeeded: Success

⏺ Read(/Users/james/git/james-website/src/router/index.ts)
⎿  Read 46 lines
⎿  PostToolUse:Callback hook succeeded: Success

∴ Thinking…

Now I have a good understanding of the project. It's a Vue-based personal website with sophisticated GSAP animations, 3D effects, and multiple sections. Let me see what the latest commit
today is about (clay shaders and audio-synced navigation) to understand the current state.

I still don't know what "TTD" means. It's not in the codebase. It could be:

- Time To Deploy
- Test-Driven Development
- Technical Design Document
- Something else project-specific

Let me ask the user what TTD refers to, but first provide a summary of what I've found in the git log.

⏺ Based on my review of the git log, here's a summary of the past 8 days of development:

---

Summary: January 18-26, 2026

What Was Accomplished

Phase 1: Site Architecture (Jan 21-22)

- Restructured navigation to dedicated routes (/james, /resume, /aion, /blog)
- Implemented an advanced floating terminal system with GSAP Flip, MorphSVG, command history, and Zod parsing
- Added the "BIG FLYINGROBOTS" conditional wordmark for non-hero pages
- Improved nav/footer coordination and site structure

Phase 2: Continuum Render System (Jan 24)

- Built a sophisticated 5-phase render pipeline including:
    - Scene Color Grab Pass with perfect refraction sync
    - Centralized post-processing
    - SSR safety measures
    - Triangle-level wireframes on the AION 3D logo
    - Polish pass for cinematic smoothness
- Implemented the "WORLD CLASS" input system and hero section enhancements

Phase 3: Homepage Portal Refactor (Jan 24)

- Migrated homepage content to /aion page
- Created a pure CSS/SVG portal navigation system
- Resolved numerous visibility, contrast, and interaction bugs
- Added glassmorphism cards, high-contrast pillar cards, and "Director's Commentary" copy

Phase 4: Audio & Interaction (Jan 24-26)

- Added 29 arcade UI sound effects for interactive feedback
- Implemented high-fidelity clay shaders
- Created audio-synced menu navigation with per-letter draw/fill animations and bounce trails

Benefits

- The site now has a polished, professional portfolio presence with sophisticated 3D graphics and microinteractions
- Clear information architecture with dedicated sections for personal info, resume, AION project, and blog
- High production value with GSAP-powered animations, sound design, and glassmorphism aesthetics

What to Expect Next 7 Days

Based on current trajectory: likely refinement of the /test scene (TestScene.vue), completion of the /simple theme variations, and possibly blog content population.

---

Regarding "TTD": I couldn't find any reference to "TTD" in the codebase or documentation. Could you clarify what TTD refers to? (e.g., Time To Deploy, Test-Driven Development, Technical
Design Document, or a project-specific feature?)

✻ Churned for 59s

❯ Time Travel Debugger

∴ Thinking…

The user is asking about a "Time Travel Debugger" (TTD). Let me search the codebase for anything related to time travel debugging, debugger, or similar concepts. This could be a feature
they're planning to build or have started working on.

⏺ Let me search for Time Travel Debugger related code and references.

⏺ Search(pattern: "time.?travel|debugger|replay|snapshot|state.?history")
  ⎿  Found 25 files
src/components/InteractiveSimulation.vue
src/views/Aion.vue
src/views/Simple.vue
public/images/warp.svg
src/content/home/profile.md
src/posts/what-makes-echo-tick.md
src/posts/git-stunts-1.md
src/posts/aion-foundations-series-intro.md
src/posts/aion-foundations-series-conclusion.md
src/posts/praxis.md
inspo/aion.plan.md
src/case_studies/gala_games.md
pnpm-lock.yaml
inspo/wesley.md
inspo/GIT-MIND-REPORT.md
inspo/what_is_echo.md
inspo/james-ross-staff-engineer-games.md
inspo/james-ross-staff-engineer-universal.md
inspo/gala-analytics-team-reality-check.md
inspo/time-replayable-pipeline-masterpiece.md
inspo/Ember Entertainment Brain Dump.md
inspo/ember-data-collection-system.md
inspo/ember-documentation-complete-summary.md
inspo/smile-break-brain-dump.md
inspo/Gala Role Brain Dump.md
⎿  PostToolUse:Callback hook succeeded: Success

⏺ Search(pattern: "\**/*debug\*")
  ⎿  Found 9 files
/Users/james/git/james-website/node_modules/.pnpm/@dimforge+rapier3d-compat@0.12.0/node_modules/@dimforge/rapier3d-compat/pipeline/debug_render_pipeline.d.ts
/Users/james/git/james-website/node_modules/.pnpm/@xterm+xterm@6.0.0/node_modules/@xterm/xterm/src/vs/base/common/observableInternal/debugName.ts
/Users/james/git/james-website/node_modules/@xterm/xterm/src/vs/base/common/observableInternal/debugName.ts
/Users/james/git/james-website/node_modules/source-map/dist/source-map.debug.js
/Users/james/git/james-website/node_modules/dagre-d3-es/src/dagre/debug.js
/Users/james/git/james-website/node_modules/dagre-d3-es/src/dagre/debug.d.ts
/Users/james/git/james-website/src/debugState.ts
/Users/james/git/james-website/node_modules/@types/three/node_modules/@dimforge/rapier3d-compat/pipeline/debug_render_pipeline.d.ts
/Users/james/git/james-website/node_modules/@dimforge/rapier3d-compat/pipeline/debug_render_pipeline.d.ts
⎿  PostToolUse:Callback hook succeeded: Success

⏺ Search(pattern: "\**/*replay\*")
  ⎿  Found 1 file
/Users/james/git/james-website/inspo/time-replayable-pipeline-masterpiece.md
⎿  PostToolUse:Callback hook succeeded: Success

∴ Thinking…

Let me check the most relevant files - the aion.plan.md (which likely has the roadmap), the debugState.ts, and the time-replayable-pipeline-masterpiece.md to understand what the Time Travel
Debugger is about.

⏺ Read(/Users/james/git/james-website/inspo/aion.plan.md)
⎿  Read 129 lines
⎿  PostToolUse:Callback hook succeeded: Success

⏺ Read(/Users/james/git/james-website/src/debugState.ts)
⎿  Read 22 lines
⎿  PostToolUse:Callback hook succeeded: Success

⏺ Read(/Users/james/git/james-website/inspo/time-replayable-pipeline-masterpiece.md)
⎿  Read 201 lines
⎿  PostToolUse:Callback hook succeeded: Success

∴ Thinking…

Now I understand. The "Time Travel Debugger" (TTD) is a core feature of the AION/Echo project - it's described in the aion.plan.md as the "Chronoscope" - an interactive component that
allows users to scrub through time and see server state history. This is a key differentiating feature of the Echo runtime.

Let me look at what's been implemented so far in the Aion.vue and related files to understand the current state of the TTD implementation.

⏺ Now I understand - the Time Travel Debugger is the Chronoscope feature in AION/Echo. Let me check the current implementation state.

⏺ Read(/Users/james/git/james-website/src/views/Aion.vue)
⎿  Read 553 lines
⎿  PostToolUse:Callback hook succeeded: Success

∴ Thinking…

Now I need to check the InteractiveSimulation component to see the current state of the Time Travel Debugger (Chronoscope) implementation.

⏺ Read(/Users/james/git/james-website/src/components/InteractiveSimulation.vue)
⎿  Read 302 lines
⎿  PostToolUse:Callback hook succeeded: Success

∴ Thinking…

Now I have a complete picture. The Time Travel Debugger (TTD) / Chronoscope is partially implemented in the InteractiveSimulation.vue component. Let me summarize the current state.

⏺ Now I have the full picture. Here's the updated summary:

---

Summary: January 18-26, 2026

What Was Accomplished

Phase 1: Site Architecture (Jan 21-22)

- Restructured navigation with dedicated routes (/james, /resume, /aion, /blog, /simple, /test)
- Advanced floating terminal with GSAP Flip, MorphSVG, command history, and Zod parsing
- "BIG FLYINGROBOTS" wordmark and site structure finalization

Phase 2: Continuum Render System (Jan 24)

- 5-phase render pipeline with Scene Color Grab Pass, refraction sync, SSR safety
- High-fidelity clay shaders with vertex displacement on icosahedron geometry
- AION 3D logo with wireframes and rotation constraints

Phase 3: Homepage Portal & Polish (Jan 24-26)

- Migrated homepage to a 2x2 card grid with full-bleed images and 3D tilt effects
- Per-letter SVG draw/fill animation with bounce trails on FLYINGROBOTS wordmark
- Audio-synced navigation with 29 arcade UI sound effects

Benefits

- Professional portfolio with sophisticated 3D graphics and microinteractions
- Clear information architecture with AION as a dedicated showcase page
- High production value animations and sound design

---

Time Travel Debugger (TTD) Status

The Chronoscope (TTD) is ~40% complete as a working proof-of-concept in InteractiveSimulation.vue:

What's Implemented:
┌───────────────────────────────────────────────────────────┬─────────┐
│ Feature │ Status │
├───────────────────────────────────────────────────────────┼─────────┤
│ Rapier physics world with ball + ground │ ✅ Done │
├───────────────────────────────────────────────────────────┼─────────┤
│ State recording (600 frames / 10 seconds) │ ✅ Done │
├───────────────────────────────────────────────────────────┼─────────┤
│ Clay shader material with vertex displacement │ ✅ Done │
├───────────────────────────────────────────────────────────┼─────────┤
│ Mode state machine (idle -> dropped -> timetravel -> forked) │ ✅ Done │
├───────────────────────────────────────────────────────────┼─────────┤
│ Timeline scrubber with visual replay │ ✅ Done │
├───────────────────────────────────────────────────────────┼─────────┤
│ Fork timeline (bifurcate with new gravity) │ ✅ Done │
├───────────────────────────────────────────────────────────┼─────────┤
│ Reset functionality │ ✅ Done │
└───────────────────────────────────────────────────────────┴─────────┘
What's Missing for Full TTD:
┌────────────────────────────────────────────┬────────────────┐
│ Feature │ Status │
├────────────────────────────────────────────┼────────────────┤
│ Multi-object state recording │ ❌ Not started │
├────────────────────────────────────────────┼────────────────┤
│ Event causality graph visualization │ ❌ Not started │
├────────────────────────────────────────────┼────────────────┤
│ Diff view between timelines │ ❌ Not started │
├────────────────────────────────────────────┼────────────────┤
│ Persistent worldline export/import │ ❌ Not started │
├────────────────────────────────────────────┼────────────────┤
│ Production crash -> worldline download flow │ ❌ Not started │
├────────────────────────────────────────────┼────────────────┤
│ Network sync demonstration │ ❌ Not started │
└────────────────────────────────────────────┴────────────────┘

---

What to Expect Over the Next 7 Days

Based on current momentum and the aion.plan.md roadmap:

1. TTD Enhancement - Likely focus on multi-object state capture and causality visualization
2. TestScene.vue - The /test route suggests shader/graphics experimentation continuing
3. Simple Theme - The /simple page with pallete.png indicates theme variant work
4. Blog Content - The blog infrastructure is ready but content population may follow

The TTD is a strong proof-of-concept demo. The core "drop ball -> time travel -> fork" loop works. The next major milestone would be adding causal graph visualization and multi-object support
to match the "Worldline" debugging narrative in the AION copy.

# Flyingrobots.dev Update (Part 2)

---

Summary: The Past 8 Days

54 commits with ~15,400 insertions and ~6,000 deletions across 149 files - an intense period of development.

Major Accomplishments

1. "Continuum Render System" (Jan 24)
   A custom WebGL rendering pipeline was implemented from scratch:

- Scene Color Grab Pass for real-time refraction
- Post-processing architecture
- SSR (server-side rendering) safety
- High-fidelity clay shaders

1. Portal Homepage Redesign (Jan 24)
   The homepage was completely reimagined:

- Migrated original content to a dedicated /aion page
- New "Portal navigation" paradigm with SVG/CSS
- Glassmorphism card-based layouts
- Custom cursor and interactive 3D AION logo with rotation constraints

1. Advanced Terminal System (Jan 22)

- GSAP Flip and MorphSVG animations
- Command history with Zod parsing
- Floating terminal UX with resize handling

1. Navigation & Routing Overhaul (Jan 22)

- Restructured to dedicated paths for all sections
- Added /james route
- Conditional "BIG FLYINGROBOTS" wordmark
- Nav/footer sync and overlap prevention

1. Audio-Synced Menu Navigation (Jan 26 - today)

- Harp glissando sounds for UI interactions
- Arcade-style UI sound effects library (25+ sounds)
- Audio-visual synchronization framework

---

How This Has Been Beneficial

- Professional polish: The clay shaders, post-processing, and glassmorphism give a premium, distinctive feel
- Better UX architecture: Separating content (/aion) from entry portal (homepage) creates a cleaner user journey
- Interactivity: Custom cursor, 3D logo, and audio feedback make the site feel "alive"
- Technical foundation: The ShaderManager (~2,200 lines) and RenderSystem provide reusable infrastructure for future effects

---

Current Status

The branch is called simple but the work is anything but - you're deep in experimental territory. The most recent commit is literally titled "WIP getting crazy with audio."

Stability: The Jan 24 commits show a pattern of stabilization work (7 "fix:" commits) after the big features landed. The portal homepage went through several iterations before settling.

Completion level: I'd estimate you're about 70-80% through a major visual/UX overhaul. The core systems are in place, but the audio integration is fresh and likely needs polish.

---

What to Expect Over the Next 7 Days

Based on the trajectory:

1. Audio system completion - The sound library is loaded but likely needs tuning (volume levels, trigger timing, mobile considerations)
2. Stabilization pass - The pattern suggests a round of bug fixes after experimental features
3. Content integration - The clay shader backgrounds and generated images in /public/baked-dreams/ suggest more visual assets incoming
4. Mobile/responsive work - Not much evidence of responsive fixes in the log; this often comes after desktop UX is locked
5. Merge preparation - If targeting production, expect lint/build validation commits

The foundation is strong. The next week is likely about refinement and ensuring all the experimental pieces work cohesively together.

# Echo Update

    8-Day Git History Report (Jan 18-26, 2026)

Executive Summary

Two major PRs were merged, plus ongoing work on a third feature branch:
PR: #260
Title: SPEC-0004: Worldlines, Playback, TruthBus + BOAW Phase 6B
Merged: Jan 24
Commits: 30
+/- Lines: +11,216 / -513
────────────────────────────────────────
PR: #261
Title: Footprint Enforcement Guard (Phase 6B)
Merged: Jan 26
Commits: 61
+/- Lines: +7,421 / -2,294
────────────────────────────────────────
PR: —
Title: TTD (Time-Travel Debugging)
Merged: in progress
Commits: ~15
+/- Lines: on ttd-spec branch

---

1. PR #260: SPEC-0004 + BOAW Phase 6B Parallel Execution

What: Core infrastructure for deterministic parallel execution and worldline/playback
architecture.

Why: Enable cross-warp parallelism and establish the foundation for time-travel debugging
(worldlines track history, playback cursors navigate it).

Key Components:

- WorldlineId — unique identifier derived from initial state hash
- PlaybackCursor — materialized viewpoint in history
- ViewSession — client subscription binding
- TruthSink — cursor-stamped frame delivery
- ProvenanceStore trait — hexagonal port for history storage
- Global work queue with stride fallback removal

Commits (sample):
c04daa4 Merge pull request #260 (Jan 24)
Branch graph-boaw was deleted after merge.

Closed Issues:

- #259: SPEC-0004: Worldlines, Playback, TruthBus implementation
- #258: BOAW Phase 6B: Parallel Execution Engine Integration

---

1. PR #261: Footprint Enforcement Guard

What: Runtime validation ensuring all WarpOps stay within declared read/write footprints
during parallel execution.

Why: Prevent data races and ensure deterministic execution. Rules must declare what they
read/write; the guard catches violations before execution.

Key Components:

- FootprintGuard — runtime enforcement struct
- FootprintViolation / FootprintViolationWithPanic — typed panic payloads
- Complete footprint declarations for all rules
- Slice theorem proof tests (determinism regardless of worker count)
- DeleteNode cascade removal (must explicitly delete edges first)

Commits (your session today):
95ffd82 fix(snapshot_accum): add parent/root validation to apply_open_portal
47e52a2 docs(footprint): document FootprintViolationWithPanic in module docs
249f770 fix(review): update OpTargets doc, cfg-gate checks, fix UpsertWarpInstance
5f66ec8 fix(review): add cross-warp detection, track_caller, unconditional check
6a897a4 fix(review): address code review feedback
6cf2a78 fix(review): address code review feedback

Earlier commits on this branch:
f57c5af fix(graph): DeleteNode must not cascade edges
4ecf8c8 feat(warp-core): poison deltas and guard panics
001e140 fix(warp-core): scope guard metadata per warp

Branch footprint-enforcement was deleted after merge.

---

1. Active Work: TTD (Time-Travel Debugging) — ttd-spec branch

What: Browser-based time-travel debugger with WASM bindings.

Why: Enable visual debugging of Echo simulations with rewind/replay.

Recent commits (not yet merged):
33835bb feat(ttd-app): scaffold React UI for TTD browser app (Task 6.1)
e63de35 feat(ttd-browser): add WASM TtdEngine for time-travel debugging (Task 5.4)
4d34dd4 feat(warp-wasm): add TTD WASM bindings (Task 5.3)
43803e3 feat(echo-ttd): add compliance engine with channel policy checks (Task 3.2)
465c8eb feat(ttd): add echo-ttd-gen crate for TTD IR code generation

---

Commit Activity by Day
┌───────────┬─────────┬──────────────────────────────────────────┐
│ Date │ Commits │ Notes │
├───────────┼─────────┼──────────────────────────────────────────┤
│ Jan 18 │ ~5 │ Initial PR #260 work │
├───────────┼─────────┼──────────────────────────────────────────┤
│ Jan 19 │ ~20 │ Heavy PR #260 development │
├───────────┼─────────┼──────────────────────────────────────────┤
│ Jan 20 │ ~40 │ PR #260 finalization, Issue #258 closed │
├───────────┼─────────┼──────────────────────────────────────────┤
│ Jan 22-23 │ ~20 │ PR #261 footprint guard development │
├───────────┼─────────┼──────────────────────────────────────────┤
│ Jan 24 │ ~3 │ PR #260 merged, Issue #259 closed │
├───────────┼─────────┼──────────────────────────────────────────┤
│ Jan 25 │ ~55 │ Heavy PR #261 review feedback + TTD work │
├───────────┼─────────┼──────────────────────────────────────────┤
│ Jan 26 │ ~2 │ PR #261 merged (your session) │
└───────────┴─────────┴──────────────────────────────────────────┘

---

Branch Topology

main ─────────────────────────────────────────────────────► 19f3236
\ /
└─ graph-boaw (#260) ────────────────────────┘ (merged Jan 24)
\ /
└─ footprint-enforcement (#261) ┘ (merged Jan 26)

ttd-spec ──────────────────────────────────────────────────► 33835bb (in progress)

✻ Crunched for 1m 36s

❯ What id you look back 12 days ?

∴ Thinking…

The user wants me to look back 12 days instead of 8 days to get a more complete picture.

⏺ Bash(git log --all --oneline --since="12 days ago" --date-order --format="%h %ad %s"
--date=short | tail -80)
⎿ eead915 2026-01-17 fix(hooks): address code review feedback for pre-push and scripts
a927976 2026-01-17 fix(hooks): only log timing when cargo actually compiles
94a57e3 2026-01-17 fix(hooks): only log timing when Rust files change
0c18df9 2026-01-17 fix(plot): use median instead of average for timing stats
d319a8e 2026-01-17 feat(hooks): add round-robin pre-push benchmark with chart
dadd42b 2026-01-17 fix(sweep): only clean incremental/, leave deps/ alone
1d0c1e4 2026-01-17 feat(hooks): add parallel pre-push and stale artifact sweeping
09e8212 2026-01-17 fix(wsc): use monotonic index for attachment validation errors
cfb8270 2026-01-17 fix(wsc): compare out_edge indices as u64 before usize cast
096f973 2026-01-17 fix(dind-tests): re-export SCHEMA_HASH for harness access
24cdaec 2026-01-17 test(wsc): add negative validation tests
02ce3bd 2026-01-17 fix(wsc): return error on non-zero reserved bytes
de73e01 2026-01-17 fix(dind-tests): make generated modules private to avoid
missing_docs
34ea946 2026-01-17 fix(wsc): return error on node/edge ordering violations
2117c7e 2026-01-17 fix(testdata): regenerate math_determinism fixture with unique
hashes
5d5d396 2026-01-17 fix(wsc): return error on node/edge ordering violations
fe4c352 2026-01-17 fix(testdata): update hash_domain to V2 in fixtures
a7a4bf2 2026-01-17 fix(wsc): use read_slice in warp_view to prevent panics
6d9acca 2026-01-17 refactor(wsc): simplify write_padding with resize
2ac510d 2026-01-17 docs: fix markdown lint issues in WARP-GRAPH.md
3d83673 2026-01-17 fix(dind-tests): update bootstrap scripts for new generated file
paths
d7607d4 2026-01-17 fix(wsc): use read_slice in warp_view to prevent panics
c9dca64 2026-01-17 fix(hooks): correct regex pattern in pre-commit hook
6058f75 2026-01-17 fix: Reject WSC files with out-of-bounds index ranges
a41aa55 2026-01-17 docs: Update CHANGELOG, README and docs-index for WSC
d5f9b75 2026-01-17 feat: Add WSC (Write-Streaming Columnar) snapshot format
0c84a42 2026-01-16 docs: Add WARP-GRAPH.md
cb68e41 2026-01-16 Merge branch 'main' into generic-atoms
c8e69db 2026-01-16 Add codec toolkit and fixed-point helpers
a42c377 2026-01-16 Merge pull request #252 from flyingrobots/generic-atoms
1d72b77 2026-01-16 Merge branch 'main' into generic-atoms
b06952f 2026-01-16 Fix rustdoc Hash links
7673012 2026-01-16 Fix tests and remove mac CI
c8eeb0c 2026-01-16 Fix determinism guards and docs updates
994a787 2026-01-16 Harden determinism checks and math policies
4a9c9d1 2026-01-16 docs: add meta folder and update links
0be33d9 2026-01-16 docs: add visual topic map to landing page
8053525 2026-01-16 docs: trim course index to existing modules
c5e9aa7 2026-01-16 docs: consolidate stubs and add redirects
2f70f30 2026-01-16 docs: add background links to specs
60f1b4c 2026-01-16 docs: add status badges to methodology
484327a 2026-01-16 refactor: remove globals from warp-core and warp-wasm
16f285a 2026-01-16 Merge pull request #251 from flyingrobots/generic-atoms
ae72678 2026-01-16 ci: install ripgrep for determinism guards
fda0e1c 2026-01-16 fix: remove unresolvable serde_json paths from clippy.toml
f79020d 2026-01-16 fix: remove append-only CI check and fix clippy errors
0e2229d 2026-01-16 fix: improve test coverage, docs, and SPDX compliance
e2394e6 2026-01-16 fix: address PR review feedback and purge stale doc references
3169514 2026-01-16 fix: address additional PR review feedback
282a113 2026-01-16 fix: resolve remaining PR #251 review feedback
8bc9210 2026-01-16 fix: resolve all clippy warnings for strict CI lint pass
ecd00f3 2026-01-16 Remove decision-log.md and all references
ec39c51 2026-01-16 WIP---not sure if needed?
d4761c9 2026-01-16 fix: address PR #251 review feedback and CI failures
12feebb 2026-01-16 refactor: delete warp-demo-kit, move demo rules to echo-dry-tests
a0138de 2026-01-15 refactor(warp-core): remove test-only GenSet methods from
production API
62566e8 2026-01-15 refactor(warp-core): add EngineBuilder and audit #[allow] usage
127caa4 2026-01-15 refactor(warp-core): introduce TelemetrySink trait for hexagonal
architecture
b87bce1 2026-01-15 refactor(warp-core): make intent bytes opaque (hexagonal
architecture)
a812add 2026-01-15 refactor(warp-core): extract inbox magic strings to constants
9b064a3 2026-01-15 fix(echo): address CodeRabbitAI trivial issues (batch 8)
b88ce40 2026-01-15 fix(scripts): validate SCHEMA_HASH length before hex decoding
6179bfd 2026-01-15 fix(echo): address CodeRabbitAI P1 issues (batch 6)
fe720e0 2026-01-15 fix(echo): address CodeRabbitAI P1 issues (batch 5)
21255c4 2026-01-15 docs(warp-core): improve delete_node_cascade rustdoc
e8ed285 2026-01-15 fix(echo): address CodeRabbitAI P1 issues (batch 4)
5026563 2026-01-15 fix(echo): address CodeRabbitAI critical issues (batch 3)
52dd752 2026-01-15 docs(warp-wasm): improve install_registry rustdoc
16a3085 2026-01-15 fix(echo): address CodeRabbitAI critical issues (batch 2)
1946682 2026-01-15 fix(echo): address CodeRabbitAI review feedback (batch 1)
e1c4cb2 2026-01-15 Canonical inbox sequencing (content-addressed)
85a075d 2026-01-15 Add spec for canonical inbox sequencing
42ea44b 2026-01-15 Document converge scope and enforce determinism guards
2fa3d0c 2026-01-15 Ignore dind-report.json
02a3691 2026-01-15 Scope convergence to projected hashes
92f5e88 2026-01-15 Add DIND test app and consolidate DIND assets
e6a2622 2026-01-15 Move DIND scenarios and runner scripts into echo
c99d838 2026-01-15 Move DIND harness into echo and add determinism guards
3e2bc28 2026-01-14 merge: resolve conflicts in Cargo.toml and warp-wasm
c3a3fd2 2026-01-14 feat(dind): implement binary intent log and deterministic hashing

⏺ Bash(git log --all --oneline --since="12 days ago" --format="%ad" --date=short | sort | uniq
-c | sort -k2)
⎿   2 2026-01-14
23 2026-01-15
29 2026-01-16
48 2026-01-17
13 2026-01-18
17 2026-01-19
38 2026-01-20
2 2026-01-22
22 2026-01-23
3 2026-01-24
72 2026-01-25
3 2026-01-26

⏺ Bash(gh pr list --state merged --limit 30 --json number,title,mergedAt,headRefName --jq '.[]
| select(.mergedAt > "2026-01-14") | "#\(.number) [\(.mergedAt | split["T"](0))]
\(.title)"')
⎿  #261 [2026-01-26] feat(warp-core): footprint enforcement guard (Phase 6B)

# 260 [2026-01-24] SPEC-0004: Worldlines, Playback, TruthBus + BOAW Phase 6B parallel

execution

# 257 [2026-01-20] Graph BOAW

# 256 [2026-01-17] feat(materialization): complete Phase 3 with FinalizeReport and

cross-platform DIND

# 254 [2026-01-17] feat: Add WSC (Write-Streaming Columnar) snapshot format

# 253 [2026-01-17] Add codec toolkit and fixed-point helpers

# 252 [2026-01-16] Generic atoms

# 251 [2026-01-16] feat(warp-core): Determinism Refactor (Float Audit + CBOR Protocol)

∴ Thinking…

Now I have the full 12-day picture. Let me organize this into a comprehensive report.

⏺ ---
12-Day Git History Report (Jan 14-26, 2026)

Executive Summary

8 PRs merged over 12 days, representing a major push on determinism infrastructure, parallel
execution, and debugging tooling:
┌──────┬────────────────────────────────────────────────────┬────────┬──────────────┐
│ PR │ Title │ Merged │ Key Theme │
├──────┼────────────────────────────────────────────────────┼────────┼──────────────┤
│ #251 │ Determinism Refactor (Float Audit + CBOR Protocol) │ Jan 16 │ Foundation │
├──────┼────────────────────────────────────────────────────┼────────┼──────────────┤
│ #252 │ Generic atoms │ Jan 16 │ Foundation │
├──────┼────────────────────────────────────────────────────┼────────┼──────────────┤
│ #253 │ Codec toolkit and fixed-point helpers │ Jan 17 │ Foundation │
├──────┼────────────────────────────────────────────────────┼────────┼──────────────┤
│ #254 │ WSC (Write-Streaming Columnar) snapshot format │ Jan 17 │ Storage │
├──────┼────────────────────────────────────────────────────┼────────┼──────────────┤
│ #256 │ Phase 3 Materialization + DIND │ Jan 17 │ Testing │
├──────┼────────────────────────────────────────────────────┼────────┼──────────────┤
│ #257 │ Graph BOAW │ Jan 20 │ Parallelism │
├──────┼────────────────────────────────────────────────────┼────────┼──────────────┤
│ #260 │ SPEC-0004: Worldlines + BOAW Phase 6B │ Jan 24 │ Architecture │
├──────┼────────────────────────────────────────────────────┼────────┼──────────────┤
│ #261 │ Footprint Enforcement Guard │ Jan 26 │ Safety │
└──────┴────────────────────────────────────────────────────┴────────┴──────────────┘

---

Commit Volume by Day

Jan 14 ██ 2
Jan 15 ███████████████████████ 23
Jan 16 █████████████████████████████ 29
Jan 17 ████████████████████████████████████████████████ 48
Jan 18 █████████████ 13
Jan 19 █████████████████ 17
Jan 20 ██████████████████████████████████████ 38
Jan 21 (none)
Jan 22 ██ 2
Jan 23 ██████████████████████ 22
Jan 24 ███ 3
Jan 25 ████████████████████████████████████████████████████████████████████████ 72 ← peak
Jan 26 ███ 3
─────────────────────────────────────────────────────
TOTAL: ~272 commits

---

Phase 1: Determinism Foundation (Jan 14-16)

PR #251: Determinism Refactor

- Float audit and CBOR protocol
- Removed globals from warp-core and warp-wasm
- Hardened determinism checks and math policies

PR #252: Generic Atoms

- Flexible attachment payloads

Key commits:
c99d838 Move DIND harness into echo and add determinism guards
42ea44b Document converge scope and enforce determinism guards
994a787 Harden determinism checks and math policies
484327a refactor: remove globals from warp-core and warp-wasm

---

Phase 2: Storage & Codec Infrastructure (Jan 16-17)

PR #253: Codec Toolkit

- Fixed-point helpers for deterministic math

PR #254: WSC Format

- Write-Streaming Columnar snapshot format
- Bounds checking and validation

Key commits:
d5f9b75 feat: Add WSC (Write-Streaming Columnar) snapshot format
6058f75 fix: Reject WSC files with out-of-bounds index ranges
a7a4bf2 fix(wsc): use read_slice in warp_view to prevent panics
34ea946 fix(wsc): return error on node/edge ordering violations
24cdaec test(wsc): add negative validation tests

---

Phase 3: Testing Infrastructure (Jan 17)

PR #256: Materialization + DIND

- FinalizeReport for materialization
- Cross-platform DIND (Determinism-In-Non-Determinism) harness

Key commits:
1d0c1e4 feat(hooks): add parallel pre-push and stale artifact sweeping
d319a8e feat(hooks): add round-robin pre-push benchmark with chart

---

Phase 4: Parallel Execution (Jan 18-20)

PR #257: Graph BOAW

- Basic BOAW (Batch-Ordered Atomic Writes) infrastructure

PR #260: SPEC-0004 + BOAW Phase 6B (30 commits, +11,216 lines)

- Cross-warp parallelism via global work queue
- WorldlineId, PlaybackCursor, ViewSession, TruthSink
- ProvenanceStore trait (hexagonal architecture)

Key commits:
c04daa4 Merge pull request #260 (Jan 24)

Closed issues:

- #258: BOAW Phase 6B: Parallel Execution Engine Integration
- #259: SPEC-0004: Worldlines, Playback, TruthBus implementation

---

Phase 5: Safety Enforcement (Jan 22-26)

PR #261: Footprint Enforcement Guard (61 commits, +7,421 / -2,294 lines)

- Runtime validation of read/write footprints
- FootprintGuard, FootprintViolation, FootprintViolationWithPanic
- DeleteNode cascade removal (explicit edge deletion required)
- Slice theorem proof tests

Key commits:
f57c5af fix(graph): DeleteNode must not cascade edges
4ecf8c8 feat(warp-core): poison deltas and guard panics
95ffd82 fix(snapshot_accum): add parent/root validation to apply_open_portal

---

Ongoing: TTD (Time-Travel Debugging) — ttd-spec branch

Not yet merged, parallel development:
33835bb feat(ttd-app): scaffold React UI for TTD browser app
e63de35 feat(ttd-browser): add WASM TtdEngine for time-travel debugging
4d34dd4 feat(warp-wasm): add TTD WASM bindings
43803e3 feat(echo-ttd): add compliance engine with channel policy checks
465c8eb feat(ttd): add echo-ttd-gen crate for TTD IR code generation

---

Architecture Evolution

Jan 14-16: Determinism foundation (floats, CBOR, globals removal)
│
▼
Jan 16-17: Storage layer (WSC format, codec toolkit)
│
▼
Jan 17: Testing infrastructure (DIND harness, pre-push benchmarks)
│
▼
Jan 18-24: Parallel execution (BOAW, work queues, worldlines)
│
▼
Jan 22-26: Safety layer (footprint enforcement, poison deltas)
│
▼
Ongoing: Debugging tooling (TTD browser app)

---

Why This Matters

The 12-day arc shows a deliberate progression:

1. Determinism first — Can't parallelize safely without deterministic primitives
2. Storage format — WSC enables efficient snapshots for playback
3. Test harness — DIND proves determinism across platforms
4. Parallelism — BOAW enables multi-warp concurrent execution
5. Safety — Footprint guards prevent data races at runtime
6. Debugging — TTD lets developers rewind and inspect

    Predicted Work: Jan 27 - Feb 2, 2026

HIGH CONFIDENCE (80%+)

1. TTD PR Completion & Merge (Days 1-2)

The ttd-spec branch is well-structured with numbered tasks:
Task 2.5 ✓ compute_tick_commit_hash_v2
Task 3.2 ✓ compliance engine
Task 5.3 ✓ WASM bindings
Task 5.4 ✓ TtdEngine
Task 6.1 ✓ React UI scaffold
Task 6.2 ✓ Three.js ScenePort adapter

Missing: Tasks 1.x, 4.x, and likely integration/polish work. Expect 15-25 more commits, then
PR #262 or similar.

Evidence: The branch follows the same pattern as #260 and #261 — structured task completion ->
review feedback rounds -> merge.

---

1. CodeRabbit Review Feedback Rounds (Days 2-4)

Based on the pattern from #261 (your session today had 6+ "fix(review)" commits), expect:

- 3-5 rounds of review feedback
- Documentation clarifications
- Edge case handling
- Test coverage additions

---

MEDIUM CONFIDENCE (50-70%)

1. TT1 Spec Work (Days 3-5)

Four open TT1 issues are queued:

# 243: dt policy (fixed timestep vs admitted dt stream)

# 244: TimeStream retention + spool compaction

# 245: Merge semantics for admitted stream facts

# 246: Security/capabilities for fork/rewind/merge

These are spec issues, not code. Likely outcome: documentation/design work in docs/spec-\*.md
files, possibly a new PR for "TT1 specs".

Evidence: The TTD work (time-travel debugging) naturally leads to specifying the underlying
time semantics.

---

1. Reliving Debugger UX Work (Days 5-7)

Issue #239 and #205 describe the debugger UI:

# 239: Constraint Lens + Provenance Heatmap

# 205: Reliving debugger MVP (scrub timeline + causal slice + fork branch)

With TTD WASM bindings and React scaffold done, the next step is wiring up the actual
debugger UI.

---

LOWER CONFIDENCE (30-50%)

1. Demo 2: Splash Guy (Days 5-7)

Five staged issues waiting:

# 222: deterministic rules + state model

# 223: lockstep input protocol + two-peer harness

# 224: controlled desync lessons

# 225: minimal rendering

# 226: networking-first course modules

This could start if TTD stabilizes quickly. The lockstep harness (#223) would leverage the
new footprint enforcement.

---

UNLIKELY THIS WEEK (<30%)

- Demo 3: Tumble Tower — Physics work (#231-238) is further out
- Wesley/Provenance specs (#193, #194, #198) — Queued but not urgent
- Major new architecture — You just shipped two big PRs; consolidation more likely

---

Predicted Commit Pattern

Jan 27 ████████████████████████ ~25 (TTD completion push)
Jan 28 ████████████████ ~18 (TTD review feedback)
Jan 29 ████████████ ~12 (TTD merge + TT1 specs start)
Jan 30 ████████ ~8 (TT1 spec writing)
Jan 31 ████████████ ~12 (Debugger UX or Demo 2 start)
Feb 01 ████████████████ ~15 (Continued development)
Feb 02 ████████ ~8 (Weekend wind-down)

---

Key Files to Watch

crates/echo-ttd/ # TTD completion
crates/ttd-browser/ # WASM debugger
docs/spec-time-streams-_# TT1 specs
docs/spec-reliving-_ # Debugger UX spec
crates/echo-dry-tests/ # Demo 2 rules (if started)

---

TL;DR: TTD PR lands by Wednesday, TT1 specs by Friday, possibly Demo 2 kickoff by weekend.

---

# 📡 The Weekly Echo — Crossing the Determinism Threshold

**Activity Report | Jan 17–24, 2026**  
_Theme: Parallel Execution + Cryptographic Worldlines + Cursor-Addressed Truth_

---

## TL;DR

This week merged parallel execution with cryptographic worldlines and laid the foundation for a time travel debugger that can verify both.

**Echo** now runs graph rewrites in parallel (with same output regardless of schedule), records history with tamper-evident commits, and prepares truth for observers via cursor-addressed subscriptions.

---

## BOAW — Parallel Execution + Footprint Enforcement

| Component                        | Status  | Timeline | Notes                                                                                            |
| -------------------------------- | ------- | -------- | ------------------------------------------------------------------------------------------------ |
| Phase 6A — Parallel Execution    | ✅ Done | Shipped  | Warp-level parallel execution with deterministic convergence. `PR #260` (`c04daa4`)              |
| Phase 6B — Footprint Enforcement | ✅ Done | Shipped  | Runtime guard enforces rule read/write footprints. Commits: `44aebb0`, `0d0231b`, docs `c84251b` |

**What this does:** Same output whether you run 1 thread or 32. Violations are deterministic and caught at runtime.

**How it works:** `BTreeMap` grouping -> virtual sharding (256 shards by hash) -> lockless claiming -> canonical merge by `WarpOpKey`. Footprint enforcement is what makes the massive parallelization possible and is critical for determinism as well.

**Test coverage:** 6 worker counts (1->32), 50 shuffle iterations/test, 50 DIND permutation seeds.

---

## SPEC-0004 — Worldlines, Playback, Sessions

| Component                   | Status  | Timeline | Notes                                                                                                                        |
| --------------------------- | ------- | -------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Worldlines                  | ✅ Done | Shipped  | Named history branches with fork/checkpoint semantics. `PR #260`                                                             |
| `PlaybackCursor`            | ✅ Done | Shipped  | Seek/step/play with Merkle verification. Reader/writer roles, frontier enforcement. Commits: `1a11892`, `774abcd`, `5d98165` |
| `ViewSession` + `TruthSink` | ✅ Done | Shipped  | Subscription routing, cursor switching, observer isolation. Commits: `fda5082`, `bb11aed`                                    |
| `TruthBus`                  | ✅ Done | Shipped  | Cursor-addressed truth publication decoupled from UI.                                                                        |
| Live-follow Subscriptions   | 🔶 Next | Feb 1–7  | Real-time push when writer advances (WebSocket/SSE). Needs session service integration.                                      |
| Reactive Notification Layer | 🔶 Next | Feb 1–7  | Push-on-cursor-advance machinery with delivery guarantees.                                                                   |

**What this does:** Engine can record, branch, and verify its own history. Observers can subscribe to truth without mutating it.

**What's missing:** Live push when the writer advances. Transport and notification layer work.

---

## MaterializationBus + MBUS Protocols

| Component            | Status  | Timeline | Notes                                                                                                     |
| -------------------- | ------- | -------- | --------------------------------------------------------------------------------------------------------- |
| `MaterializationBus` | ✅ Done | Shipped  | Tick-scoped emission with `Log`/`StrictSingle`/`Reduce` policies. `PR #256` (`f85f140`), commit `a3ebe30` |
| MBUS Frame V1        | ✅ Done | Shipped  | Internal materialization envelope. Fast inner loop.                                                       |
| MBUS Frame V2        | ✅ Done | Shipped  | Adds cursor/worldline context + per-entry BLAKE3 hashes. Commits: `1a11892`, `6076651`, `644087c`         |
| JS-ABI Wire Protocol | ✅ Done | Shipped  | Session-level hub↔client protocol with canonical CBOR + BLAKE3. echo-session-proto: `61b5386`, `80981dd`  |

**What this does:** Two protocols for two jobs. MBUS (V1/V2) for fast inner-loop truth frames. JS-ABI for boundary integrity.

**Design principle:** Order-independent storage (`BTreeMap`). No panics, no data loss. Separation keeps inner loop fast and boundary trustworthy.

---

## Time Travel Debugger

| Component                    | Status         | Timeline  | Notes                                                                       |
| ---------------------------- | -------------- | --------- | --------------------------------------------------------------------------- |
| UI Cockpit                   | ✅ Done        | Shipped   | Full-screen workspace: panels, onboarding, time controls, inspection views. |
| Server-Side Control          | ✅ Done        | Shipped   | `PlaybackCursor` state machine, `ViewSession`, `TruthSink` routing.         |
| TTD ↔ Engine Wiring          | 🔶 In Progress | Jan 25–31 | Wire cockpit commands to simulation via MBUS/FrameV1.                       |
| Inspector/Provenance Queries | 🔶 Next        | Feb 8–21  | "Explain value X" operations. Provenance slicing, causal extraction.        |

**What this does:** UI exists, server-side control exists, but they're not connected yet.

**Next milestone:** Wire UI commands to `PlaybackCursor` so the cockpit actually drives the simulation. Unlocks demos and FTUE end-to-end.

**Killer feature (pending):** Click a value -> request provenance -> UI shows exact history + rule chain that produced it.

---

## Wesley v2 + Schema-Driven Infrastructure

| Component                        | Status         | Timeline     | Notes                                                                                                     |
| -------------------------------- | -------------- | ------------ | --------------------------------------------------------------------------------------------------------- |
| Wesley v2 — Modular Schema/IR    | 🔶 In Progress | Jan 25–Feb 7 | Plugin-able targets, schema-driven codegen for protocols.                                                 |
| Schema-declared Channel Registry | 🔶 Next        | Feb 8–14     | @channel + policy declarations. Generated registries for typed subscriptions. Depends on Wesley v2.       |
| Rule Emission Contracts          | 🔶 Next        | Feb 8–14     | must-emit / may-emit-only directives. Prevents telemetry from breaking determinism. Depends on Wesley v2. |

**What this enables:** Type-safe channel subscriptions, emission contract enforcement, and the foundation for inspector/provenance operations.

**Why it matters:** Without this, telemetry can creep into nondeterminism. Schema declarations make the rules enforceable.

---

## WARPSITE (flyingrobots.dev)

| Component                  | Status  | Timeline | Notes                                                               |
| -------------------------- | ------- | -------- | ------------------------------------------------------------------- |
| App Schema                 | ✅ Done | —        | GraphQL schema declares mutations/queries for theme/nav/route/FTUE. |
| Rewrite Rules + Footprints | ✅ Done | —        | Rule executors with `compute_footprint`.                            |
| View-op Emission           | ✅ Done | —        | Kernel emits view ops to UI.                                        |
| WARP Graph Dump Types      | ✅ Done | —        | Types exist for visualization.                                      |
| MBUS Integration           | 🔶 Next | Feb 1–14 | Wire to MBUS/FrameV1 + channel subscription registry.               |
| Schema-declared Channels   | 🔶 Next | Feb 8–14 | Depends on Wesley v2 channel registry work.                         |
| Inspector/Provenance Ops   | 🔶 Next | Feb 8–21 | Operations for "explain value X" in the demo.                       |

**What this is:** Echo-in-WASM demo. Schema -> rewrites -> footprints -> deterministic view ops.

**What's missing:** Protocol layer (MBUS integration) and contract layer (channels + inspector ops).

---

## Test Coverage + Rigor

| Metric                        | Value        | What It Validates                                    |
| ----------------------------- | ------------ | ---------------------------------------------------- |
| Workspace tests passing       | 627 ✅       | Integration-heavy. Determinism is a global property. |
| `warp-core` integration tests | 491 ✅       | -                                                    |
| Failures                      | 0            | —                                                    |
| Worker counts tested          | 6 (1->32)    | Parallel execution under different schedules.        |
| Shuffle iterations/test       | 50           | Order independence.                                  |
| DIND permutation runs         | 50 seeds     | Canonical merge correctness.                         |
| Unsafe blocks                 | 0 (≈58k LoC) | No UB in parallel engine.                            |
| Determinism Torture Harness   | 🔶 TBD       | rcutorture-style seeded fuzzing. Tracked as #190.    |

---

## Up Next (Jan 25–31)

1. Wire TTD cockpit to MBUS/FrameV1 --- UI commands drive simulation
2. Continue Wesley v2 --- Schema-driven codegen for channels and contracts
3. Add live-follow subscriptions --- Writer advances trigger observer updates
4. Build reactive notification layer --- Push-on-cursor-advance machinery

Then inspector/provenance ops unlock the "causality microscope."

---

## Footer: Open for Work

I'm open for Staff/Principal roles (Seattle or remote). If you like deterministic systems, simulation, or "Git for reality," let's talk.

- James

---

**Changes:**

- One table per logical project/subsystem
- Each table shows done + in-progress + next in context
- Easier to scan "where is BOAW? oh it's done. where is TTD? oh it's half-wired"
- Removed the artificial "shipped vs next" split

Better flow?
