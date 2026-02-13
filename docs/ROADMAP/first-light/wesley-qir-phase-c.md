<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley QIR Phase C

> **Milestone:** [First Light](README.md) | **Priority:** P1 | **Repo:** Wesley

Wesley-repo work. Extend Wesley's Query IR to compile GraphQL operations into executable SQL query plan ASTs. This builds on the existing E0-E4 foundation.

## T-2-1-1: GraphQL operation parser for QIR

**User Story:** As a Wesley user, I want to write GraphQL operations against my schema and have Wesley parse them into a typed QIR AST so that I can generate SQL query plans automatically.

**Requirements:**

- R1: Parse GraphQL query/mutation/subscription operations from `.graphql` files or inline strings.
- R2: Resolve field references against the Wesley type catalog (E0-E4 types).
- R3: Produce a typed QIR AST with resolved field paths, argument types, and return types.
- R4: Emit parse errors with source locations (line:column) for invalid operations.
- R5: Support fragment spreads and inline fragments in the initial parser.

**Acceptance Criteria:**

- [ ] AC1: A simple `query { user(id: 1) { name email } }` parses to a QIR node tree with resolved types.
- [ ] AC2: Invalid field references produce errors with source location.
- [ ] AC3: Fragment spreads are inlined into the QIR AST.
- [ ] AC4: Unit tests cover at least 5 operation shapes (simple query, nested query, mutation, fragment, variables).

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Repo: Wesley. New `qir/` module with parser and AST types. Integration with existing type catalog.
**Out of Scope:** SQL plan generation (T-2-1-2). Subscription semantics beyond parsing. Runtime execution.

**Test Plan:**

- **Goldens:** Snapshot tests for QIR AST output of 5+ canonical operations.
- **Failures:** Malformed GraphQL, unknown fields, type mismatches in arguments.
- **Edges:** Empty query body, deeply nested selections (10+ levels), duplicate fragment names.
- **Fuzz/Stress:** Fuzz the parser with random GraphQL-like strings (100k iterations).

**Blocked By:** none
**Blocking:** T-2-1-2

**Est. Hours:** 6h
**Expected Complexity:** ~400 LoC

---

## T-2-1-2: SQL query plan generation from QIR

**User Story:** As a Wesley user, I want QIR ASTs compiled into SQL query plan ASTs so that I can generate efficient database queries from my GraphQL schema.

**Requirements:**

- R1: Transform QIR AST nodes into SQL SELECT/JOIN/WHERE plan nodes.
- R2: Support basic relational mapping: object types to tables, fields to columns, relations to JOINs.
- R3: Emit a plan AST (not raw SQL strings) suitable for target-specific SQL rendering.
- R4: Handle argument-based filtering (WHERE clauses) and nested selection (JOINs).
- R5: Plan AST must be serializable to JSON for debugging and downstream consumption.

**Acceptance Criteria:**

- [ ] AC1: A nested query `{ user(id: 1) { posts { title } } }` produces a plan with SELECT + JOIN.
- [ ] AC2: Plan AST serializes to JSON matching a golden snapshot.
- [ ] AC3: Mutation operations produce INSERT/UPDATE/DELETE plan nodes.
- [ ] AC4: Plan generation errors (unmapped types, ambiguous joins) have clear messages.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Repo: Wesley. `qir/planner.ts` module. JSON serialization of plan AST.
**Out of Scope:** Actual SQL string rendering (target-specific). Query optimization. Subscription plans.

**Test Plan:**

- **Goldens:** Snapshot tests for plan AST JSON output of 5+ operations.
- **Failures:** Unmapped type, circular relation, missing required argument.
- **Edges:** Self-referencing type (recursive query), zero-field selection, aggregate-only query.
- **Fuzz/Stress:** Property test: all parseable QIR ASTs produce either a valid plan or a structured error (no crashes).

**Blocked By:** T-2-1-1
**Blocking:** none

**Est. Hours:** 6h
**Expected Complexity:** ~450 LoC

---
