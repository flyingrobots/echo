#!/usr/bin/env node
const fs = require('fs');

const ALLOWED_GATES = new Set(['G1', 'G2', 'G3', 'G4']);

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

    if (data.version !== 1) {
      console.error(`Error: Invalid version in ${filePath}`);
      return false;
    }

    const classes = data.classes || {};
    const crates = data.crates || {};
    const policy = data.policy || {};

    try {
      assertValidRequiredGates(classes);
    } catch (error) {
      console.error(`Error: ${error.message}`);
      return false;
    }

    // Check crates
    for (const [crateName, crateInfo] of Object.entries(crates)) {
      if (!crateInfo.class) {
        console.error(`Error: Crate ${crateName} missing class`);
        return false;
      }
      const cls = crateInfo.class;
      if (!classes[cls]) {
        console.error(`Error: Crate ${crateName} has unknown class ${cls}`);
        return false;
      }

      if (!crateInfo.paths || !Array.isArray(crateInfo.paths) || crateInfo.paths.length === 0) {
        console.error(`Error: Crate ${crateName} missing or invalid paths`);
        return false;
      }

      if (policy.require_owners_for_critical && cls === 'DET_CRITICAL') {
        if (!crateInfo.owner_role) {
          console.error(`Error: DET_CRITICAL crate ${crateName} missing owner_role`);
          return false;
        }
      }
    }

    console.log(`${filePath} is valid.`);
    return true;
  } catch (e) {
    console.error(`Error parsing JSON: ${e}`);
    return false;
  }
}

module.exports = { assertValidRequiredGates, validateDetPolicy };

if (require.main === module) {
  const filePath = process.argv[2] || 'det-policy.json';
  if (!validateDetPolicy(filePath)) {
    process.exit(1);
  }
}
