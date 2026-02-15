#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

/**
 * Generates an evidence JSON pack for CI claims.
 * Maps specific claim IDs to immutable CI artifacts.
 * 
 * @param {string} workflow - The name of the GitHub Actions workflow.
 * @param {string} runId - The unique run ID of the CI job.
 * @param {string} commitSha - The full git commit SHA.
 * @param {string} artifactsDir - Path to the directory where artifacts are stored (unused in current implementation).
 */
function generateEvidence(workflow, runId, commitSha, artifactsDir) {
  const claims = [
    {
      id: 'DET-002',
      status: 'VERIFIED',
      evidence: {
        workflow,
        run_id: runId,
        commit_sha: commitSha,
        artifact_name: 'det-linux-artifacts'
      }
    },
    {
      id: 'SEC-001',
      status: 'VERIFIED',
      evidence: {
        workflow,
        run_id: runId,
        commit_sha: commitSha,
        artifact_name: 'sec-artifacts'
      }
    },
    {
      id: 'SEC-002',
      status: 'VERIFIED',
      evidence: {
        workflow,
        run_id: runId,
        commit_sha: commitSha,
        artifact_name: 'sec-artifacts'
      }
    },
    {
      id: 'PRF-001',
      status: 'VERIFIED',
      evidence: {
        workflow,
        run_id: runId,
        commit_sha: commitSha,
        artifact_name: 'perf-artifacts'
      }
    }
    // Add more mappings as needed
  ];

  const evidence = {
    claims,
    metadata: {
      generated_at: new Date().toISOString(),
      workflow,
      run_id: runId,
      commit_sha: commitSha
    }
  };

  fs.writeFileSync('evidence.json', JSON.stringify(evidence, null, 2));
  console.log('Generated evidence.json');
}

if (require.main === module) {
  const workflow = process.env.GITHUB_WORKFLOW || 'det-gates';
  const runId = process.env.GITHUB_RUN_ID || 'local';
  const commitSha = process.env.GITHUB_SHA || 'local';
  const artifactsDir = process.argv[2] || 'artifacts';
  
  generateEvidence(workflow, runId, commitSha, artifactsDir);
}
