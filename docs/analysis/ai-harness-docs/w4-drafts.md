# ai-harness-docs: wave 4 verbatim drafts

The packets in [work-packets-w4.md](work-packets-w4.md) and [work-packets-w4b.md](work-packets-w4b.md) copy these blocks into the target files with the splice command (C16) or by extraction. Rules for every block:

- The text between `<!-- BEGIN <name> -->` and `<!-- END <name> -->` is copied exactly, with no edits. The marker lines themselves are not copied.
- Links inside a block are relative to the **target** file, not to this file. They resolve once the target is written, not here.
- No block contains a generated-block marker on a line of its own, so `tools/docs-gen/regen.py` never treats this file as a generator target.
- The splice command writes the block with the target file's own line endings.

Blocks: `reviewer-agent` (AH-17), `skill-s4` (AH-18), `ah-cycle` (AH-21). The W4b blocks are in [w4b-drafts.md](w4b-drafts.md).

<!-- BEGIN reviewer-agent -->
---
name: packet-reviewer
description: "Read-only adversarial reviewer of one packet, one wave or a whole campaign: checks it against its spec and writes numbered fix notes a packet-coder can apply. Never edits."
model: sonnet
effort: high
maxTurns: 60
tools: Read, Grep, Glob, PowerShell
---

You review finished work in the Cimmeria repository (a Rust server emulator) and write fix notes that a cheaper agent applies. You never edit files, commit, push or build. Use after a packet-coder commit, at a wave boundary, and once at the end of a campaign.

The brief gives you the scope (`packet`, `wave` or `campaign`), the worktree path, the commit range (`<base>..<head>`), the spec (the packet section path; for a wave or a campaign, the work-packets file and the ledger README), the pass number (1 to 3), from pass 2 the previous report's path, and your report path.

What to look for:
- **Packet scope.** Correctness: does the code do what the spec says in every path, including errors, empty input, concurrency and Windows-specific behaviour? Name concrete inputs or states that give a wrong result. Spec drift: anything the spec required that is missing, and anything changed that it did not ask for. Tests: does each regression test fail when the fix is reverted? A test that passes either way is theatre: say so. Repo rules: no direct `cargo` (lane only), files under 700 lines, no `helpers.rs` or `utils.rs`, doc comments on public items, CRLF for `docs/**/*.md`, no hand-edited `<!-- gen: -->` block, no secrets or IPs, telemetry on new failure paths. Code outside the diff: callers, other instances or threads, and existing tests the change silently invalidates.
- **Wave scope.** Only what no single-packet review can see: contract drift between packets (a type, signature, format or wording that two packets use differently), the same helper or text written twice, two packets editing one file or one generated block, and merge-order hazards. For every pair of branches that touch the same file, run `git -C <worktree> merge-tree --write-tree <branch-a> <branch-b>` and report each conflict.
- **Campaign scope**, on `origin/main` after the last merge: docs that no longer match behaviour, ledger decisions with nothing behind them, broken links and anchors in the changed docs, stale generated blocks (`python tools/docs-gen/regen.py --check --skip-crate-graph`), and the status-doc lines the close-out still owes.
- **From pass 2:** check each earlier note first and say whether it was applied correctly. Judge a rejected note on its merits, and drop it if the coder's reason holds.

Use `git -C <worktree> diff <range>`, `git -C <worktree> show` and `git -C <worktree> log` in PowerShell, with Read and Grep. Verify every finding against the code before you write it down; drop anything you cannot point to.

Write the report with three sections, in this order:
- `## Verdict`: one line, `ship`, `fix` or `decision`. `ship` means there are no fix notes. `fix` means every finding is a fix note. `decision` means a finding needs a spec change, a re-slice or an owner call, which a coder cannot make.
- `## Fix notes`: numbered `N1`, `N2` and so on, most severe first, one per line: `N<k>. <path>:<line> — <what is wrong> — <the exact change> — verify: <the check that proves it>`. At wave scope, tag each note with its packet: `N<k> [<packet>]. ...`. Write the exact change so a small model can apply it without judgment: the old text and the new text, or the function and what it must return.
- `## Not fixed`: the findings that are not fix notes (the `decision` items, and anything out of scope), each with its reason.

No praise, and no summary of what the diff does.

## Reporting

When the brief gives a report path, write your full report there and return exactly one line: `<packet-or-scope> done <reviewed-head-sha> <report-path> verdict=<ship|fix|decision>`. At wave scope, end the line with `verdicts=<packet>:<ship|fix|decision>,...` instead of `verdict=`. If you could not review (a missing worktree, a bad range), return `<packet-or-scope> blocked - <report-path>` and give the reason in the report. With no report path, keep your final answer under 15 lines and put the verdict on its first line. Never message other agents or wait for replies.
<!-- END reviewer-agent -->

<!-- BEGIN skill-s4 -->
## 4. Review loops, then ship

Three review loops run. They have the same shape and the same limit:

| Loop | When | Scope | Fix notes go to |
|---|---|---|---|
| Packet | as soon as that packet's coder returns `done` | the packet's commit range | a fresh `packet-coder` in that packet's worktree |
| Wave | once every packet of the wave has `ship` | every packet of the wave, together | a fresh `packet-coder` for each packet the notes name |
| Campaign | after the last wave merges, before close-out | everything the campaign merged, on `origin/main` | a fresh `packet-coder` in an `ops` worktree named `<campaign>-review` |

One pass is one fresh, read-only [`packet-reviewer`](../../agents/packet-reviewer.md)
session. It returns `verdict=ship`, `fix` or `decision`:

1. `ship`: the loop is done.
2. `fix` on pass 1 or 2: dispatch a fresh `packet-coder` with "review fix,
   pass N of 3", the worktree and the report path. It applies only the
   report's `## Fix notes`, rejects a note that does not match the code (and
   says why), reruns every check the packet lists, and commits
   `<packet> review fixes, pass N`. Then dispatch a fresh reviewer for pass
   N+1, with the previous report's path.
3. `fix` on pass 3, or `decision` on any pass: § Sessions, passes and
   escalation, step 3.

A reviewer's brief carries the scope, the worktree, the commit range
(`<base>..<head>`), the spec (the packet section path, or for a wave or the
campaign the work-packets file and the ledger README), the pass number, from
pass 2 the previous report, and the report path
`<worktree>\.claude\reports\<campaign>\<packet>-review-<N>.md` (a wave:
`wave<W>-review-<N>.md`; the campaign: `campaign-review-<N>.md`). Dispatch a
packet's first pass as soon as its coder returns; don't wait for the rest of
the wave. Add the matching domain advisor or `server-authority-enforcer` as a
parallel read-only pass only when a packet touches their area: it writes
notes `A1`, `A2` and so on, and the same fix session applies both reports.

The wave reviewer looks only for what one packet's review cannot see:
contract drift between packets, the same helper or text written twice, two
packets editing one file or generated block, and merge-order hazards. The
campaign reviewer looks for docs that no longer match behaviour, decisions
with nothing behind them, broken links and anchors, stale generated blocks,
and the status-doc lines the close-out still owes. Its fixes ship as one PR
from the `<campaign>-review` worktree. A wave of one packet has no wave loop.
In a wave that ships from an integration worktree, the wave loop's fix notes
go to one fresh `packet-coder` in the integration worktree.

Older ledgers name a combined review-and-fix agent per wave. It was retired
in 2026-10 (ai-harness-docs D-AH42); run the loops above instead.

Then ship:

1. When the wave loop returns `ship` for every packet, ship each packet
   through `ops`: `ship.ps1 pr -C <worktree> -m <msg>`, then
   `ship.ps1 merge <PR> --retire <name>`. The merge queue requires green CI,
   so the packet's lane nextest must be green first. A packet that builds on
   an unmerged one stacks on its branch (`--base <branch>`); merge the base
   first and `ship.ps1` restacks the child. Code merges stop while a
   `main-red` issue is open
   ([pre-pr-checks.md § Main is red](../../../docs/agents/pre-pr-checks.md#main-is-red)).
   A wave that must land at once ships from an integration worktree instead:
   `ops` merges each packet branch into it with `merge-noff.ps1`, the wave
   loop reviews it there, and one PR ships it. A packet merged only into an
   integration branch has no PR of its own, so `rm-worktree.ps1 --merged`
   skips its worktree and `ops` may not force it: list those worktrees for
   the owner.
2. Update the packet rows in the ledger, and write
   `worknotes/<packet>.md` only if anything is left over.

### Sessions, passes and escalation

| Count | Limit | Meaning |
|---|---|---|
| Fix round | 5 per check | One rerun of a lane check inside one session |
| Coder session | 3 per packet, and 3 per fix pass | One packet-coder run. A context stop leaves a worknote, and the next fresh session continues from it. |
| Review pass | 3 per loop | One fresh packet-reviewer session over one scope: a packet, a wave or the campaign |

Escalation, one tier at a time:

1. Context stop → a fresh packet-coder with "continuation N of 3" and the
   worknote. A fix session that stops on context continues the same way.
   Fix notes never go back into the session that wrote the code.
2. After 3 coder sessions, or 2 sessions that each end at 5 failed fix
   rounds → one Sonnet implementer (`rust-gameserver-dev` for code and
   tools, `documentation-writer` for docs) with the worknote → then
   `architect` re-slices. Opus never implements a packet.
3. `fix` after pass 3, or `decision` on any pass → `architect` (or the Opus
   advisor for protocol and RE) decides only what to fix and how, in a
   ruling note in `worknotes/<packet>.md` (a wave: `worknotes/wave<W>.md`;
   the campaign: `worknotes/campaign-review.md`). The architect writes no
   code or fix. A ruling is one of: re-spec, re-slice, accept with a
   follow-up packet, or BlockedDecision. A Sonnet implementer (as in step 2)
   applies it, then one fresh reviewer runs a ruling check, outside the
   3-pass count. A ruling check that is not `ship`, or an owner call, is
   BlockedDecision.

Why: Haiku 5.5 bills 5x per request above a 100k prompt, so a fresh session
is cheaper than a long one; oversteps where Opus wrote code cost $107.43 in
the Haiku campaigns (docs/analysis/campaign-metrics/findings.md), so
escalation climbs one tier at a time and Opus only rules. A reviewer that
never edits stays adversarial on every pass, and its fixes run on Haiku:
campaign-metrics ran its own 20 packets that way, with Sonnet reviews at
$13.58 and Haiku fix rounds at $0.72, and 17 of the 20 needed one.
<!-- END skill-s4 -->

<!-- BEGIN ah-cycle -->
## Cycle and escalation

Each count has a limit, and the ladder moves up when a limit runs out:

| Count | Limit | Meaning |
|---|---|---|
| Fix round | 5 per check | One rerun of a lane check inside one session |
| Coder session | 3 per packet, and 3 per fix pass | One packet-coder run. A context stop leaves a worknote, and the next fresh session continues from it. |
| Review pass | 3 per loop | One fresh, read-only packet-reviewer session over one scope: a packet, a wave or the campaign |

Three review loops run: one per packet as soon as its coder is done; one per wave once every packet in it has `ship`, for contract drift, duplicated helpers and merge-order hazards; and one per campaign after the last merge, for docs against behaviour, links, generated blocks and status docs. A pass returns `ship`, `fix` or `decision`. On `fix`, a fresh packet-coder applies the report's numbered fix notes and commits `<packet> review fixes, pass N`, and a fresh reviewer runs the next pass. The loop, the briefs and the report paths are in the [campaign-packet skill § 4](../../.claude/skills/campaign-packet/SKILL.md#4-review-loops-then-ship).

The ladder:

- **Coder:** 3 Haiku sessions, then 1 Sonnet implementer (`rust-gameserver-dev` for code and tools, `documentation-writer` for docs), then `architect` re-slices. Haiku comes first, and a packet is sized to about 40k working tokens, because about 35-40k of each session is fixed overhead. Nine of 34 Haiku sessions hit the 90k cap before a re-slice, and 0 of 36 did after. A fresh session starts because Haiku 5.5 bills 5x per request above a 100k prompt, so the loop restarts from the worknote instead of growing one session. Sonnet is the next step for packets that still fail: 16.9% of Haiku-regime packets escalated (44 of 261), and each accepted packet needed 0.60 retries. Opus never implements: $107.43 went to Opus oversteps, of which $53.72 to $84.20 was recoverable at the intended tier.
- **Reviewer:** the verdict is `ship`, `fix` or `decision`, with at most 3 passes per loop. Review is a built-in step because only 4.8% of judged Haiku packets were accepted on the first pass: almost every packet needed a review fix. The reviewer never edits, so it stays adversarial on every pass, and the fixes run on Haiku. In campaign-metrics' own 20 packets the Sonnet reviews cost $13.58 and the Haiku fix rounds $0.72 ([findings](../analysis/campaign-metrics/findings.md#this-campaigns-own-cost)).
- **Ruling:** after 3 passes without `ship`, or on `decision`, `architect` (or the Opus advisor for protocol or RE work) decides only what to fix and how, in a ruling note: re-spec, re-slice, accept with a follow-up packet, or BlockedDecision. The architect writes no code or fix. A Sonnet implementer applies it, and one fresh reviewer checks it outside the pass count. If that check fails, or the call is the owner's, it is a BlockedDecision.

The cost is 2.5x lower per 1,000 changed lines ($3.03 against $7.44 for Opus implementing). That is the fair comparison. The per-PR figures are not: Haiku campaigns implement from a finished spec, and the cost of writing that spec sits in the Opus coordinator. The median main-session request carried 349k tokens of context, which is why the harness has no teammates and no parked workers. What this loop costs against the retired combined reviewer-fixer has not been measured yet.
<!-- END ah-cycle -->

The W4b blocks (`readme-ai-a`, `readme-ai-b` and `ai-sdlc`) moved to [w4b-drafts.md](w4b-drafts.md) on 2026-10-18 (D-AH50).
