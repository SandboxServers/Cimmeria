---
name: project-mail-expiry-ss-m4-review
description: SS-M4 mail expiry/quarantine/notify review (2026-09-27, commit 1c4aa7616) — cleared shape, residual gaps (paid-COD TTL not restamped, empty quarantine, no GM release)
metadata:
  type: project
---

SS-M4 (worktree ss-m4, branch social/m4-notify-expiry) reviewed 2026-09-27: verdict SHIP, no dupe/destruction path.

Cleared: `expiry/terminal.rs::expire_one` reuses `claim::lock_mail` (advisory key 0 + INV_MAIN, then row FOR UPDATE, escrow, players ascending) and SS-M3's `return_locked`, so sweep vs take/pay/return serialise on the row; `return_locked` UPDATE also guards `NOT cod_paid AND NOT returned`. `lock_mail`, headers, body, archive, delete guard, send cap and system-mail cap all filter `NOT quarantined`. Archive is one-way (no unarchive path exists), so no archive/unarchive expiry laundering. Notify uses `send_to_current_player` (re-checks `active_player_id` at send time) and re-reads the header owner-scoped after commit. No new lock cycle vs trade (trade takes both advisory keys before player rows).

Residual: `clear_cod` (paid, or cancelled-sender-gone) does not restamp `expires_at`, so a COD paid near day 30 is quarantined before the payer can take it; sender-less unpaid COD with no item is quarantined empty instead of deleted; no GM release command, so quarantine is de facto sink; `mark_read` lacks `NOT quarantined` (harmless). (Update, same day, commit 70bd7799b: `clear_cod` now restamps `expires_at`, the empty sender-less COD is deleted, and `mark_read` filters `quarantined`; the GM release command is still missing.) Owner question open: self-mail + archive = uncapped never-expiring storage (recommend archive cap at archive time).

**Why:** SS-M5+/black-market expiry will reuse this terminal-path shape.
**How to apply:** when reviewing later mail/BM expiry or a GM quarantine-release command, check TTL restamp on state change and that release goes through `lock_mail`-order locks. See [[project-mail-escrow-ss-m2]].
