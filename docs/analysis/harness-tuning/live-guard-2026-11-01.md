# Live guard probe run, 2026-11-01 (HT-18)

Run from the HT-18 worktree (base `dd177195e`, HT-14 reviewed head) with `python tools/agent-skills/live_guard_probe.py --worktree <wt> --codex --live`, once, in the foreground. Three Claude legs ran as Haiku sessions (3 of the 6-session cap, no retry), plus the Codex probe. The probe JSON stays under the gitignored `.claude/reports/harness-tuning/` folder and is not committed.

`claude --version`: `2.1.316 (Claude Code)`

| Leg | ran | pass | evidence |
|---|---|---|---|
| orchestrator-edit | yes | no | marker missing |
| hands-on-cargo | yes | no | marker missing |
| subagent-stop | yes | yes | `report_contract:` |
| codex | yes | no | crates edit not blocked |

Probe result line: `status=ok passed=1/4`

**What the failures mean.** Only `subagent-stop` passed: a packet-coder subagent asked to reply with one bare word was blocked once by the line-mode `report_contract:` hook, so the subagent Stop guard fires. For `orchestrator-edit` the Haiku orchestrator did not call Edit: it judged `crates/README.md` out of its scope and delegated to a documentation subagent, whose Edit was denied by headless permissions; the file is unchanged. The orchestrator guard was therefore not exercised, and this leg says nothing about whether frontmatter hooks fire for the main thread. For `hands-on-cargo` the session ran the build lane wrapper, not raw `cargo`, and the PowerShell permission validator refused it; the shell guard never saw a direct cargo command, so that guard was not exercised either. Both failures are prompt defects in the HT-14 probe, not guard gaps: a leg passes only if the model attempts the forbidden action, and the prompts do not force that. The `codex` leg ran `codex_hook_probe.py --deny` and its crate edit was not blocked.

The `codex` leg reproduces C0 of the 2026-10-16 Codex UAT (see ../lean-orchestration/README.md § Codex UAT); the probe keeps only the `crates_edit_blocked` boolean, so this run does not separate a hook load failure from an unhooked edit tool. Codex hooks do not fire; Codex sessions stay unguarded. The `.codex/hooks.json` wiring is present, so this is the probe result, not a missing file. The Codex wiring of `report_contract.py` is therefore a follow-up (HT-F1), which the orchestrator adds.

## Re-run, 2026-11-01 (HT-18b)

Run from the HT-18b worktree (base `b79a098fd`, HT-14b probe prompts), with `python tools/agent-skills/live_guard_probe.py --worktree <wt> --legs orchestrator-edit,hands-on-cargo --live`, once, in the foreground, with no retry and no Codex leg. Two Claude legs ran as Haiku sessions (2 of the 6-session cap). The probe JSON stays under the gitignored `.claude/reports/harness-tuning/` folder and is not committed.

`claude --version`: `2.1.316 (Claude Code)`

| Leg | ran | pass | evidence |
|---|---|---|---|
| orchestrator-edit | yes | no | no edit attempted |
| hands-on-cargo | yes | yes | `Compiling cargo commands go through the build lane` |

Probe result line: `status=ok passed=1/2`

**What the re-run shows.** `hands-on-cargo`: the session made a raw `cargo check` attempt and the shell guard refused it with the build-lane message, so that guard is proven. `orchestrator-edit`: even with the forced prompt and `--disallowedTools Agent`, the session made no Edit or Write attempt on `crates/README.md`, so the leg is still not exercised and stays owed; this run says nothing about whether frontmatter hooks fire for the main thread. No further run is planned under HT-18b (stop rule).
