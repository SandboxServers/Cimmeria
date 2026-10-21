# Cellblock boot lock: resume card

Ledger: [README.md](README.md) · Packets: [work-packets.md](work-packets.md) · Designed 2026-10-20

**Goal.** On a fresh Praxis character's first Castle_CellBlock load, a Prison Boot Control prop stands by the spawn and the player is held in place. Right-clicking the prop opens Livewire, and winning it releases the hold (mission 689).

**How.**
- A new content verb `movement_lock` (hold/release) stores a server-side `MovementHold` on the cell entity.
- A gate in `apply_client_position_update_at`, placed after the GM bypass, snaps the player back to a fixed anchor, at most every 500 ms.
- Seeds: template 1250, spawn 284, chains 1023/1024/1025 changed, 1047/1048 new.
- No client patch. Layer A (`BSF_MOVEMENT_LOCK`) stays off until OQ-3 and BL-U2.

**State (2026-10-20): wave 1 code complete on cellblock-boot-lock/integration (worktree cbl-integration): BL-01..BL-07 merged; wave review N1/N2 fixed. BL-U1 lab gate PASSED 2026-10-20 (worknote [worknotes/BL-U1.md](worknotes/BL-U1.md)). Next: merge PR #1952, then BL-08 (OQ-3) and BL-09. Retire cbl-01..06 after it merges.** Owner decisions 2026-10-20: Goa'uld are held too (OQ-7, D-BL8 reversed; no archetype conditions), Livewire confirmed (OQ-8), Layer A stays off (OQ-3 open).

| Wave | Packets | Status |
|---|---|---|
| 1 (one integration PR, `cellblock-boot-lock/integration`, worktree `cbl-integration`) | BL-01, BL-04 | Ready |
| | BL-02, BL-03, BL-05, BL-06, BL-07 | BlockedDependency |
| | BL-U1 lab gate (ask the user first) | BlockedDependency |
| 2 (one PR per packet) | BL-08 | BlockedDecision (OQ-3) |
| | BL-U2, BL-09 | BlockedDependency |

Merge order into integration: BL-01 → BL-04 → BL-02 → BL-06 → BL-03 → BL-05 → BL-07, then the wave review, then BL-U1, then the PR.

**Next step.** Merge PR #1952 (green CI), retire cbl-01..06, then BL-08 (waits on OQ-3) and BL-09.

**Open owner questions** (README § Open owner questions; each has a default, so wave 1 is not blocked):
- OQ-1: the prop mesh and nameplate.
- OQ-2: the hold and release texts.
- OQ-3: Layer A client flag; stays off (blocks BL-08).
- OQ-4, OQ-5, OQ-6, OQ-9: auto-release after losses, the prop after release, the no-op abilities, the superseded four-zone probe.
- Decided by owner 2026-10-20: OQ-7 (no Goa'uld exclusion), OQ-8 (Livewire; the owner's note called it OQ-5).

**Gotchas.**
- `BSF_MOVEMENT_LOCK` is ref-counted, so set it once and unset it once.
- Rows naming `set_interaction_type`/`interact_tag` stay on one line (the linter).
- Ask the user before any lab use; the user builds the server.
- Files under `docs/` are CRLF.
