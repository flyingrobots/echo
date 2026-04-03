<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Staging vs production blocker matrix

Ref: #281

Document which blockers gate staging vs production deployments.
`RELEASE_POLICY.md` has the four gates (G1-G4) but doesn't
distinguish staging from production or define the decision matrix
for when a gate failure blocks one but not the other.
