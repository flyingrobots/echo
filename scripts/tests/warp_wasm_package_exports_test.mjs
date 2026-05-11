// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import assert from 'node:assert/strict';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath, pathToFileURL } from 'node:url';

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const WARP_WASM_MODULE_PATH = path.join(REPO_ROOT, 'crates', 'warp-wasm', 'pkg', 'rmg_wasm.js');

const REQUIRED_BYTE_ABI_EXPORTS = Object.freeze([
  'init',
  'dispatch_intent',
  'observe',
  'scheduler_status',
  'get_codec_id',
  'get_registry_version',
  'get_schema_sha256_hex',
]);

test('warp-wasm bundler package exports the byte ABI surface', async () => {
  const module = await importWarpWasmPackage();

  for (const exportName of REQUIRED_BYTE_ABI_EXPORTS) {
    assert.equal(
      typeof module[exportName],
      'function',
      `expected pkg/rmg_wasm.js to export ${exportName}`,
    );
  }
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
