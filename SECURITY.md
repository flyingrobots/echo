<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Security Policy

## Supported Versions
Echo Engine is under active development on the `main` branch. Until the first
stable release, security fixes will land directly on `main` and be included in
the next tagged version.

## Reporting a Vulnerability
Please email **security@echoengine.dev** with details. Include:
- Description of the issue and potential impact.
- Steps to reproduce or proof-of-concept.
- Suggested remediation if available.

You can also open a security advisory via GitHub's "Report a vulnerability" feature.

We aim to acknowledge reports within 72 hours and provide a remediation plan
within 7 days. Please do not disclose vulnerabilities publicly until we have
coordinated a fix.

## Scope
Echo is a deterministic engine; common vectors include:
- Sandbox escapes via adapters.
- Abuse of timeline branching APIs.
- Denial-of-service through unbounded queue inputs.

If a vulnerability affects third-party dependencies, we will coordinate with
upstream maintainers.

Thank you for helping keep Echo’s timelines safe.
