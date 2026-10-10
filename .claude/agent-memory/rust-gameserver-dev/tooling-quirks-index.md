---
name: tooling-quirks-index
description: Sub-index of tooling-quirk memories (rustfmt, clippy, sqlx, shell, client patch files, Python encoding) moved out of MEMORY.md to keep it small
metadata:
  type: reference
---

# Tooling quirks

- [launcher-native-owner-handle.md](launcher-native-owner-handle.md) — native launch reads installation identity through its locked handle; Windows revert proof must run natively.
- [patchset-supersede-and-restore-to-stock](patchset-supersede-and-restore-to-stock.md) — apply skips target==result before source check; a delta back to an LZO stock map ships CME bytes.
- [ring-rig-clone-to-new-map](ring-rig-clone-to-new-map.md) — patch 010: region 3 rig roots, donor pinned to 007's result, central chunk for streaming, floor heights.
- [client-file-case-and-stock-listing](client-file-case-and-stock-listing.md) — game UI lookups are case-sensitive on Windows (eula.lua = no login); rename keeps target spelling; DATA.INF = stock names.
- [offline-client-event-trace-and-udp-port-trap](offline-client-event-trace-and-udp-port-trap.md) — no Ghidra: client Lua + PE bytes + RTTI name the CME event a handler raises.
- [mail-escrow-lock-order-and-proof-traps](mail-escrow-lock-order-and-proof-traps.md) — inventory lock order is advisory → item row → sgw_player.
- [bash-heredoc-backslash-and-metric-tests](bash-heredoc-backslash-and-metric-tests.md) — a doubled backslash in a heredoc arrives as one: use Edit for backslash text; metric tests use a per-test world label.
- [python-write-mangles-utf8-and-crlf](python-write-mangles-utf8-and-crlf.md) — `write_text` encodes cp1252: use bytes + restore CRLF.
- [i686-test-exe-uac-installer-detection](i686-test-exe-uac-installer-detection.md) — a 32-bit test exe named `*patch*` fails with os error 740 under UAC.
- [rustfmt-trailing-line-comment-quirk](rustfmt-trailing-line-comment-quirk.md) — rustfmt pulls a standalone comment into the previous line's trailing column.
- [rustfmt-skips-sigil-tracing-macros](rustfmt-skips-sigil-tracing-macros.md) — a `%`/`?` field makes rustfmt skip the whole tracing call; reflow scripted inserts by hand.
- [rustfmt-reorders-mod-declarations](rustfmt-reorders-mod-declarations.md) — `reorder_modules` sorts `mod` lines, so "append at the end" never survives `cargo fmt`.
- [clippy-items-after-test-module](clippy-items-after-test-module.md) — `#[cfg(test)] mod tests` must be last.
- [tooling-filter-and-path-traps](tooling-filter-and-path-traps.md) — `live-db-test.sh` takes positional substrings, not filtersets.
- [sqlx-dynamic-sql-string](sqlx-dynamic-sql-string.md) — `sqlx::query` needs `&'static str`.
- [sqlx-chain-id-is-i32-vacuous-guards](sqlx-chain-id-is-i32-vacuous-guards.md) — `content_*.chain_id` is i32; a wrong decode type hides inside "no rows" guards.
- [gitignore-swallows-new-dirs](gitignore-swallows-new-dirs.md) — unanchored `.gitignore` dir rules hide a new `foo/mod.rs`.
- [worktree-shell-and-external-binary-tests](worktree-shell-and-external-binary-tests.md) — worktree Bash refuses `env VAR=x cmd`, heredoc appends, chained commits.
