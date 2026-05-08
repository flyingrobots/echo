<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Stale Task Triage

This note replaces the temporary root `ISSUES.md` scratch inventory. That file
was useful for one pass over old work, but it became too large to maintain by
hand and is not intended to be long-lived.

Use this file only for compact human triage signals. Task truth still belongs
in the source backlog cards under `docs/method/backlog/**`, GitHub issues, or
retrospectives. After a triage decision is applied to the real source, remove it
from this note.

For the full M001-M181 feature grouping and staleness pass, see
[Backlog Staleness Audit](./backlog-staleness-audit.md).

## Captured From Deleted ISSUES Scratchpad

Generated `M###` IDs were renumbered after completed backlog items were pruned.
The current IDs below correspond to the same source cards captured from the old
scratchpad.

| Task | Decision        | Follow-up                                                                                                                                                                              |
| ---- | --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| M001 | Keep            | Keep `docs/method/backlog/asap/DOCS_docs-cleanup.md`.                                                                                                                                  |
| M002 | Keep, update    | Reframe `docs/method/backlog/asap/KERNEL_echo-git-warp-compatibility-sanity-check.md` around verifying whether Echo and git-warp can share causal history through Continuum transport. |
| M003 | Keep            | Keep deterministic trig release-gate work.                                                                                                                                             |
| M004 | Keep            | Keep determinism classification CI hardening.                                                                                                                                          |
| M005 | Keep            | Keep CLI config/completions follow-up.                                                                                                                                                 |
| M006 | Keep            | Keep decoder negative-test audit mapping.                                                                                                                                              |
| M007 | Keep            | Keep Echo contract-hosting roadmap.                                                                                                                                                    |
| M008 | Needs more info | Inspect rollback playbooks before pulling; unclear whether the current TTD integration path still needs this exact task.                                                               |
| M009 | Keep            | Keep TTD protocol schema reconciliation.                                                                                                                                               |
| M010 | Keep            | Keep Wesley compiled contract-hosting doctrine.                                                                                                                                        |

## Working Rule

Do not hand-edit generated inventory files for task truth.

Prefer:

- edit the source backlog card;
- close or comment on the GitHub issue when relevant;
- regenerate `cargo xtask method matrix` and `cargo xtask method dag`;
- use `cargo xtask method frontier` and `cargo xtask method critical-path` for
  small task lists.
