# Launcher adopt UX: resume card

Ledger: [README.md](README.md) · Packets: [work-packets.md](work-packets.md) (wave 1), [work-packets-wave2.md](work-packets-wave2.md) (wave 2) · Designed 2026-10-20 at `0d71b3b30`; wave 2 redesigned 2026-10-20

**Goal.** "Use an existing game folder": name the offending entries of an `unsafe_file` refusal (relative paths, reason, fix), make the mode choice one row per mode, and document both (wave 1). Wave 2: a local launcher log, and the adoption steps sent to SigNoz at once for opted-in players.

**How (wave 2, after owner OQ-4/5/6).** Opt-in, off by default, same checkbox (renamed **Send game diagnostics**). The launcher mints a short `launcher` session on `/api/auth/dev-session` (own scope, 15 min, no refresh, no account or machine) and posts `client.launcher.adoption` rows to `/api/telemetry/upload-chunk` without Play, through its own sender (no disk queue). The server enforces the scope, caps, an exact event list, a field allow-list and a scrub. Security review: "acceptable with changes", folded in (README § Security review).

**State (2026-10-20).**
- Wave 1 is code complete on `au-integration` (tip 573e90b7c; review fixed; checks green); PR #1953 is open as a DRAFT, held by the owner until the whole campaign (waves 1 and 2) is done and ready for release UAT. Do not merge it before then.
- 2a (server) is built, reviewed, security-passed and checked green; PR #1954 is open from `launcher-adopt-ux/launcher-session` (worktree `au-srv-integration`), waiting on CI and the merge queue. Colo deploy needed before 2b is exercised.
- 2b (launcher) AU-07/10/13/08 are merged into `launcher-adopt-ux/telemetry` (tip 409f12f29); the wave 2b review is running; AU-09 waits on wave 1 reaching main.

| Wave | Packets | Ships as | Status |
|---|---|---|---|
| 1 | AU-01..AU-06 | one PR from `au-integration` | in progress |
| 2a | AU-11, AU-14 | one PR from `au-srv-integration` (`launcher-adopt-ux/launcher-session`), **held for OQ-8** | Ready |
| | AU-12 (after AU-11), AU-15 (after AU-12, AU-14), AU-16 (after AU-15) | | BlockedDependency |
| 2b | AU-07, AU-10 | one PR from `au-tel-integration` (`launcher-adopt-ux/telemetry`), after 2a | Ready |
| | AU-13 (after AU-07), AU-08 (after AU-13), AU-09 (after AU-08 and the wave-1 PR) | | BlockedDependency |

Merge order: 2a: AU-11 → AU-14 → AU-12 → AU-15 → AU-16. 2b: AU-07 → AU-10 → AU-13 → AU-08 → merge `origin/main` once wave 1 is in → AU-09. Into `main`: wave 1 → 2a (after OQ-8 approval, then a colo deploy) → 2b. The 2a wave review adds a parallel network-security-auth pass against R1-R16.

**Next step.** Finish wave 1 as planned. In parallel, `ops` creates `au-srv-integration`, `au-tel-integration`, `au-11`, `au-14`, `au-07` and `au-10` from `origin/main`; four `packet-coder`s run AU-11, AU-14, AU-07 and AU-10.

**Open owner questions** (README § Open owner questions):
- **OQ-8 (decided by the session user, 2026-10-20: "Just merge the new session kind in"):** the 2a PR is NOT held for a separate maintainer approval; merge it through the merge queue once the 2a packets, the wave review, the network-security-auth pass against R1-R16 and green CI are done. The user is not the repo owner, so name this decision in the PR body for the maintainer. 2a still needs a colo deploy before 2b is exercised.
- OQ-9: the label "Send game diagnostics". Default: yes (non-blocking).
- OQ-1 (OneDrive and junctions stay refusals), OQ-2 (non-release offenders keep blocking), OQ-7 (keep the local log): defaults built in.

**Gotchas.**
- The frontend decodes with `onExcessProperty:'error'`: Rust and TS status fields land in one commit (AU-03).
- `main.rs` ~599, `adoption/mod.rs` ~494, `handlers.rs` 515 lines: new logic goes in new files; line budgets are in each packet.
- AU-08 alone updates `Cargo.lock` (L0); every other check is `--locked`.
- Player and lab tokens and chunks must stay byte-identical; every existing admin-api test passes unchanged except one refusal text (AU-11).
- No lab, live client, live server or SigNoz in any packet; the SigNoz check is an owner step in the 2b PR body, after the 2a deploy.
- Retire each PR's worktrees with `rm-worktree.ps1 --merged-into <PR> ...` when it merges. Files under `docs/` are CRLF.
