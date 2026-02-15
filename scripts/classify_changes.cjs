#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

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
    .replace(/\*\*/g, '.*')
    .replace(/\*/g, '[^/]*');
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
    console.error(`Error: ${policyPath} not found.`);
    process.exit(1);
  }

  // Expecting JSON format to avoid external dependencies like js-yaml
  const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
  const changedFiles = fs.readFileSync(changedFilesPath, 'utf8').split('\n').filter(Boolean);

  let maxClass = 'DET_NONCRITICAL';

  const classPriority = {
    'DET_CRITICAL': 2,
    'DET_IMPORTANT': 1,
    'DET_NONCRITICAL': 0
  };

  if (policy.crates) {
    for (const file of changedFiles) {
      for (const [crateName, crateInfo] of Object.entries(policy.crates)) {
        const paths = crateInfo.paths || [];
        for (const pattern of paths) {
          if (matches(file, pattern)) {
            const cls = crateInfo.class;
            if (classPriority[cls] > classPriority[maxClass]) {
              maxClass = cls;
            }
          }
        }
      }
    }
  }

  process.stdout.write(`max_class=${maxClass}\n`);
  process.stdout.write(`run_full=${maxClass === 'DET_CRITICAL'}\n`);
  process.stdout.write(`run_reduced=${maxClass === 'DET_IMPORTANT' || maxClass === 'DET_CRITICAL'}\n`);
  process.stdout.write(`run_none=${changedFiles.length === 0}\n`);
}

if (require.main === module) {
  const policyPath = process.argv[2] || 'det-policy.json';
  const changedFilesPath = process.argv[3] || 'changed.txt';
  classifyChanges(policyPath, changedFilesPath);
}
