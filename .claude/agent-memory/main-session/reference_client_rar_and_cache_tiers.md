---
name: reference_client_rar_and_cache_tiers
description: archive.org client RAR = stored RAR of the 2009 installer (SetupQA.exe + MakeCAB DATA1-4.CAB); hashes, layout, and the Cache.en-US vs SourceCache.en-us cooked-data tiers
metadata:
  type: reference
---

Learned 2026-09-27 while adding RAR seeds to the launcher (`crates/launcher/src/unpack/`).

**The client download.** archive.org item `StargateWorlds_0.8348.1.4046`, one file
`Stargate Worlds (0.8348.1.4046) (2009-06-30) (beta).rar`, 4,135,724,034 bytes,
SHA-1 `1ad25c4dbd4b8717447b0de7145f5eb52d34ab06` (archive.org's), SHA-256
`7ba97ed2cb94f86edaba17a513824ae08d0f19920583d2a3da242faf1e034f07` (computed). It is a
stored (method m0) RAR 4 holding the original installer, not a playable tree:
`SetupQA.exe`, `Data\Prerequisites\` (DX9 etc.), and a MakeCAB set `Data\DATA.INF` +
`DATA.DDF` + `DATA1-4.CAB` (1 GiB volumes, files continue across them, 5,983 files).
`DATA.INF` `[file list]` paths are the installed layout: `Common\`, `Resources\`,
`Working\binaries\SGW.exe`, `Working\SGWGame\...`. Expanding the cabinets reproduces a
live install (5,951/5,963 shared files same size; differences are the known client
patches). The launcher does it with `cabinet.dll` FDI in ~50 s. `postinstall.exe` moves
no files.

**Cooked-data tiers.** `Working\Engine\Config\GameplayEngine.ini`: `CachePath=..\SGWGame\Cache`
(writable; lands in `Documents\My Games\Firesky\SGWGame\Cache.en-US`, rewritten by the
server version push on every connect) and `SourceCachePath=..\SGWGame\SourceCache`
(read-only bundled tier, `Working\SGWGame\SourceCache.en-us`; `LaunchMisc.cpp` reads it, see
`docs/reverse-engineering/findings/cooked-data-pipeline.md`). The cabinets ship the bundled
PAKs in `Working\SGWGame\Cache.en-US`, which the client never reads; a stock install logs
`WARN common - Non-existent source archive directory: ...\SourceCache.en-US` once per
category. The launcher renames the folder after a seed (`install_layout::place_bundled_cooked_data`).
Not yet verified: whether the client *uses* the bundled tier for anything the server
doesn't push, beyond silencing the warning.

**Firesky folder.** `Documents\My Games\Firesky\SGWGame\` also holds `Config\*.ini`
(effective engine/game config), `<account>\<character>\* - Saved Vars.lua` (client-local
hotbar and UI state), `Logs\`, `CrashDumps\`, `Content\LocalShaderCache-*.upk`.

Related: [[reference_client_map_tree]], [[project_dialog_portrait_client_patch]].
