#!/usr/bin/env node
const fs = require('fs');

const ALLOWED_GATES = new Set(['G1', 'G2', 'G3', 'G4']);
const REQUIRED_CLASSES = new Set(['DET_CRITICAL', 'DET_IMPORTANT', 'DET_NONCRITICAL']);

function assertValidRequiredGates(classes) {
  for (const [className, classInfo] of Object.entries(classes)) {
    if (!Array.isArray(classInfo.required_gates)) {
      throw new Error(`Class ${className} missing or invalid required_gates (must be an array)`);
    }

    const seen = new Set();
    for (const gate of classInfo.required_gates) {
      if (!ALLOWED_GATES.has(gate)) {
        throw new Error(`Class ${className} has invalid required gate ${gate}`);
      }
      if (seen.has(gate)) {
        throw new Error(`Class ${className} has duplicate required gate ${gate}`);
      }
      seen.add(gate);
    }
  }
}

function assertValidDetPolicy(data) {
  if (data.version !== 1) {
    throw new Error('invalid policy version');
  }

  const classes = data.classes;
  if (!classes || typeof classes !== 'object' || Array.isArray(classes)) {
    throw new Error('missing or invalid classes');
  }
  for (const className of REQUIRED_CLASSES) {
    if (!Object.hasOwn(classes, className)) {
      throw new Error(`missing required class ${className}`);
    }
  }
  for (const className of Object.keys(classes)) {
    if (!REQUIRED_CLASSES.has(className)) {
      throw new Error(`unsupported class ${className}`);
    }
  }
  assertValidRequiredGates(classes);

  const crates = data.crates;
  if (!crates || typeof crates !== 'object' || Array.isArray(crates)) {
    throw new Error('missing or invalid crates');
  }
  const policy = data.policy || {};
  for (const [crateName, crateInfo] of Object.entries(crates)) {
    if (!crateInfo || typeof crateInfo !== 'object' || Array.isArray(crateInfo)) {
      throw new Error(`Crate ${crateName} has invalid policy`);
    }
    if (!Object.hasOwn(classes, crateInfo.class)) {
      throw new Error(`Crate ${crateName} has unknown class ${crateInfo.class}`);
    }
    if (
      !Array.isArray(crateInfo.paths)
      || crateInfo.paths.length === 0
      || crateInfo.paths.some((path) => typeof path !== 'string' || path.length === 0)
    ) {
      throw new Error(`Crate ${crateName} missing or invalid paths`);
    }
    if (
      policy.require_owners_for_critical
      && crateInfo.class === 'DET_CRITICAL'
      && (typeof crateInfo.owner_role !== 'string' || crateInfo.owner_role.length === 0)
    ) {
      throw new Error(`DET_CRITICAL crate ${crateName} missing owner_role`);
    }
  }
}

/**
 * Validates the structure and content of a det-policy JSON file.
 * Checks for required gate definitions, crate classifications, and owner assignments.
 * 
 * @param {string} filePath - Path to the det-policy JSON file.
 * @returns {boolean} - True if the policy file is valid.
 */
function validateDetPolicy(filePath) {
  if (!fs.existsSync(filePath)) {
    console.error(`Error: ${filePath} not found.`);
    return false;
  }

  try {
    // Expecting JSON format to avoid external dependencies
    const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));

    assertValidDetPolicy(data);

    console.log(`${filePath} is valid.`);
    return true;
  } catch (e) {
    console.error(`Error: ${e.message}`);
    return false;
  }
}

module.exports = { assertValidDetPolicy, assertValidRequiredGates, validateDetPolicy };

if (require.main === module) {
  const filePath = process.argv[2] || 'det-policy.json';
  if (!validateDetPolicy(filePath)) {
    process.exit(1);
  }
}
