# Project Rules and Gotchas

Decisions the maintainers have already made, and traps contributors have already hit. None of this is derivable from reading the code, which is why it is written down. Read it before proposing an approach. If a rule here blocks something you think is right, say so in the PR or issue; do not work around it.

Each entry says what to do and why. Build commands, the pre-PR checklist, the test policy, and the doc-update map stay in [`CLAUDE.md`](../../CLAUDE.md) and [`TESTING.md`](../../TESTING.md) and are not repeated here.

## Evidence and reverse engineering

- **Check `docs/` before you investigate anything.** The docs cover nearly every system: protocol, gameplay, engine internals, RE findings. Search them before opening Ghidra, counting `.def` entries, or guessing at behaviour. Where to look is in [`domain.md`](domain.md).
- **Method indices come from the dispatch tables**, not from counting `.def` files by hand: `docs/protocol/sgwplayer-base-method-dispatch-table.md`, `cell-method-dispatch-table.md`, `client-method-dispatch-table.md`, `message-dispatch-table.md`. If an index is missing, derive it, then add it to the table.
- **A ticket is a claim, not evidence.** Reconcile it against `docs/protocol/` and the findings before acting. See "When sources disagree" in [`domain.md`](domain.md).
- **Quote the whole decompiled function, not the block that supports your theory.** The costliest wrong claim in this repo's history came from a snippet that stopped one block short of the code that contradicted it.
- **Cite a Ghidra address for every binary claim**, and tag confidence per [`docs/reverse-engineering/evidence-standards.md`](../reverse-engineering/evidence-standards.md).
- **`deprecated/` shows original intent, not client truth.** Use `deprecated/python/` and the `.def` files to find features that were designed but never wired. Do not copy a legacy behaviour that makes the client feel broken.

## Protocol traps

- **Entity type IDs on the wire are the client's clientIndex, not the `entities.xml` row number.** The client skips every entity whose `.def` is `<ServerOnly/>` when numbering. `SGWPlayer = 0x02`, `SGWGmPlayer = 0x03`, `SGWMob = 0x04`, `Account = 0x07`. `SGWBlackMarket` sits before `Account` in the file but is server-only, so `Account` is **not** `0x08`. Details and addresses: [`docs/protocol/client-verified-wire-formats.md`](../protocol/client-verified-wire-formats.md) "Entity Class IDs".
- **`onPlayerTeleport` (client method 116) is a streaming-load hint, not a move.** An authoritative reposition is `BASEMSG_FORCED_POSITION` plus an AoI refresh. A teleport built on 116 snaps back. Ask `movement-teleport-advisor`.
- **Item property ids are easy to transpose.** propId 3 is `AmmoTypeId`; propId 7 is `AccessLevel`. Check `entities/defs/` before sending a property update.
- **Do not add `remove_item` next to a `UseInventoryItem`-driven chain.** The base service already consumes the item through `UseInventoryItem → ItemUsed`; the extra action double-consumes from any stack larger than one.
- **The server is authoritative, and that is an opportunity.** The client renders whatever create, destroy, position, and entity-method stream it is sent. Rules about who sees whom, validation, presence, NPC behaviour, and content can all change server-side with messages the client already speaks.

## Scoping a feature: "free" or "needs a client patch"

Classify before you design.

- **Free:** server-authoritative, reuses existing messages. AoI rules, validation and anti-cheat, missions and content, NPC behaviour, finishing features the original `.def` files declare but the original server never wired (a `BASE` property plus an `Exposed` method with a stub implementation is the usual sign). Prefer these.
- **Needs a client patch:** new opcodes, wire-crypto changes, UI the client does not have. Real cost and real risk. Needs a maintainer decision before any work starts.

## Client UI feedback

**The player must get visible acknowledgement on the first press of any button.** A feature that matches the original server's behaviour but looks dead to the player is not done. When a server-side feature is tied to a client UI element (button, indicator, cursor state, highlight), send the visible state change on the first interaction, and fill in any lazily computed state afterwards. Worked example: [`docs/protocol/auto-cycle-button.md`](../protocol/auto-cycle-button.md).

## GM and developer commands

- **GM gameplay commands use the client's native `/` console.** The client consumes every `/` command locally and emits `gm*` cell-method calls, so `/` lines never reach the server as chat. **Do not build a server-side `/`-chat parser.** It cannot work against the real client.
- **The client learns a player is a GM only from the entity class.** `accessLevel` is `CELL_PRIVATE` and never replicated, so GMs enter the world as `SGWGmPlayer` (`0x03`). The server still authorises every GM call against its own `access_level`, never a client-asserted value. See [`docs/architecture/gm-cell-method-gating.md`](../architecture/gm-cell-method-gating.md).
- **Commands with no native slash binding go through the `.`-console** (`crates/services/src/cell/console/`), which intercepts `.`-prefixed say-chat from GMs. See [`docs/architecture/dev-console-channel.md`](../architecture/dev-console-channel.md).
- **Authoring commands (`savespawn`, `path_*`) emit seed SQL for a human to commit.** They apply in memory and to the live database so the GM sees the result, but the durable artifact is SQL for `db/resources/`, emitted through one choke point (`cell/console/seed.rs`) to the server log, never shown in game. Deploys rebuild the database from seeds, so a live-only write is lost.

## Database and content

- **Seed files under `db/resources/` are the source of truth.** To change seeded data (abilities, effects, NVPs, missions, spawns), edit the existing `INSERT` in `db/resources/<area>/Seed/` or add a row there. Do not add a `db/scripts/*.sql` migration for a data change. There is no long-lived production database to migrate: every environment, including the colo, is rebuilt from seeds.
- **Ask a maintainer before adding any file under `db/scripts/`.** [`docs/guides/write-a-database-migration.md`](../guides/write-a-database-migration.md) still describes paired migrations for schema changes; the maintainers' current preference is to avoid them. Existing scripts stay as they are.
- **Do not hard-code seed ids in tests.** Assert by relationship or re-fetch the baseline. See `TESTING.md` "Don't trust seed data".
- **Auto-generated chains in `space_*_chains.sql` have converter bugs.** When a PR regenerates them, diff against the previous version. Review rules: [`.github/instructions/content-chains.instructions.md`](../../.github/instructions/content-chains.instructions.md).

## Build and CI

- **CI floats on stable Rust; the repo has no `rust-toolchain` pin.** Every job uses `dtolnay/rust-toolchain@stable`, so CI clippy is often newer than yours and `-D warnings` fails on lints your local version does not have (seen: `unnecessary_sort_by`, `ptr_arg`, `doc_lazy_continuation`). Before pushing Rust changes, run clippy on current stable: `rustup toolchain install stable --profile minimal` (or a specific version side by side), then `cargo +<version> clippy -p <crates> --all-targets -- -D warnings`. Clippy stops at the first failing target, so rerun until it exits 0.
- **Never run two `cargo` or `rustc` processes at once.** A full link can take ~47 GB. This includes agents running in parallel. See [`development-workflow.md`](development-workflow.md).
- **Iterate with `cargo check -p <crate>`.** Run workspace builds and tests once, at the end, with the exclusions in `CLAUDE.md`.
- **`cargo nextest` does not run doctests.** `cargo test --doc -p cimmeria-commands` covers the one crate that has them.
- **A PR with merge conflicts gets no CI run at all.** Merge `main` first.
- **`tools/lint-md.sh <file>` is slow** because the config glob still walks the whole tree. Calling `markdownlint-cli2 --no-globs <files>` directly finishes in seconds.

## Windows and tooling traps

- **Markdown line endings.** With `core.autocrlf=true`, docs check out as CRLF. Some docs are also stored CRLF in the index (for example `docs/protocol/client-verified-wire-formats.md`); `.gitattributes` does not normalise Markdown. A tool that rewrites such a file with LF turns a ten-line change into a whole-file diff that git will not collapse on `git add`. If a docs change shows an implausible diffstat, compare with `git diff --ignore-all-space --stat`, check what the index holds with `git ls-files --eol <file>`, and restore the original endings before staging.
- **Scripts that rewrite files must open them in binary mode** and write UTF-8 explicitly. Python's default text mode on Windows converts both the encoding and the line endings.
- **Git Bash rewrites arguments that start with `/` or contain `:.`** into Windows paths. This breaks `gh ... --body "/release"` and `git show <ref>:.github/...`. Use PowerShell, `--body-file`, or `MSYS_NO_PATHCONV=1`.
- **Revert-verification wipes uncommitted work.** Commit (or make a WIP commit) before you `git checkout` a file to prove a guard fails.
- **Removing a worktree that has an `external/` junction:** remove the junction first. See [`development-workflow.md`](development-workflow.md).

## Client assets and RE tooling

- **The game client is not in the repo.** `game/sgw/` is a placeholder. Map recon, prefab positions, navmesh extraction, and any `crates/upk-objects` or `crates/navmesh-extractor` work needs your own copy of the client; point the tool at its `SGWGame/CookedPC/` directory. Do not conclude that assets are missing because the repo does not contain them.
- **The client ships no `.nav` files.** The ones under `data/spaces/` are original server assets.
- **Breakpoints on a live, server-connected `SGW.exe` must not pause it.** A paused client stalls its network thread, Mercury keepalive times out, and the server disconnects it. In x64dbg use a logging breakpoint: `SetBreakpointCondition <addr>, 0`, `SetBreakpointLogCondition <addr>, 1`, `SetBreakpointLog <addr>, "<text with {expr} captures>"`, `SetBreakpointFastResume <addr>, 1`. Since the breakpoint never stops, encode the registers and stack slots you want into the log text.
- **Client-side instrumentation is written from scratch.** `AteraLoader.exe` and `AtreaRL.dll` are third-party. [`docs/technical/atrealoader-exe.md`](../technical/atrealoader-exe.md) and [`atrearl-loader.md`](../technical/atrearl-loader.md) are references for what to hook, not code to extend, wrap, or ship. Depending on the output of `AtreaFixASLR.bat` (an `SGW.exe` with ASLR cleared) is fine.
- **MCP servers are per-machine.** `.mcp.json` is ignored by git. Copy [`.mcp.json.example`](../../.mcp.json.example) and follow [`docs/guides/re-toolchain-setup.md`](../guides/re-toolchain-setup.md) for Ghidra, x64dbg, and the docs RAG server.

## Where project state lives

- Status and gaps: [`docs/project-status.md`](../project-status.md), [`docs/gap-analysis.md`](../gap-analysis.md). Not `docs/architecture/migration-roadmap.md`, which describes the deprecated C++ tree.
- Long-running campaigns keep their own ledgers and resume notes under `docs/analysis/` (for example `docs/analysis/castle-rebuild/handoffs/`). Read the newest resume note before continuing one.
- Playtest reports: `docs/analysis/playtests/`.
