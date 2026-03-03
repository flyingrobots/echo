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

  /**
   * Parse static-inspection.json for DET-001 claim status.
   * Returns VERIFIED only when the file exists, parses as valid JSON,
   * contains claim_id "DET-001", and status "PASSED".
   * All other conditions return UNVERIFIED with an error description.
   */
  const checkStaticInspection = (artifactsDir) => {
    const jsonPath = path.join(artifactsDir, 'static-inspection', 'static-inspection.json');
    let raw;
    try {
      raw = fs.readFileSync(jsonPath, 'utf8');
    } catch (e) {
      console.error(`DET-001: static-inspection.json not found at ${jsonPath}`);
      return { status: 'UNVERIFIED', error: 'static-inspection.json not found' };
    }

    let parsed;
    try {
      parsed = JSON.parse(raw);
    } catch (e) {
      console.error(`DET-001: invalid JSON in static-inspection.json: ${e.message}`);
      return { status: 'UNVERIFIED', error: `invalid JSON: ${e.message}` };
    }

    if (parsed.claim_id !== 'DET-001' || typeof parsed.status !== 'string') {
      console.error(`DET-001: unexpected structure in static-inspection.json: ${JSON.stringify(parsed)}`);
      return { status: 'UNVERIFIED', error: 'missing or unexpected claim_id/status field' };
    }

    const verified = parsed.status === 'PASSED';
    if (!verified) {
      console.error(`DET-001: static inspection reported status "${parsed.status}"`);
    }
    return { status: verified ? 'VERIFIED' : 'UNVERIFIED', source_status: parsed.status };
  };

  const claims = [
    (() => {
      const det001 = checkStaticInspection(gatheredArtifactsDir);
      return {
        id: 'DET-001',
        status: det001.status,
        evidence: {
          workflow, run_id: runId, commit_sha: commitSha,
          artifact_name: 'static-inspection',
          source_file: 'static-inspection.json',
          source_status: det001.source_status || null,
          ...(det001.error ? { error: det001.error } : {})
        }
      };
    })(),
    {
      id: 'DET-002',
      status: checkArtifact('det-linux-artifacts') ? 'VERIFIED' : 'UNVERIFIED',
      evidence: { workflow, run_id: runId, commit_sha: commitSha, artifact_name: 'det-linux-artifacts' }
    },
    {
      id: 'DET-003',
      status: checkArtifact('det-macos-artifacts') ? 'VERIFIED' : 'UNVERIFIED',
      evidence: { workflow, run_id: runId, commit_sha: commitSha, artifact_name: 'det-macos-artifacts' }
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
      id: 'REPRO-001',
      status: checkArtifact('build-repro-artifacts') ? 'VERIFIED' : 'UNVERIFIED',
      evidence: { workflow, run_id: runId, commit_sha: commitSha, artifact_name: 'build-repro-artifacts' }
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

module.exports = { generateEvidence };

if (require.main === module) {
  const gatheredArtifactsDir = process.argv[2] || '.';
  generateEvidence(gatheredArtifactsDir);
}
