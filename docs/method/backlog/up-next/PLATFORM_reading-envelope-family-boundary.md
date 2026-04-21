<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Reading envelope family boundary

Depends on:

- [PLATFORM_observer-plan-reading-artifacts](../asap/PLATFORM_observer-plan-reading-artifacts.md)

## Why now

Echo now clearly needs a read-side boundary built around:

- observer plan
- runtime observer instance where needed
- emitted reading artifact

What is still missing is one exact family boundary for the emitted reading
artifact itself.

Without that boundary, different consumers will keep guessing at what a reading
should carry:

- just payload
- payload plus coordinate
- payload plus witness
- payload plus rights/budget posture

The point of the reading envelope is to stop that guessing.

## What it should look like

The emitted reading envelope should be explicit about:

- plan identity
- coordinate or frontier reference
- payload
- witness or shell reference
- budget posture
- rights or revelation posture
- plurality, obstruction, or other read-status posture where relevant

This does not require one global UI shape. It does require one honest runtime
family boundary.

## Done looks like

- one packet names the minimum reading-envelope fields Echo should emit
- the boundary clearly distinguishes:
    - authored family
    - compiled artifacts
    - runtime-emitted values
- downstream repos can depend on one named family instead of reconstructing
  their own "reading result" wrappers
- the family stays narrow enough to be shared by Echo, Continuum, and debugger
  consumers

## Repo evidence

- `docs/WARP_DRIFT.md`
- `docs/design/0006-echo-continuum-alignment/design.md`
- `docs/design/0009-witnessed-causal-suffix-sync/design.md`
