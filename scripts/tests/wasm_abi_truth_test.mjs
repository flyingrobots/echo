// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import { CURRENT_WASM_ABI_EXPORTS } from './wasm_abi_exports.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

function read(relativePath) {
  return fs.readFileSync(path.join(REPO_ROOT, relativePath), 'utf8');
}

test('callable wasm-bindgen exports match the ABI 12 contract', () => {
  const source = read('crates/warp-wasm/src/lib.rs');
  const sourceExports = Array.from(
    source.matchAll(/#\[wasm_bindgen\]\s*\npub fn ([a-z0-9_]+)\s*\(/g),
    (match) => match[1],
  ).sort();
  assert.deepEqual(sourceExports, [...CURRENT_WASM_ABI_EXPORTS].sort());

  const spec = read('docs/spec/SPEC-0009-wasm-abi.md');
  const inventoryStart = spec.search(/Current (?:callable )?exports are/);
  const inventoryEnd = spec.indexOf('Removed exports stay removed');
  assert.notEqual(inventoryStart, -1, 'ABI spec lacks a current export inventory');
  assert.ok(inventoryEnd > inventoryStart, 'ABI spec export inventory has no end marker');
  const specExports = Array.from(
    spec.slice(inventoryStart, inventoryEnd).matchAll(/`([a-z][a-z0-9_]*)`/g),
    (match) => match[1],
  );
  assert.deepEqual(specExports.sort(), [...CURRENT_WASM_ABI_EXPORTS].sort());
});

test('current public doctrine makes bounded optics the product read boundary', () => {
  const surfaces = new Map([
    ['docs/spec/SPEC-0009-wasm-abi.md', read('docs/spec/SPEC-0009-wasm-abi.md')],
    ['crates/warp-wasm/src/lib.rs', read('crates/warp-wasm/src/lib.rs')],
    ['crates/warp-wasm/README.md', read('crates/warp-wasm/README.md')],
    ['crates/warp-core/README.md', read('crates/warp-core/README.md')],
    ['crates/echo-wasm-abi/src/kernel_port.rs', read('crates/echo-wasm-abi/src/kernel_port.rs')],
    [
      'docs/architecture/application-contract-hosting.md',
      read('docs/architecture/application-contract-hosting.md'),
    ],
  ]);
  const forbidden =
    /Observation is the only public world-state read|observe\(\.\.\.\).*only public|canonical world-state read entrypoint|canonical public read for plural|canonical shared observer\/debugger read|ABI exports \(v3\)|Phase 6 \/ ABI v3/;

  for (const [name, contents] of surfaces) {
    assert.doesNotMatch(contents, forbidden, `${name} contains superseded read doctrine`);
  }

  const applicationGuide = surfaces.get('docs/architecture/application-contract-hosting.md');
  assert.match(applicationGuide, /product contract shape is a bounded optic/);
  assert.match(applicationGuide, /Raw `ObservationRequest`.*lower-level/s);

  for (const path of [
    'docs/spec/SPEC-0009-wasm-abi.md',
    'docs/adr/0021-public-optic-observation-boundary.md',
    'docs/architecture/application-contract-hosting.md',
  ]) {
    const contents = read(path);
    assert.match(
      contents,
      /does not\s+verify/,
      `${path} hides the capability-law limitation`,
    );
    assert.match(contents, /QueryBytes/s, `${path} hides the generated-query limitation`);
    assert.match(contents, /UnsupportedProjectionLaw/s, `${path} hides the typed obstruction`);
  }
});
