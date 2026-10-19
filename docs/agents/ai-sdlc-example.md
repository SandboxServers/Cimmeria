# A day in the life of one packet

> **Last updated**: 2026-10-18
> **Audience**: Anyone who has read the README's AI section and wants to see the workflow on one real change
> **Type**: Explanation

[How we build with AI](ai-sdlc.md) gives the rules. This page follows one real packet, AH-11m (PR #1842), from its place in the plan to the merge, so you can see what each rule looks like on an actual change. The rules live in ai-sdlc.md, the README and the [campaign-packet skill](../../.claude/skills/campaign-packet/SKILL.md); this page links them and does not repeat them.

Every fact below comes from three sources: the `ai-harness-docs` campaign ledger ([README](../analysis/ai-harness-docs/README.md) and [work-packets-w3d.md](../analysis/ai-harness-docs/work-packets-w3d.md)), the PR's metadata (`gh pr view 1842`), and its diff. Where those sources are silent, this page says **not recorded** instead of guessing.

## One difference from today's loop

AH-11m ran in wave 3d, before the read-only review loop of contract C12 (D-AH43) landed. Its review was one Sonnet session that both reviewed and fixed (the combined reviewer-fixer role, retired by D-AH43). Today a read-only `packet-reviewer` finds problems, a fresh `packet-coder` fixes them, there are at most 3 fix passes, and an architect rules on a dispute. The README explains that loop under [Review loops](../../README.md#review-loops).

## 1. The packet exists before any code does

The campaign had an architect write the ledger and the packets first. The ledger row for AH-11m reads: wave 3d, agent `packet-coder`, subject "`branch-at.sh`", starts when AH-11e has merged. Its packet section in work-packets-w3d.md names:

- a branch (`ai-harness-docs/ah-11m-branch-at-sh`), a worktree (`ah-11m`) and a size ("about 30k working tokens");
- the commit subject, fixed in advance: `feat(build-lane): branch-at.sh, the bash twin of branch-at.ps1 (AH-11m)`;
- the two files to create, `tools/build-lane/branch-at.sh` and `tools/build-lane/test_branch_at_sh.py`.

That is the packet's whole scope. The coder edits nothing else. (The PR's head branch is `ah-11m`, not the planned long name; why is not recorded.)

## 2. The brief

The packet section is the spec the brief points to; the text of the brief actually dispatched is not recorded. These are the packet's steps and its guard list, quoted from the section (its header, its **Files:** line and its closing checks line are left out), and the shared rules carry the detail:

> **Steps:**
>
> 1. Read `branch-at.ps1` (106 lines) and `test-branch-at.ps1` (154 lines).
> 2. Write `branch-at.sh`. Its parameters are `-C`, `-Name` and `-Sha`, all mandatory. `-C` may be any repository or worktree: it needs only `git -C "$OPT_C" rev-parse --git-dir` for `not-a-git-repository`.
> 3. Port every name check with the `.ps1`'s letter case. Its reserved-name list and its `refs/` test are case-insensitive (`-contains`, `-match`), and the existing-branch compare refuses `Topic` against `topic`.
> 4. Write `test_branch_at_sh.py`, with one method per section of `test-branch-at.ps1`.
>
> **Prove these guards:** the pseudo-ref name (`rebase_head` in lower case), `branch-exists` for a case-only difference, and the "already resolves" check (`stash`).

Three things in that brief are worth noticing:

- **It names a spec the coder can read.** The existing `branch-at.ps1` is the spec, and the shared rules say that where the packet and the script disagree, the script wins.
- **It names the traps.** Step 3 exists because PowerShell's `-match` and `-contains` ignore case and bash's `[[ =~ ]]` does not (ledger finding F32). A literal port would flip the checks.
- **It names the tests that must fail on revert.** "Prove these guards" is the repo's rule that a change needs a test that fails when the fix is reverted ([TESTING.md](../../TESTING.md)), applied to a port.

The shared rules add the rest: bash 3.2 syntax limits, `set -euo pipefail` must not end the script before it prints its status line, create exactly the two named files, and `git update-index --chmod=+x` on the new script.

The coder is a Haiku `packet-coder` (ledger D-AH2) in its own worktree. See [Agents, tools and models](ai-sdlc.md#agents-tools-and-models) and [One worktree, one database](ai-sdlc.md#one-worktree-one-database).

## 3. The return line

A packet-coder writes its full report to a file in its worktree and returns one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>` ([Reports and one-line returns](ai-sdlc.md#reports-and-one-line-returns)). For AH-11m that is a line of this shape:

```text
AH-11m done f87b140144dfcf3618b6826ef9193175f2f1179f <report-path>
```

The SHA is real: it is the PR's only commit. The line the coder actually returned and its report path are **not recorded**, so the line above shows the shape, not a quotation.

The orchestrator then sets the ledger row to Review. It opens the report only on `blocked` or `failed`.

## 4. The commit

The PR has one commit, authored 2026-10-18 at 19:17:48Z, with the subject the packet fixed in advance and the attribution trailers. The diff is two new files, `tools/build-lane/branch-at.sh` and `tools/build-lane/test_branch_at_sh.py`, 346 added lines in all (136 and 210), and nothing else. A diff that touches only the files the packet named is the first thing a reviewer checks.

## 5. The review

Wave 3d held six packets (AH-11j to AH-11o) for one combined Sonnet review session; the wave's own header describes that one session for all six. The PR body records the result in one sentence that gives the verdict, ship.

What the review found, and whether it changed anything, is **not recorded**. The PR has a single commit, so no separate fix commit survived into it, but no review report is linked from the PR or the ledger, and this page cannot say what it checked.

Under today's loop the same packet goes through these steps instead:

1. A read-only `packet-reviewer` checks the diff against the packet and returns `verdict=ship`, `fix` or `decision`.
2. On `fix`, the orchestrator passes the report's path to a fresh `packet-coder`, which fixes the findings and commits. The reviewer never edits, and the coder never reviews its own work. There are at most 3 fix passes.
3. On `decision`, or when the passes run out, an architect rules, and the ruling is written down.

See [Review loop detail](ai-sdlc.md#review-loop-detail) for the counts and for the wave and campaign reviews.

## 6. The PR

PR #1842, "AH-11m: branch-at.sh", opened at 20:09:17Z from branch `ah-11m` onto `main`. Its body has the three sections every PR needs:

- **Summary:** the packet, the campaign, the spec section, and the review verdict.
- **Tests:** new unit tests with guard-revert proofs; `bash -n` clean; not run on WSL, because the Linux CI job covers `bash -n`, shellcheck and the build-lane tests.
- **Docs:** the doc-update-map rows are the files the packet names, and status docs change once at campaign close-out (AH-16), not per packet ([doc-update-map.md](doc-update-map.md)).

The Tests line states what ran on the author's host and which CI job covers the rest, which is the upfront tested-versus-untested statement the repo asks for.

## 7. The merge

PR #1842 merged at 20:27:12Z as `2b4017c030a0059a5e42d7fb7bee4ac51643ec66`, 18 minutes after it opened. The ledger row reads Integrated (#1842). How it was merged and whether the worktree `ah-11m` was retired the same day are **not recorded**; the rule is `ship.ps1 merge <PR> --retire <name>` ([Shipping in detail](ai-sdlc.md#shipping-in-detail)).

## What the example shows

| Step | Who | What stays in the repo |
|---|---|---|
| Plan | architect | ledger row, packet section, commit subject |
| Code | packet-coder (Haiku) | one commit |
| Review | the combined reviewer-fixer role here (retired by D-AH43); `packet-reviewer` and fresh coders today | a verdict in the PR body here |
| Ship | `ship.ps1` | the PR, its body, the merge commit |
| Close | orchestrator | the ledger row |

Not shown, because AH-11m did not need it, or because nothing records it: a `blocked` return, a worknote after a context stop, a `fix` pass and an architect ruling. The mechanics for those are in [ai-sdlc.md](ai-sdlc.md).

## Further reading

- [How we build with AI](ai-sdlc.md): the rules and numbers behind each step.
- [README: How AI development works here](../../README.md#how-ai-development-works-here): the short version.
- [development-workflow.md](development-workflow.md): the ticket-to-PR pipeline and the agent roster.
- [work-packets-w3d.md](../analysis/ai-harness-docs/work-packets-w3d.md): the packet section and shared rules quoted above.
