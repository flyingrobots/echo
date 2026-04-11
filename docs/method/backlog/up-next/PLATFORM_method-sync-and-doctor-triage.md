<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Triage METHOD drift against ~/git/method

Echo already has METHOD scaffolding and active cycle/backlog structure, so this
should not become an open-ended "refresh everything" cleanup pass.

The right bounded follow-up is:

1. compare Echo's local METHOD surface against `~/git/method`
2. run the doctor once as a diagnostic, not a gate
3. sort findings into:
    - blocking now
    - worth batching soon
    - ignore for now
4. pull only the upstream METHOD changes that materially improve current Echo
   work

What this should explicitly avoid:

- a repo-wide process migration with no direct payoff
- blocking Echo/TTD integration work on template or bookkeeping drift
- treating every doctor warning as equally important

Why keep this on the backlog:

- Echo is already participating in the cross-repo Continuum work
- stale METHOD scaffolding will eventually create friction
- the repo should adopt useful upstream METHOD improvements deliberately rather
  than by accident
