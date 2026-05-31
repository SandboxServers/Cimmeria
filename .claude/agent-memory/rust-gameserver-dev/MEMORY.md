# Rust Gameserver Dev Memory

## Phase −0.5 triage status (2026-05-13)

- [build-environment.md](build-environment.md) — **[PROMOTE → user-local feedback]** — Windows linker workaround for the repo's hardcoded WSL rust-lld path. Not bible-relevant; this is operational guidance specific to this contributor's host setup. Keep in memory as user-feedback; will not promote to `docs/spec/`.

### Inline-content section status

- **Project Context section** — **[DISCARD]** — `feature/rust-rewrite` branch reference is stale (Rust lives on `main`). The file-path list cites `crates/services/src/auth.rs` and `mercury_ext.rs` — both have since become directories. Don't trust this section; re-derive from the current crate layout. The C++ reference path was rewritten to `deprecated/cpp/src/baseapp/mercury/sgw/` in the mechanical pass.
- **Audit Findings link to audit-findings.md** — **[DISCARD]** — `audit-findings.md` does not exist (broken link). Drop the reference.
- **Critical Bug (FIXED): RESOURCE_FRAGMENT u32→u16** — **[PROMOTE → spec.protocol.mercury-wire-format §"InterfaceElement length encoding"]** — V5-confirmed against `findings/mercury-protocol-internals.md`. The fix has shipped and the regression test guards it; the bug history is bible-section-4-vs-section-5 material.
- **Packet Layout Gotcha (build_outgoing)** — **[PROMOTE → spec.protocol.mercury-wire-format §"packet layout"]** — section-5 implementation detail worth recording in the chapter.
- **Entity Class IDs** — **[PROMOTE → spec.engine.entity-description-parse-chain §"class ID assignment"]** — V5-confirmable from entities.xml; the 8-entry table is canonical.
- **Account Method Indices** — **[PROMOTE → spec.engine.entity-description-parse-chain §"method index assignment"]** — same chapter as the SGWPlayer 157-method table (in `bigworld-engine-advisor/sgwplayer-method-index-table.md`); Account is a separate entity with its own 8-index inheritance from ClientCache. Worth a row in the chapter's appendix.
- **Wire Format Notes** — **[PROMOTE → spec.protocol.mercury-wire-format]** — V5-confirmed. The rotation-swap claim should be re-cross-referenced against `bigworld-engine-advisor/protocol-comparison.md`'s flagged-for-verification status before promoting to canon.
- **C++ Account.py createCharacter Flow** — **[RE-VERIFY]** — the "Rust version is missing most of this" claim is **OUT OF DATE**. Current `crates/services/src/base/character_create.rs` persists alignment, archetype, gender, bodyset, world_id, abilities, components, skin_color_id. Re-snapshot before promoting; the python-side flow description still maps cleanly to `spec.player.character-creation` section 3.
- **C++ Account.py requestCharacterVisuals Flow** — **[RE-VERIFY]** — the Rust-divergence claim (primaryTint=0, secondaryTint=0, raw skin_color_id index) needs re-verification against current `crates/services/src/base/world_entry_appearance.rs` or wherever character-visuals now lives. V5 `findings/character-creation-pipeline.md` confirms the canonical SkinTintColorID resolution to `0xRRGGBB00` packed uint32 — so the python flow is bible-ready, but the Rust gap claim may have been closed.

## Build Environment

- See [build-environment.md](build-environment.md) — repo `.cargo/config.toml` hardcodes another user's rust-lld path; need `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS` override.

## Working Environment

- [concurrent-claude-sessions.md](concurrent-claude-sessions.md) — when other Claude sessions are running on the same repo, use a git worktree under `.claude/worktrees/<slug>/` for branch isolation. Junction-link `external/` into the worktree (`external/` is gitignored).

## Wire-format gotchas

- [read-wstring-offset-semantic.md](read-wstring-offset-semantic.md) — `read_wstring` returns BYTES CONSUMED, not the new absolute offset; chain with `offset += n`, never `offset = n`.
