# Castle continuation resume card

**Castle is a second concurrent campaign.** Cellblock owns `.claude/state/active-campaign`; Castle never writes it (D-CA3). The canonical ledger is `content/castle-campaign` in `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle`. Read `README.md` (especially "Closeout state"), then `worknotes/CS-07.md`.

## State, 2026-10-18 (CS-07b G-REVIEW round 3 fixed; nothing in flight)

- `FEATURE_SHA=25beef08e` (CS-INT4S). INT4.1 `9a3a3b076`, INT4.2 `887f5d48b`, CS-07R-ID `0f1af91d4` (Castle templates 420/421 became 424/425; Tollana keeps 420/421).
- CS-07R rerun: merge `0a851a410` of local `campaign/four-praxis-zone-restoration` (`b28b4fa11`), fix commit `a8753bbe5`. Never pushed; that branch exists only locally.
- Branch head: after CS-07bF5, see `git log`; the G-REVIEW fix rounds are CS-07bF1 (`6b677cdbc`, `5f089d601`), CS-07bF2 (`7b676b438`), CS-07bF3 (docs `1eab79abc` plus two later commits), CS-07bF4 (`cc76b10d2`, `d5bc251f3`) and CS-07bF5 (revert `b21eb0013` of regen commit `2c038da1c`, plus the docs commit `12af405fa`). The pre-review head was `5faf0ae9d`. Nothing is pushed.
- Mission 567's Castle leg ships enabled (FZ-12, D-FZ1): all nine Castle chains and Harset 6503/6504 are `enabled = true`, pinned by `live_db_castle_567_chains_ship_enabled` and `live_db_chains_6503_and_6504_ship_enabled`. Client behaviour is unproven until the owner's lab UAT FZ-U1. Caveat (CS-07b3 m-3): a GM-forced step 2000 gets no cabinet feedback; do not widen 1400, because it would then fire together with 1397.
- The 115 Castle live-DB tests carry the `live_db_` prefix (CS-07bF2), so the nextest live-db group runs them.
- CS-07b1 found two reds in Harset's `letter_turn_in_route.rs`, a file Castle did not touch: `unpaired_id_fields_only_shrink` and `every_source_target_reaches_signoz_at_its_level`. Unproven as pre-existing (never run on `b28b4fa11`); assigned to the Opus four-zone review.
- Built and replay-tested: missions 10300, 10301, 10310, 10312, 10320, 10321, 10330, the 567 Castle leg, the 1521 implant (chain 1386), the hub services and barks. Everything client-visible is UATPending in `uat/end-uat.md` (D-CA11, no lab).
- Never re-verified: two `recheck` review verdicts (04c1c2, 04c2c2; CS-DOC4 was rechecked by PU-04b, and its four wrong rows are fixed in PU-02) and the hand-unreviewed `reliable_fit.rs` re-application. See the README.

## Next actions (owner decision 2026-10-18, CD-18)

1. CS-07b: rounds 1 to 3 have run, and round 3 (the last allowed round, `CS-07b3-review`, scope through `12af405fa`) gives the verdict ship: 0 blockers, 0 majors, 3 minors, 3 nits. Its m-2, n-1 and m-3 are applied in the PU-02 docs commit; m-1, n-2 and n-3 stay open.
2. Merge Castle into the local `campaign/four-praxis-zone-restoration`; the combined branch ships all four zones at once.
3. Opus four-zone review of the combined campaign, producing prescriptive Haiku fix packets; then Haiku fixes plus a Sonnet review loop.
4. CS-07c: the user runs `uat/end-uat.md`.
5. Ops opens the PR; if all CI is green, merge and release.

Do not commit regenerated `gen:` blocks in the PR (CS-07bF5 reverted that).

## Owner decisions

- OD-1 answered A (implant item), OD-2 accepted (567 reconstruction), OD-3 answered B (build now). OD-4 (hub medic, a new medical `item_lists` row) is open; template 414 and spawn 13803 stay reserved.
- OD-5 accepted for now: quartermaster 415 reuses Debug Area list 1300; flag for an economy pass before a public release.

## Blockers

HA-01b gates CS-06a/b (D-CA18). The 567 leg ships enabled under FZ-12 (D-FZ1; CD-11 discharged), with Harset 6503/6504 enabled alongside it. OD-4. Chain 1399 is dropped (CD-1). `medical_q3` and `defense_q2` are deferred (CD-8).

## Cautions and cleanup

- The Castle server is still running: PID 36540 (`castle-cs-01b`), DB `sgw_castle_cs_01b` (holds Cellblock's `Cellutb`). CS-01d stops it and is unrun (D-CA13). Never reload or drop that DB.
- For CS-07c, restart the Castle server with the CS-01b recipe; `Stop-Process` it afterwards.
- Ops retires every integrated `castle-*` worktree with `rm-worktree.ps1 <name> --force`, `castle-cs-01b` last, and the local branch `wip/cs-07r-merge-unresolved-ids`. Never run `git worktree prune`. Never touch Cellblock's worktrees, DBs, server or overlay.
- Copy worthwhile reports into `worknotes/` before any retire.