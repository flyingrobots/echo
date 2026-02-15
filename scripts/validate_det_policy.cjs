#!/usr/bin/env node
const fs = require('fs');

function validateDetPolicy(filePath) {
  if (!fs.existsSync(filePath)) {
    console.error(`Error: ${filePath} not found.`);
    return false;
  }

  try {
    // Expecting JSON format to avoid external dependencies
    const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));

    if (data.version !== 1) {
      console.error('Error: Invalid version in det-policy.yaml');
      return false;
    }

    const classes = data.classes || {};
    const crates = data.crates || {};
    const policy = data.policy || {};

    // Check classes
    for (const [className, classInfo] of Object.entries(classes)) {
      if (!classInfo.required_gates) {
        console.error(`Error: Class ${className} missing required_gates`);
        return false;
      }
    }

    // Check crates
    for (const [crateName, crateInfo] of Object.entries(crates)) {
      const cls = crateInfo.class;
      if (!classes[cls]) {
        console.error(`Error: Crate ${crateName} has unknown class ${cls}`);
        return false;
      }

      if (policy.require_owners_for_critical && cls === 'DET_CRITICAL') {
        if (!crateInfo.owner_role) {
          console.error(`Error: DET_CRITICAL crate ${crateName} missing owner_role`);
          return false;
        }
      }
    }

    console.log('det-policy.json is valid.');
    return true;
  } catch (e) {
    console.error(`Error parsing JSON: ${e}`);
    return false;
  }
}

if (require.main === module) {
  const filePath = process.argv[2] || 'det-policy.json';
  if (!validateDetPolicy(filePath)) {
    process.exit(1);
  }
}
