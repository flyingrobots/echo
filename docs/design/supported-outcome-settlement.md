<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Supported Outcome Settlement

Status: design doctrine for Echo's role in the XYPH/Continuum/Jim stack.

Supported Outcome Settlement is the cross-repo doctrine formerly discussed as
ULTRAGOLD. Echo's responsibility is not to settle XYPH Quests. Echo produces
witnessed runtime outcomes that can later be carried through Continuum and
judged by a native authority.

## Echo Boundary

Echo owns:

- admission and obstruction posture for Echo-hosted contract artifacts;
- deterministic execution or non-application under Echo law;
- witness cores, holograms, receipts, retained evidence, and replay posture;
- legal unselected counterfactual candidates where Echo admitted a candidate
  but did not select it for commitment.

Echo does not own:

- XYPH Quest completion;
- Jim product wording;
- Continuum admissibility policy;
- proof-tier policy for a consuming authority;
- application-specific success criteria.

## Attempt Outcomes

Echo should publish attempt outcomes precisely enough for consumers to distinguish
the supported outcome they are asking about:

```text
committed_success
legal_unselected_counterfactual
obstructed_attempt
repair_candidate
invalid_proposal
runtime_fault
```

The strict vocabulary is:

```text
Counterfactual = legal but unselected.
Obstructed = refused or blocked but causally witnessed.
Repair candidate = new lawful proposal derived from obstruction.
```

Admission obstruction, runtime obstruction, scheduler rejection, legal
counterfactual, and runtime fault must not collapse into one error string.

## Obstruction As History

A stale-basis obstruction is not "nothing happened." Echo should witness that:

- the artifact/input reached the relevant runtime boundary;
- the basis guard failed;
- no write became visible on the success path;
- the obstruction kind and relevant support facts were recorded;
- replay can reproduce the same witness core when challenge replay is available.

That witness can support a downstream native consequence such as
`BLOCKED_REPAIRABLE`. It does not itself complete the downstream Quest.

## Support Tier Posture

Echo should expose the atoms needed by Tier 3 support ledgers where the owning
feature has them:

- source/runtime signature or authenticated source identity;
- history inclusion or retained coordinate;
- admission decision inclusion;
- receipt inclusion;
- witness core;
- hologram or equivalent retained execution boundary;
- state openings or retained evidence refs;
- claim binding supplied by the application/runtime consumer.

Tier 4 native hologram verification and Tier 5 proof-carrying verification are
valid future lanes. They must not be required for the v0.1.0 Jim Evidence Gate.

## Relationship To Existing Issues

- Jim Evidence Gate work should consume this as the outcome vocabulary for
  applied/rejected/obstructed/replayable evidence.
- Contract strands and counterfactual work should publish strand neighborhoods,
  not redefine every obstruction as a counterfactual.
- Proof-carrying aperture work remains future verifier support, not the default
  settlement requirement.
