#!/usr/bin/env node
const fs = require('fs');

/**
 * Checks if a file path matches a glob-like pattern.
 * Supports ** for recursive directory matching and * for single level.
 * 
 * @param {string} file - The file path to check.
 * @param {string} pattern - The glob-like pattern to match against.
 * @returns {boolean} - True if the path matches the pattern.
 */
function matches(file, pattern) {
  const regexPattern = pattern
    .replace(/\./g, '\\.')
    .replace(/\*\*/g, '___DBL_STAR___')
    .replace(/\*/g, '[^/]*')
    .replace(/___DBL_STAR___/g, '.*');
  const regex = new RegExp(`^${regexPattern}$`);
  return regex.test(file);
}

/**
 * Classifies the impact of changed files based on a det-policy JSON.
 * Outputs max_class and run_* flags for GitHub Actions.
 * 
 * @param {string} policyPath - Path to the det-policy JSON file.
 * @param {string} changedFilesPath - Path to the file containing list of changed files.
 */
function classifyChanges(policyPath, changedFilesPath) {
  if (!fs.existsSync(policyPath)) {
    throw new Error(`Policy file not found: ${policyPath}`);
  }
  if (!fs.existsSync(changedFilesPath)) {
    throw new Error(`Changed files list not found: ${changedFilesPath}`);
  }

  const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
  const changedFiles = fs.readFileSync(changedFilesPath, 'utf8').split('\n').filter(Boolean);

  let maxClass = 'DET_NONCRITICAL';
  const classPriority = {
    'DET_CRITICAL': 2,
    'DET_IMPORTANT': 1,
    'DET_NONCRITICAL': 0
  };

  const requireFull = policy.policy && policy.policy.require_full_classification;

  for (const file of changedFiles) {
    let matched = false;
    if (policy.crates) {
      for (const [crateName, crateInfo] of Object.entries(policy.crates)) {
        const paths = crateInfo.paths || [];
        for (const pattern of paths) {
          if (matches(file, pattern)) {
            matched = true;
            const cls = crateInfo.class;
            if (classPriority[cls] > classPriority[maxClass]) {
              maxClass = cls;
            }
          }
        }
      }
    }
    
    if (requireFull && !matched) {
      throw new Error(`File ${file} is not classified in det-policy.yaml and require_full_classification is enabled.`);
    }
  }

  // Debug log for CI visibility
  console.error(`Classified ${changedFiles.length} files. Max class: ${maxClass}`);

  process.stdout.write(`max_class=${maxClass}\n`);
  process.stdout.write(`run_full=${maxClass === 'DET_CRITICAL'}\n`);
  process.stdout.write(`run_reduced=${maxClass === 'DET_IMPORTANT' || maxClass === 'DET_CRITICAL'}\n`);
  const noGates = changedFiles.length === 0 || maxClass === 'DET_NONCRITICAL';
  process.stdout.write(`run_none=${noGates}\n`);
}

module.exports = { classifyChanges, matches };

if (require.main === module) {
  try {
    const policyPath = process.argv[2] || 'det-policy.json';
    const changedFilesPath = process.argv[3] || 'changed.txt';
    classifyChanges(policyPath, changedFilesPath);
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}
