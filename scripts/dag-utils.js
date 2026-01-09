// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Escape a value for safe inclusion inside DOT quoted strings.
 */
export function escapeDotString(value) {
  return String(value).replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

/**
 * Parse an edge key of the form "from->to" into numeric node ids.
 * Throws with context if the key is malformed or non-numeric.
 */
export function parseEdgeKey(edgeKey, context = "edge key") {
  const parts = String(edgeKey)
    .split("->")
    .map((segment) => segment.trim());
  if (parts.length !== 2) {
    throw new Error(`Malformed ${context}: "${edgeKey}" (expected from->to)`);
  }
  const [fromStr, toStr] = parts;
  const from = Number.parseInt(fromStr, 10);
  const to = Number.parseInt(toStr, 10);
  if (!Number.isFinite(from) || !Number.isFinite(to)) {
    throw new Error(`Non-numeric ${context}: "${edgeKey}" (parsed ${fromStr}, ${toStr})`);
  }
  return { from, to };
}
