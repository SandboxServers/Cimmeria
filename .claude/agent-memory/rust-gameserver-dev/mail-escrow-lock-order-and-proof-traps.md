---
name: mail-escrow-lock-order-and-proof-traps
description: Any code that removes inventory inside another system's transaction must lock advisory → item row → sgw_player (not player first); layered race guards need a refusal-code assertion to be provable; the Bash tool mangles backslashes in heredoc'd Python.
metadata:
  type: project
---

Learned on SS-M2 (gate-mail escrow, 2026-09-27).

1. **Shared inventory lock order** is `crates/base-session/src/base/crafting/inventory_locks.rs`: `take_inventory_locks(player, bags)` (advisory `(player, 0)` then per-bag keys), then inventory rows `FOR UPDATE`, then the `sgw_player` row. A send/trade-like path that locks `sgw_player` first and the item row second can deadlock against a crafting write. The database-persistence advisor caught this in review; the mail send now calls `lock_source_item` before `deliver` locks player rows.

2. **Layered race guards are hard to prove.** Advisory lock + `FOR UPDATE` + a conditional `UPDATE … WHERE stack_size > $1` (or a `UNIQUE`) each stop a double move alone, so removing one leaves "moved once" green. What pins the locks is the *refusal the loser gets*: with locks it re-reads and gets the specific code (`ItemNotAvailable`), without them it trips the backstop and gets `db_error`. Assert the sorted result codes, not only the row counts.

3. **Forcing a failure after a mid-transaction write:** point the sequence at an id an existing row already holds (`setval(seq, TAKEN-1)`), so the next `nextval` insert hits `UNIQUE`; restore `setval(seq, old)` after. Worked for "rollback after the stack decrement".

4. **Tooling:** in this Bash tool, a heredoc'd Python script had `\\n` and `\\|` altered (unterminated-string errors, stray backslashes). Write edit scripts with the Write tool and run them with `python file.py`. `docs/**` and most `.rs` files are CRLF on disk (autocrlf); normalise to LF, edit, restore CRLF.

Related: [[stacked-branch-rebase-traps]], [[python-write-mangles-utf8-and-crlf]].
