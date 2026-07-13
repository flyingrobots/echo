// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import assert from 'node:assert/strict';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { CURRENT_WASM_ABI_EXPORTS } from './wasm_abi_exports.mjs';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const WARP_WASM_MODULE_PATH = path.join(REPO_ROOT, 'crates', 'warp-wasm', 'pkg', 'rmg_wasm.js');

test('warp-wasm bundler package exports the callable ABI surface', async () => {
  const module = await importWarpWasmPackage();
  const publicCallableExports = Object.entries(module)
    .filter(([name, value]) => !name.startsWith('__') && typeof value === 'function')
    .map(([name]) => name)
    .sort();

  assert.deepEqual(publicCallableExports, [...CURRENT_WASM_ABI_EXPORTS].sort());
});

async function importWarpWasmPackage() {
  try {
    return await import(pathToFileURL(WARP_WASM_MODULE_PATH).href);
  } catch (cause) {
    throw new Error(
      `${WARP_WASM_MODULE_PATH} is not importable; run scripts/build-warp-wasm-package.sh first`,
      { cause },
    );
  }
}
