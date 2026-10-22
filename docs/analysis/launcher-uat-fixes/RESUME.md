# Launcher UAT fixes: resume card (CLOSED)

Ledger: [README.md](README.md) (packet status, owner risks, close-out) · Packets: [work-packets.md](work-packets.md), [work-packets-logs.md](work-packets-logs.md) · Source: [laptop UAT worknote](../launcher-telemetry/worknotes/laptop-uat-2026-10-22.md) · UAT rows: [unified-uat.md](../../guides/unified-uat.md#launcher-uat-fixes)

**State (2026-10-22): closed, pending merge of the close-out PR X.** All code is merged: U1 #1986, U2 #1984, U3 #1985, U4 #1987, U5 #1988. PR X (branch `launcher-uat-fixes/post-merge-fixes`) carries LF-20 (this ledger, UAT rows U1-U6, regenerated doc blocks) and the reviewed LF-18 follow-up fixes. Every packet row reads `UATPending`. The orchestrator empties `.claude/state/active-campaign` after X merges.

**Tested and untested.** Unit, flow and review coverage per packet, plus security passes on wave 3, LF-14 and LF-19. **No live UAT ran**: nothing on the laptop, the lab or the colo.

**Owner owes:**
1. UAT rows U1-U6 (ask before any laptop use).
2. Cut both releases together: the colo server and the launcher, the server live before or with the launcher (D-LU5; owner GO given 2026-10-22). `ops` cannot release; use `claude --agent hands-on`.
3. Delete the `LAUNCHER_LOG_SAS_URL` GitHub secret (LF-18 does not).
4. D2 from launcher-telemetry (the `inPlaceMaintenance.abandon` wording after a light abandon) is still open.

**Owner risks (accepted, D-LU3 and D-LU4):** chat and names leave the machine (per-word scrub only); 2,048-byte message floor; the server drops rows over 4,096 bytes after escaping; lines beyond 20,000 new per file are not uploaded; daily ceilings (80,000 per install, 400,000 per address); secrets under 32 characters or split by dots are kept; log rows are forgeable by a log-token holder; budgets are speed bumps; scrub gaps B2, B3 and B4. Details: [README § Close-out](README.md#close-out-2026-10-22).

**Gotchas for any follow-up.** `in_place.rs`, `inventory.rs`, `session.rs`, `log_upload/tests.rs` and `upload_gate.rs` sit near the line cap: call lines only. Engine tests use `cargo test`. `Plan` is digested: never add a field. Never `git worktree prune`.
