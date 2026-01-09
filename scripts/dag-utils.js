// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Escape a value for safe inclusion inside DOT quoted strings.
 * Escapes backslashes, quotes, newlines, carriage returns, and tabs.
 */
export function escapeDotString(value) {
  return String(value)
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t");
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
    const err = new Error(
      `Malformed ${context}: "${edgeKey}" (expected from->to)`,
    );
    err.name = "ParseError";
    throw err;
  }
  const [fromStr, toStr] = parts;
  const intRegex = /^\d+$/;
  if (!intRegex.test(fromStr) || !intRegex.test(toStr)) {
    const err = new Error(
      `Non-integer ${context}: "${edgeKey}" (parsed ${fromStr}, ${toStr})`,
    );
    err.name = "ParseError";
    throw err;
  }
  const from = Number(fromStr);
  const to = Number(toStr);
  if (!Number.isSafeInteger(from) || !Number.isSafeInteger(to)) {
    const err = new Error(
      `Non-safe-integer ${context}: "${edgeKey}" (parsed ${fromStr}, ${toStr})`,
    );
    err.name = "ParseError";
    throw err;
  }
  return { from, to };
}
