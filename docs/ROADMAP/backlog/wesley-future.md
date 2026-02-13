<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley Future

> **Milestone:** [Backlog](README.md) | **Priority:** Unscheduled

Long-horizon Wesley enhancements tracked at the feature level. These live in the Wesley repo and are recorded here for cross-project visibility.

## T-10-9-1: Shadow REALM Investigation

**User Story:** As the Wesley runtime, I want a restricted execution and linear memory (REALM) sandbox for generated code so that user-defined validators run safely in resource-constrained environments.

**Requirements:**

- R1: Research feasibility of a REALM-like sandbox for Wesley-generated validators
- R2: Evaluate memory models (linear memory vs. arena allocation)
- R3: Prototype a minimal sandbox that runs a Wesley validator with bounded resources
- R4: Document findings and recommendation (proceed, defer, or abandon)

**Acceptance Criteria:**

- [ ] AC1: Feasibility report exists in Wesley repo
- [ ] AC2: Prototype demonstrates bounded execution of a simple validator
- [ ] AC3: Report includes performance comparison (sandbox vs. unsandboxed)
- [ ] AC4: Recommendation is documented with rationale

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Feasibility research, prototype, recommendation.
**Out of Scope:** Production implementation, WASM target, multi-language support.

**Test Plan:**

- **Goldens:** n/a (research task)
- **Failures:** Sandbox correctly rejects out-of-bounds access
- **Edges:** Validator at exactly the resource limit
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 6h
**Expected Complexity:** ~300 LoC (prototype)

---

## T-10-9-2: Multi-Language Generator Survey

**User Story:** As a Wesley user, I want code generation targets beyond TypeScript and Rust so that I can use Wesley schemas in Go, Python, and Swift projects.

**Requirements:**

- R1: Survey existing Wesley IR to identify language-agnostic vs. language-specific nodes
- R2: Design a generator plugin interface (input: IR, output: source files)
- R3: Prototype a minimal Go generator to validate the plugin interface
- R4: Document the generator plugin API for third-party authors

**Acceptance Criteria:**

- [ ] AC1: Generator plugin interface is defined and documented
- [ ] AC2: Go generator produces compilable Go types for a simple Wesley schema
- [ ] AC3: Plugin API document explains how to add a new language target
- [ ] AC4: Existing TS and Rust generators are refactored to use the plugin interface (or a migration plan is filed)

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Plugin interface design, Go prototype, documentation.
**Out of Scope:** Production-quality Go/Python/Swift generators, Zod integration for non-TS targets.

**Test Plan:**

- **Goldens:** Generated Go code for a known Wesley schema
- **Failures:** IR with unsupported type (generator should emit a clear error)
- **Edges:** Schema with recursive types, schema with enums
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 6h
**Expected Complexity:** ~400 LoC (interface + Go prototype)
