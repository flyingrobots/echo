// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import assert from "node:assert/strict";
import { parseTasksDag } from "../parse-tasks-dag.js";

function runTests() {
  const content = `## [#1: Foo](https://example.com/1)
- Blocks:
  - [#2: Bar](https://example.com/2)
  - Confidence: medium
  - Evidence: Confirmed in plan
  - Blocked by:
    - [#3: Baz](https://example.com/3)
    - Confidence: weak
`;
  const { nodes, edges } = parseTasksDag(content);
  assert.strictEqual(nodes.get(1).title, "Foo");
  assert.strictEqual(nodes.get(2).title, "Bar");
  assert.strictEqual(edges.length, 2);
  const [firstEdge, secondEdge] = edges;
  assert.deepStrictEqual(firstEdge, {
    from: 1,
    to: 2,
    confidence: "medium",
    note: "Confirmed in plan",
  });
  assert.deepStrictEqual(secondEdge, {
    from: 3,
    to: 1,
    confidence: "weak",
    note: "",
  });
  console.log("parseTasksDag tests passed");
}

runTests();
