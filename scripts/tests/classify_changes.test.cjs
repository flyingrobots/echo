// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

const assert = require('node:assert/strict');
const { mkdtempSync, readFileSync, rmSync, writeFileSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join, resolve } = require('node:path');
const { spawnSync } = require('node:child_process');
const test = require('node:test');

const repoRoot = resolve(__dirname, '../..');
const classifier = join(repoRoot, 'scripts/classify_changes.cjs');

const policy = {
  version: 1,
  classes: {
    DET_CRITICAL: { required_gates: ['G1', 'G2', 'G3', 'G4'] },
    DET_IMPORTANT: { required_gates: ['G2', 'G4'] },
    DET_NONCRITICAL: { required_gates: [] },
  },
  crates: {
    critical: { class: 'DET_CRITICAL', paths: ['crates/critical/**'] },
    important: { class: 'DET_IMPORTANT', paths: ['crates/important/**'] },
    catch_all: { class: 'DET_NONCRITICAL', paths: ['**'] },
  },
  policy: { require_full_classification: true },
};

function runClassifier(changedFile, fixturePolicy = policy) {
  const fixtureDir = mkdtempSync(join(tmpdir(), 'echo-det-policy-'));
  const policyPath = join(fixtureDir, 'policy.json');
  const changedPath = join(fixtureDir, 'changed.txt');

  try {
    writeFileSync(policyPath, JSON.stringify(fixturePolicy));
    writeFileSync(changedPath, `${changedFile}\n`);
    return spawnSync(process.execPath, [classifier, policyPath, changedPath], {
      encoding: 'utf8',
    });
  } finally {
    rmSync(fixtureDir, { recursive: true, force: true });
  }
}

function classify(changedFile) {
  const result = runClassifier(changedFile);

  try {
    assert.equal(result.status, 0, result.stderr);
    return Object.fromEntries(
      result.stdout.trim().split('\n').map((line) => line.split('=')),
    );
  } catch (error) {
    error.message += `\nclassifier stderr:\n${result.stderr}`;
    throw error;
  }
}

test('required_gates drive the emitted workflow gate flags', () => {
  assert.deepEqual(classify('crates/critical/src/lib.rs'), {
    max_class: 'DET_CRITICAL',
    run_g1: 'true',
    run_g2: 'true',
    run_g3: 'true',
    run_g4: 'true',
    run_none: 'false',
  });

  assert.deepEqual(classify('crates/important/src/lib.rs'), {
    max_class: 'DET_IMPORTANT',
    run_g1: 'false',
    run_g2: 'true',
    run_g3: 'false',
    run_g4: 'true',
    run_none: 'false',
  });

  assert.deepEqual(classify('README.md'), {
    max_class: 'DET_NONCRITICAL',
    run_g1: 'false',
    run_g2: 'false',
    run_g3: 'false',
    run_g4: 'false',
    run_none: 'true',
  });
});

test('the determinism workflow consumes every emitted gate flag', () => {
  const workflow = readFileSync(join(repoRoot, '.github/workflows/det-gates.yml'), 'utf8');

  for (const gate of ['g1', 'g2', 'g3', 'g4']) {
    assert.match(workflow, new RegExp(`run_${gate}: \\$\\{\\{ steps\\.classify\\.outputs\\.run_${gate} \\}\\}`));
    assert.match(workflow, new RegExp(`needs\\.classify-changes\\.outputs\\.run_${gate}`));
    assert.match(workflow, new RegExp(`RUN_${gate.toUpperCase()}: \\$\\{\\{ needs\\.classify-changes\\.outputs\\.run_${gate} \\}\\}`));
  }

  assert.doesNotMatch(workflow, /run_full|run_reduced/);
  assert.match(workflow, /if \[ "\$RUN_G3" = "true" \]; then/);
  assert.match(workflow, /if \[ "\$RUN_G4" = "true" \]; then/);
});

test('classification rejects unknown and duplicate gate identifiers', () => {
  const unknownGatePolicy = structuredClone(policy);
  unknownGatePolicy.classes.DET_IMPORTANT.required_gates = ['G2', 'G5'];
  const unknown = runClassifier('crates/important/src/lib.rs', unknownGatePolicy);
  assert.equal(unknown.status, 1);
  assert.match(unknown.stderr, /invalid required gate G5/);

  const duplicateGatePolicy = structuredClone(policy);
  duplicateGatePolicy.classes.DET_IMPORTANT.required_gates = ['G2', 'G2'];
  const duplicate = runClassifier('crates/important/src/lib.rs', duplicateGatePolicy);
  assert.equal(duplicate.status, 1);
  assert.match(duplicate.stderr, /duplicate required gate G2/);
});

test('classification rejects malformed crate policy before matching paths', () => {
  const unknownClassPolicy = structuredClone(policy);
  unknownClassPolicy.crates.important.class = 'DET_IMPORANT';
  const unknownClass = runClassifier('crates/important/src/lib.rs', unknownClassPolicy);
  assert.equal(unknownClass.status, 1);
  assert.match(unknownClass.stderr, /unknown class DET_IMPORANT/);

  const missingPathsPolicy = structuredClone(policy);
  missingPathsPolicy.crates.important.paths = [];
  const missingPaths = runClassifier('crates/important/src/lib.rs', missingPathsPolicy);
  assert.equal(missingPaths.status, 1);
  assert.match(missingPaths.stderr, /missing or invalid paths/);
});

test('every workspace member has an explicit non-catch-all policy path', () => {
  const manifest = readFileSync(join(repoRoot, 'Cargo.toml'), 'utf8');
  const membersMatch = manifest.match(/members\s*=\s*\[([\s\S]*?)\]/);
  assert.ok(membersMatch, 'Cargo.toml has no workspace members array');
  const members = Array.from(membersMatch[1].matchAll(/"([^"]+)"/g), (match) => match[1]);
  const policyYaml = readFileSync(join(repoRoot, 'det-policy.yaml'), 'utf8');
  const missing = members.filter((member) => !policyYaml.includes(`"${member}/**"`));

  assert.deepEqual(missing, [], `workspace members without explicit policy paths: ${missing}`);
  assert.match(
    policyYaml,
    /echo-edict-provider-verifier:\n\s+class: DET_CRITICAL\n\s+owner_role: "Tooling Engineer"\n\s+paths: \["crates\/echo-edict-provider-verifier\/\*\*", "schemas\/edict-provider\/components\/v1\/verifier\.echo-dpo\.component\.wasm", "tests\/edict-provider-host-v1\/\*\*"\]\n/,
  );
  assert.match(policyYaml, /echo-runtime-schema:\n\s+class: DET_CRITICAL\n/);
  assert.match(policyYaml, /echo-file-aperture:\n\s+class: DET_IMPORTANT\n/);
  assert.match(policyYaml, /echo-trace:\n\s+class: DET_IMPORTANT\n/);
});
