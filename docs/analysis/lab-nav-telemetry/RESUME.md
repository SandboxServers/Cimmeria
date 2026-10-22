# Lab navigation telemetry: resume card

Ledger: [README.md](README.md) · Packets: [work-packets.md](work-packets.md) (rules R1-R8, contracts C-LN1..3, waves 1-4) and [work-packets-wave1b.md](work-packets-wave1b.md) (LN-12b, LN-20a..e) · RE: [controller-mouse-input-model.md](../../reverse-engineering/findings/controller-mouse-input-model.md)

**State (2026-10-22): design complete, owner decisions recorded.** Nothing dispatched. This is not the active campaign: `launcher-uat-fixes` owns `.claude/state/active-campaign`; leave that file alone until it closes. The campaign folder, the controller RE finding and two game-archaeology memory entries are untracked in the main checkout: brief packets with absolute main-checkout paths. LN-11 commits them.

**Decisions (owner, 2026-10-22).** D-LN1 yes, variant A (poll-only). **LN-17 stays BlockedDecision**: it needs B, and the owner did not choose B; ask one follow-up only if the owner meant B. D-LN2 (b): fix our overlay clock (LN-12b); LN-12a dropped; no lab-load before publish; re-sign and publish are owner steps. D-LN3 (b): lift the dialog quarantine (LN-20a..e); the Castle_CellBlock crash risk is accepted; the owner validates after release, no lab.

| Wave | Packets | Ships as | Status |
|---|---|---|---|
| 1 | LN-11, LN-04, LN-05, LN-01, LN-02, LN-03 | one PR each, merged in that order | Ready |
| 1b | LN-20a → LN-20b → LN-20d (stacked), LN-20c, LN-20e | one PR from `lab-nav-telemetry/wave1b`, after LN-05 merges | LN-20a, c, e Ready |
| 1b | LN-12b (BM overlay clock) | one PR | after LN-11 merges |
| 2 | LN-15 (RE), LN-19, then LN-06 + LN-07 | LN-15, LN-19 one PR each; LN-06+07 from `lab-nav-telemetry/wave2` | LN-15 after LN-11; LN-19 triage first; LN-06 after wave 1 |
| 3 | LN-08, LN-09, LN-10, LN-16 | one PR from `lab-nav-telemetry/wave3` | after wave 2; LN-10 also LN-15 |
| 4 | LN-13 (lab), LN-18 (launcher), LN-14 (close-out) | one each | blocked |

**Next.**
1. Copy `.claude/reports/lab-nav-telemetry/LN-12-re.md` to `worknotes/ln-12.md` (LN-12 is Done).
2. Check no peer is running this campaign (skill § 0); `ops`: board subcategory, worktrees `ln-01`..`ln-05`, `ln-11`, `ln-20a`, `ln-20c`, `ln-20e`, and the integration worktree `ln-w1b` (branch `lab-nav-telemetry/wave1b`).
3. Dispatch wave 1 and LN-20a, LN-20c, LN-20e (packet-coder) in the background. Merge LN-11 first, then dispatch LN-12b and LN-15.
4. LN-20b is cut at LN-20a's reviewed head, LN-20d at LN-20b's. `ops` merges LN-20a, b, d, c, e into `ln-w1b`; wave review there; one PR.
5. LN-19: first a `telemetry-triage` agent writes `worknotes/ln-19-triage.md`.
6. Before LN-10 and LN-18: a fresh architect fills LN-10's chain from LN-15 and re-checks LN-18's launcher lines.
7. LN-13 needs the lab: ask the user first and wait until it is free.

**Owner steps after release (no lab):** re-sign and publish the overlay, then check that a BM reopen sends a second `BMSearch`; check the debug-hub and Banker dialogs and two Castle_CellBlock entries (unified-uat K2/K3 after LN-20e).

**Gotchas.** Never log keys, text or cursor coordinates. Client lanes need `--target i686-pc-windows-msvc`. `aoi_dispatch.rs` (642), `inline_hooks/mod.rs` (602) and `dialog_overrides/mod.rs` (677) are near the cap. Overlay Lua is CRLF and `-text`. `docs/guides/unified-uat.md` has another campaign's uncommitted edits in the main checkout; packet worktrees cut from `main` are unaffected.
