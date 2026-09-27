---
name: mail-expiry-and-notify-seams
description: SS-M4 mail expiry/notification seams - every sgw_gate_mail writer must set expires_at, quarantined must be filtered on every player path, system-mail callers must call notify; ordered-gate race tests
metadata:
  type: project
---

Gate mail (SS-M4, 2026-09-27) added `expires_at` and `quarantined` to `sgw_gate_mail`.

- **Every writer stamps `expires_at = expiry::expires_at(sent_time)`** (deliver.rs x2, cod.rs payment mail, system/write.rs; return_locked and clear_cod restamp). A new writer that forgets it makes a mail that never expires; `every_writer_sets_expires_at` only covers the writers that existed then.
- **`NOT quarantined` must be on every player-facing query** (lock_mail, archive, delete guard, body, mark_read, header reads, cap counts, `.mailbox`). A new mail op that bypasses `claim::lock_mail` must add it.
- **Notification is opt-in per delivery path:** call `notify::notify_delivered` after the commit; system-mail callers (Black Market, SS-U3) call `SystemMailSent::notify`. `send_system_mail` itself cannot notify (no session map).
- Expiry reuses SS-M3's return via `return_::return_locked` inside `expiry::terminal::expire_one`'s own transaction.

**Why:** these are cross-cutting invariants that a later mail packet can silently break.

**How to apply:** when touching `mail/`, grep `expires_at`, `quarantined`, `notify_delivered` for the pattern. For race tests where the interesting interleaving is "A commits, then B decides", use the ordered gate in `mail/tests/expiry_race.rs` (`take_then_sweep`: park A on the SHARE-lock gate, start B once A is parked, release when both are). Removing the locks there showed B committing first rather than the stale-read dupe, so assert the ordering too. See also [[revert-proof-commit-first]], [[forced-db-race-share-lock]].
