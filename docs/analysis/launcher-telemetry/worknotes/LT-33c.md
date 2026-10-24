# LT-33c worknote

Status: done. Commit f1a64ffcb (LT-33c: log shell-originated adoption refusals with their check).

Done: refusal.rs (`refusal`, two tests), events.rs (`code` is `pub(super)`), host.rs (`adoption_refusal` re-export), main.rs (ReleaseUnavailable line), mod.rs (`mod refusal`, `use refusal::refusal`, 11 listed sites wrapped: legacy_import, copy_destination, adoption_choice_allowed, admit_adoption_preview, begin_folder_adoption no-location), docs/migration.md (one CRLF paragraph after the unsafe_entries text).

Left: nothing in LT-33c. Next step: the review of LT-33c.

Last checks:
- cargo fmt --all: status=ok exit=0
- clippy -p cimmeria-launcher-desktop --all-targets -- -D warnings: status=ok exit=0
- test -p cimmeria-launcher-desktop adoption: status=ok exit=0 (24 passed, 0 failed, 1 ignored)
