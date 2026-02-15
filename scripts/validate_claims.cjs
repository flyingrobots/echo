#!/usr/bin/env node
const fs = require('fs');

/**
 * Validates that all claims marked as VERIFIED in the evidence file
 * have the required immutable CI pointers (workflow, run_id, commit_sha, artifact_name).
 * 
 * @param {string} evidenceFile - Path to the evidence JSON file.
 * @returns {boolean} - True if all verified claims are valid.
 */
function validateClaims(evidenceFile) {
  if (!fs.existsSync(evidenceFile)) {
    console.error(`Error: Evidence file ${evidenceFile} not found.`);
    return false;
  }

  try {
    const data = JSON.parse(fs.readFileSync(evidenceFile, 'utf8'));
    const requiredFields = ['workflow', 'run_id', 'commit_sha', 'artifact_name'];
    const violations = [];

    if (!data.claims || !Array.isArray(data.claims)) {
      console.error('Error: evidence.json is missing a valid claims array.');
      return false;
    }

    for (const claim of data.claims) {
      if (claim.status === 'VERIFIED') {
        const evidence = claim.evidence || {};
        const missing = requiredFields.filter(f => !evidence[f]);
        if (missing.length > 0) {
          violations.push(`Claim ${claim.id} is VERIFIED but missing pointers: ${missing.join(', ')}`);
          continue;
        }

        // Semantic validation
        if (!/^[0-9a-f]{40}$/i.test(evidence.commit_sha)) {
          violations.push(`Claim ${claim.id} has invalid commit_sha: ${evidence.commit_sha}`);
        }
        if (!/^\d+$/.test(String(evidence.run_id)) && evidence.run_id !== 'local') {
          violations.push(`Claim ${claim.id} has invalid run_id: ${evidence.run_id}`);
        }
        if (evidence.workflow === 'local' || evidence.artifact_name === 'local') {
          // Warning or violation depending on CI context
          if (process.env.GITHUB_ACTIONS) {
            violations.push(`Claim ${claim.id} has placeholder evidence ('local') in CI environment.`);
          }
        }
      }
    }

    if (violations.length > 0) {
      violations.forEach(v => console.error(v));
      return false;
    }

    console.log('All VERIFIED claims have required evidence pointers.');
    return true;
  } catch (e) {
    console.error(`Error parsing evidence JSON: ${e}`);
    return false;
  }
}

if (require.main === module) {
  const evidencePath = process.argv[2] || 'evidence.json';
  if (!validateClaims(evidencePath)) {
    process.exit(1);
  }
}
