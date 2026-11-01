# Live guard probe run, 2026-11-01 (HT-18)

Run from the HT-18 worktree (base `dd177195e`, HT-14 reviewed head) with `python tools/agent-skills/live_guard_probe.py --worktree <wt> --codex --live`, once, in the foreground. Four Claude legs ran as Haiku sessions (4 of the 6-session cap, no retry). The probe JSON stays under the gitignored `.claude/reports/harness-tuning/` folder and is not committed.

`claude --version`: `2.1.316 (Claude Code)`

| Leg | ran | pass | evidence |
|---|---|---|---|
| orchestrator-edit | yes | no | marker missing |
| hands-on-cargo | yes | no | marker missing |
| subagent-stop | yes | yes | `report_contract:` |
| codex | yes | no | crates edit not blocked |

Probe result line: `status=ok passed=1/4`

**What the failures mean.** Only `subagent-stop` passed: a packet-coder subagent asked to reply with one bare word was blocked once by the line-mode `report_contract:` hook, so the subagent Stop guard fires. For `orchestrator-edit` the stream never held the `orchestrator_guard:` marker; git status in the worktree is clean afterwards, so the probe's append to `crates/README.md` did not land. This matches the owed-table gap that main-thread frontmatter hooks are unverified, but this run does not prove the cause. For `hands-on-cargo` the stream never held the build-lane message, so the hands-on cargo guard did not show in the stream; this run did not compile anything. The `codex` leg ran `codex_hook_probe.py --deny` and its crate edit was not blocked.

Codex hooks do not fire; Codex sessions stay unguarded. The `.codex/hooks.json` wiring is present, so this is the probe result, not a missing file. The Codex wiring of `report_contract.py` is therefore a follow-up (HT-F1), which the orchestrator adds.
