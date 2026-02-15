#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

/**
 * Generates an evidence JSON pack for CI claims.
 * Maps specific claim IDs to immutable CI artifacts if they exist.
 * 
 * @param {string} gatheredArtifactsDir - Path to the directory where all artifacts were downloaded.
 */
function generateEvidence(gatheredArtifactsDir) {
  const workflow = process.env.GITHUB_WORKFLOW || 'det-gates';
  const runId = process.env.GITHUB_RUN_ID || 'local';
  const commitSha = process.env.GITHUB_SHA || 'local';

  const checkArtifact = (name) => {
    const fullPath = path.join(gatheredArtifactsDir, name);
    try {
      return fs.existsSync(fullPath) && fs.readdirSync(fullPath).length > 0;
    } catch (e) {
      return false;
    }
  };

  const claims = [
    {
      id: 'DET-001',
      status: checkArtifact('static-inspection') ? 'VERIFIED' : 'UNVERIFIED',
      evidence: { workflow, run_id: runId, commit_sha: commitSha, artifact_name: 'static-inspection' }
    },
    {
      id: 'DET-002',
      status: checkArtifact('det-linux-artifacts') ? 'VERIFIED' : 'UNVERIFIED',
      evidence: { workflow, run_id: runId, commit_sha: commitSha, artifact_name: 'det-linux-artifacts' }
    },
    {
      id: 'SEC-001',
      status: checkArtifact('sec-artifacts') ? 'VERIFIED' : 'UNVERIFIED',
      evidence: { workflow, run_id: runId, commit_sha: commitSha, artifact_name: 'sec-artifacts' }
    },
    {
      id: 'SEC-002',
      status: checkArtifact('sec-artifacts') ? 'VERIFIED' : 'UNVERIFIED',
      evidence: { workflow, run_id: runId, commit_sha: commitSha, artifact_name: 'sec-artifacts' }
    },
    {
      id: 'SEC-003',
      status: checkArtifact('sec-artifacts') ? 'VERIFIED' : 'UNVERIFIED',
      evidence: { workflow, run_id: runId, commit_sha: commitSha, artifact_name: 'sec-artifacts' }
    },
    {
      id: 'SEC-004',
      status: checkArtifact('sec-artifacts') ? 'VERIFIED' : 'UNVERIFIED',
      evidence: { workflow, run_id: runId, commit_sha: commitSha, artifact_name: 'sec-artifacts' }
    },
    {
      id: 'SEC-005',
      status: checkArtifact('sec-artifacts') ? 'VERIFIED' : 'UNVERIFIED',
      evidence: { workflow, run_id: runId, commit_sha: commitSha, artifact_name: 'sec-artifacts' }
    },
    {
      id: 'PRF-001',
      status: checkArtifact('perf-artifacts') ? 'VERIFIED' : 'UNVERIFIED',
      evidence: { workflow, run_id: runId, commit_sha: commitSha, artifact_name: 'perf-artifacts' }
    }
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

  const outputPath = path.join(gatheredArtifactsDir, 'evidence.json');
  fs.writeFileSync(outputPath, JSON.stringify(evidence, null, 2));
  console.log(`Generated evidence.json at ${outputPath}`);
}

if (require.main === module) {
  const gatheredArtifactsDir = process.argv[2] || '.';
  generateEvidence(gatheredArtifactsDir);
}
