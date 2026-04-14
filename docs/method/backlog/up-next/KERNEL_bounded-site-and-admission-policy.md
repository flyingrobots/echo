<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# KERNEL - Bounded Site and Admission Policy

Echo already has one `super_tick()` law, several real policy surfaces, and
several partial site vocabularies. What it lacks is one explicit admission-law
packet that ties those pieces together.

This cycle should define Echo's admission-side `BoundedSite`, freeze the rule
that hosts choose engine-defined deterministic policy by reference rather than
injecting bespoke law, and restate `super_tick()` as admission over ingress
claims at bounded sites under explicit policy.
