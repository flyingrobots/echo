<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Roadmap Status Definitions

This document defines the lifecycle states for milestones and features in the Echo roadmap.

## Status Hierarchy

| State              | Definition                                                                                    |
| :----------------- | :-------------------------------------------------------------------------------------------- |
| **Planned**        | Item is scheduled but work has not yet begun.                                                 |
| **In Progress**    | Active development is underway.                                                               |
| **Pending Review** | Implementation is complete; awaiting PR review and merge.                                     |
| **Verified**       | Work is merged to `main`, and all binary exit criteria/DoD items are satisfied and evidenced. |
| **Archived**       | Item is complete and has been superseded or moved to a long-term maintenance state.           |

## Verification Requirements

An item cannot transition to **Verified** until:

1. All linked PRs are merged.
2. CI is green for the merge commit.
3. All Definition of Done (DoD) checkboxes are checked.
4. Explicit evidence (PR links, audit comments, workflow runs) is recorded in the document.
