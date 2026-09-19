# Triage Labels

> **Last updated**: 2026-09-19
> **Audience**: Agent skills that triage, and the maintainers who label
> **Type**: Reference

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker, and says what each one means here.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

Only `wontfix` exists on the repo today. The other four are created the first time a maintainer's `/triage` run applies them. Only a maintainer should apply `ready-for-agent`. An agent that thinks an issue qualifies does not label it: it comments with the missing pieces filled in (evidence, acceptance criteria, test type, doc rows) and leaves the label to the maintainer. If a label it needs does not exist yet, it says so in the comment and moves on.

**Autonomous kickoff:** unattended agents following [autonomous-agent-kickoff.md](../guides/autonomous-agent-kickoff.md) may pick work **only** from issues that already carry `ready-for-agent`. The `/triage` skill is how maintainers promote an issue from `needs-triage` (or unlabeled) into that queue.

## What `ready-for-agent` means here

An issue is `ready-for-agent` only when all of these hold. If any is missing, it is `needs-triage` or `ready-for-human`.

- **The premise is reconciled with the docs.** Any claim that a constant, index, or wire layout is wrong has been checked against [`docs/protocol/`](../protocol/) and the RE findings, per "When sources disagree" in [`domain.md`](domain.md). An unverified claim is the most expensive kind of ticket to hand an agent.
- **Acceptance criteria are observable.** A reviewer can tell from a test or a byte string whether it is done.
- **The test type is named**, using the picker in [`TESTING.md`](../../TESTING.md).
- **The doc-update map rows are named**, from [`CLAUDE.md`](../../CLAUDE.md).
- **Nothing in it needs tools an unattended agent does not have.** See the next section.

The ticket body should follow the contract in [`issue-tracker.md`](issue-tracker.md).

## What is always `ready-for-human`

- Anything that needs new reverse engineering: Ghidra, x64dbg, or a live `SGW.exe` client session.
- Anything whose acceptance test is in-game UAT (animations, UI feedback, "does the window open").
- Anything that touches the colo deployment, release workflow, secrets, or Discord webhooks.
- Anything that changes wire encryption, adds an opcode, or otherwise needs a client patch.
- Architecture decisions. Write the proposal, let a human decide.

## Category labels

These existing labels describe what an issue is about and sit alongside the triage role: `bug`, `enhancement`, `documentation`, `security`. `duplicate` and `invalid` are closing reasons, applied by a maintainer when closing. `good first issue` and `help wanted` are for human newcomers and do not imply `ready-for-agent`. `question` is not a substitute for `needs-info`.

`/wayfinder` adds its own `wayfinder:map` and `wayfinder:<type>` labels as described in [`issue-tracker.md`](issue-tracker.md).
