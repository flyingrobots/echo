#!/usr/bin/env node
const fs = require('fs');
const path = require('path');
const yaml = require('js-yaml');

function matches(file, pattern) {
  const regexPattern = pattern
    .replace(/\./g, '\\.')
    .replace(/\*\*/g, '.*')
    .replace(/\*/g, '[^/]*');
  const regex = new RegExp(`^${regexPattern}$`);
  return regex.test(file);
}

function classifyChanges(policyPath, changedFilesPath) {
  if (!fs.existsSync(policyPath)) {
    console.error(`Error: ${policyPath} not found.`);
    process.exit(1);
  }

  const policy = yaml.load(fs.readFileSync(policyPath, 'utf8'));
  const changedFiles = fs.readFileSync(changedFilesPath, 'utf8').split('\n').filter(Boolean);

  let maxClass = 'DET_NONCRITICAL';

  const classPriority = {
    'DET_CRITICAL': 2,
    'DET_IMPORTANT': 1,
    'DET_NONCRITICAL': 0
  };

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

  console.log(`max_class=${maxClass}`);
  
  // Also output individual gate flags for convenience
  console.log(`run_full=${maxClass === 'DET_CRITICAL'}`);
  console.log(`run_reduced=${maxClass === 'DET_IMPORTANT' || maxClass === 'DET_CRITICAL'}`);
  console.log(`run_none=${changedFiles.length === 0}`);
}

if (require.main === module) {
  const changedFilesPath = process.argv[2] || 'changed.txt';
  classifyChanges('det-policy.yaml', changedFilesPath);
}
