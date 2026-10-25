# Launcher: Linux and Steam Deck Play through Proton

Campaign ledger. Packets: [work-packets.md](work-packets.md) (rules, contract,
waves 0-2), [work-packets-w1b.md](work-packets-w1b.md) (wave 1B, install
quality), [work-packets-w2b.md](work-packets-w2b.md) (wave 2 additions after
PP-S1) and [work-packets-w3.md](work-packets-w3.md) (waves 3-4). PP-S1 ruling:
[worknotes/PP-S1-ruling.md](worknotes/PP-S1-ruling.md). Resume
card: [RESUME.md](RESUME.md). Host: **linux** (Steam Deck), every packet.
Spike brief this replaces: [launcher-linux-steamdeck/assessment.md](../launcher-linux-steamdeck/assessment.md).

## Purpose

The Linux AppImage installs the game (#2020, launcher-linux-install-telemetry)
but cannot play it: Play, prerequisites, Repair and Update answer
`platform_unavailable`. The owner's goal was always a launcher that **plays**
on Linux and the Steam Deck. This campaign adds Play through an installed
Proton, with the client-patches DLL and the opt-in game-telemetry DLL injected
by the same i686 launch worker that Windows and macOS use. Play comes first.
Repair, Update and self-update on Linux are follow-ups (§ Follow-ups).

Owner rules for this campaign: no Wine for cabinet expansion on Linux (done
natively already); **no new PRs until the end, one final PR**; server
authoritative, no client patch beyond what macOS already ships.

## Findings

Paths: `D/` = `crates/launcher/desktop/`, `E/` = `D/engine/src/`, `S/` = `D/shell/src/`. Read at `origin/main` `0a25ef5d4` (2026-10-25).

| ID | Finding | Evidence | Packet |
|---|---|---|---|
| F-01 | Play is blocked on Linux in three layers: the shell never counts a Linux install as playable (`S/host/launch/mod.rs:209-213`), a Linux build has no pinned resources so `bundled()` is `None` and the view says "Play unavailable: this build is missing verified game launch resources" (`S/host/launch/resources.rs`, `D/frontend/src/launch-view.ts:51`), and the engine refuses `Native` off Windows (`E/storage/launch/mod.rs:196-209`, `E/storage/launch/worker.rs:380-416`). | Explore map | PP-06, PP-09 |
| F-02 | Proton's `proton run` and `waitforexitandrun` start the target through Proton's builtin `steam.exe` shim. `proton runinprefix <exe>` runs `wine <exe>` directly with Proton's environment, and stdin/stdout are inherited (`run_proc`, `log_file = None`). `runinprefix` does **not** create or upgrade the prefix (`init_session(False)`). | `proton` script of Proton Experimental 11.0, lines 1531, 2137-2140, 2160-2172, 2208-2219 | D-PP1, D-PP2, PP-S1 |
| F-03 | `PROTON_LOG` opens a log file and sends the child's stdout there, which would cut the worker's JSON pipe. | `proton` lines 1705, 1749-1787 | D-PP10 |
| F-04 | This Deck: SteamOS (holo), Steam client running, Steam root `~/.local/share/Steam` (`~/.steam/root` and `~/.steam/steam` link to it), library also on `/run/media/mmcblk0p1`. Valve Proton 7.0, 8.0, 9.0 (Beta) and Experimental (`experimental-11.0-20261014`); GE-Proton7-38 and GE-Proton11-11 in `compatibilitytools.d`; SteamLinuxRuntime soldier, sniper and 4; ProtonUp-Qt (Flatpak); `gamescope`; no `umu-run`, no system Wine, no Flatpak Steam. | Read-only inspection, 2026-10-25 | PP-01, PP-S1 |
| F-05 | Each Proton's `toolmanifest.vdf` names its runtime: Proton 11 and GE-Proton11 need appid 4183110 (SteamLinuxRuntime_4), Proton 9 needs sniper, GE-Proton7 needs soldier. Proton 7.0 and 8.0 keep their files in `dist/` (from `proton_dist.tar`), newer ones in `files/`. Proton 11 and GE-Proton11 ship no `wine64`: 32-bit code runs through new WoW64. | `toolmanifest.vdf`, directory listings | D-PP3, PP-S1 |
| F-06 | The launch supervisor clears the environment, spawns one host process, writes the request to its stdin and reads JSON lines from its stdout; lifetime is the host PID plus `process_exited`. A launcher that forks or detaches breaks it. | `E/storage/launch/supervisor.rs` | D-PP9 |
| F-07 | macOS installs only the PhysX 7.11.13 MSI into its prefix, through the i686 `cimmeria-prerequisite-worker.exe` and the portable `prerequisites::supervisor`. VC80, `d3dx9_40` and `xinput1_3` come from Wine builtins. | `D/docs/prerequisites.md` 162-246, `E/mac_wine/prerequisites/` | PP-08 |
| F-08 | Windows embeds the worker and both DLLs in the shell binary (`D/shell/build_payload.rs`, gated on `CARGO_CFG_TARGET_OS == "windows"`) and extracts them to `<app-local-data>/payloads/<digest>/windows/` at startup with SHA-256 pins, leases and repair (`S/payload_startup.rs`, `E/embedded_payload/`). | `launch.md` § Embedded Windows Play resources | PP-02 |
| F-09 | `build-desktop-linux.sh:142-145` fails when the AppDir holds a loose `.exe` or `.dll`; `release.md` 279-306 and 432-433 say Linux carries no Windows pieces and has no Play. The Linux CI job (`launcher-desktop-linux.yml`, ubuntu-22.04) has no Windows artifact input. | Explore map | PP-03 |
| F-10 | The cabinets spell `Working\binaries` in lower case. Linux is case-sensitive, so a launcher that writes `Working/Binaries/sessions/current-session.json` creates a second folder; Wine then sees two folders that differ only in case. `cimmeria_patchset::resolve_existing_case(dest, rel)` is the resolver install already uses. | `crates/launcher-core/src/unpack/zip.rs:85-89`, `D/README.md:76-79` | PP-07, PP-11 |
| F-11 | Game-telemetry tags are `["desktop-launcher", <os>, "wine"/"native"]` (`E/storage/launch/telemetry.rs`). The server has no closed set of tag values. | `git grep` of `crates/server`, `crates/admin-api` | PP-11 |
| F-12 | The frontend learns what works from status fields (`resources_available`, `installation_id`, `setup_complete`, `prerequisites_missing`); there is no capabilities command. `prerequisites_missing` copy is DirectX/PhysX only. | `D/frontend/src/launch-view.ts`, `composition.ts:12` | PP-09, PP-12 |
| F-13 | Display: the profile lookup and `DisplayOptions::effective` branch on `cfg!(target_os = "macos")` (`E/storage/display_options.rs:74-117`, `E/storage/launch/display.rs:136`); Linux gets the Windows branch, which looks in the real Documents folder, not in the prefix. | Explore map | PP-11 |
| F-14 | Sibling README row U1 ("install … Play is offered, and the game starts to the login screen") cannot pass: Linux Play does not exist. | `../launcher-linux-install-telemetry/README.md:139` | PP-13 |
| F-15 | `umu-launcher` runs `proton waitforexitandrun`, so it has the `steam.exe` shim of F-02 as well, and it is not on SteamOS. | F-02, F-04 | D-PP1 |
| F-16 | Bare `proton runinprefix` carries the worker's stdio on GE-Proton11-11, Experimental, 9.0 and 11.0; wrapped in `_v2-entry-point` it exits 1 with no stdout (SLR_4 and sniper). GE11 reaches login, patches and telemetry DLLs, and world entry. F-04 correction: the GE folder is `GE-Proton11-11-x86_64`; the SD card also holds Proton 10.0 and 11.0. | [worknotes/PP-S1.md](worknotes/PP-S1.md) steps 1-6 | D-PP1..D-PP3 |
| F-17 | Wine's builtin `d3dx9_40` crashes SGW (`ShaderRef != NULL`, `SGW.exe!0x00EC6F9D`). The game's own `d3dx9_40.dll`, plus `xinput1_3.dll`, set native, reaches login. `DXSETUP.exe` exits 247 under Proton; Wine's `expand.exe` fails on the cab. | PP-S1 step 3b | D-PP20, PP-16 |
| F-18 | `Nov2008_d3dx9_40_x86.cab` is LZX (window 21); `APR2007_xinput_x86.cab` is MSZIP. cab_portable refuses LZX (by design, for the install set) and the tree has no LZX decoder; it has no single-member extract either (`expand_chain` only). | `7z l -slt`; `LC/unpack/cab_portable/` | D-PP21, PP-15 |
| F-19 | With the game's first-run profile (`windowedMode` false) the window stays black until a focus change; with the launcher's windowed profile it draws at once. The game writes the profile on its first run, and `display::saved_options`/`engine_ini` only edit existing files, so a fresh prefix has nothing to edit. Linux takes the Windows `native_profile` branch (`E/storage/launch/display.rs:136-145`). | PP-S1 step 3b | D-PP22, PP-17, PP-S2 |
| F-20 | The prerequisite worker answers `failed/probe` on GE11 and Experimental although the MSI returned 0 and the game then initialises PhysX. `failed/probe` comes only from four calls in `runtime-probe/src/windows.rs:81-118` (canonicalize, the `SGW.exe` check, `SetDefaultDllDirectories`, `AddDllDirectory`); `prerequisite/windows.rs:116` drops the detail and the supervisor nulls stderr. | PP-S1 step 7; Explore read | D-PP7, PP-08, PP-S2, PP-08W |
| F-21 | Deck install test (2026-10-25, 8m33s): the local copy is shown as "Downloading" (`JobProgress` drops the label, `S/host/install/contract.rs:254-274`); the full re-read (`LC/install.rs:539-570`) and the RAR listing pass (`LC/unpack/rar.rs:63-80`) report nothing; RAR and cabinet stages share one per-file `extraction` percent; cabinet folders expand one at a time (`cab_portable/mod.rs:107-109`); an archive pick or its refusal logs no `launcher.install` line (the dev-key refusal logs on `launcher.game_update`); upload lines carry no endpoint (`E/storage/log_upload/ingest.rs:409-436`); the archive flow installs into the Settings folder without showing it. | Owner test, Explore map | PP-20..PP-25 |
| F-22 | World entry on GE11 took about 2-3 minutes with `No player` minimap errors until the player arrived; there is no Windows baseline. | PP-S1 step 6 | FU-7 |
| F-23 | The 2-minute pass after the local copy is `verify_and_unpack`'s `verify_sha256` → `hash_file` over the staged file (`LC/install.rs` 394-429, 539-570), called from `LC/install_seed.rs` `apply` right after `Unpacking seed into …` (line 52). It only computes and compares a SHA-256; extraction opens and parses the RAR itself and nothing uses the digest. The copy before it (`seed_source.rs` `copy_to`, 258-287) already re-checks source identity and size, hashes the very bytes it writes, fsyncs and renames only on a match. Downloads and patches have no other hash. | `origin/main` `85cc212f3` read (OD-3 amend) | D-PP26, PP-20 |

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-PP1 | Start the launch worker with the selected Proton's own script: `<proton root>/proton runinprefix <Z:\ path of cimmeria-launch-worker.exe>`. Do not use `run`/`waitforexitandrun`, umu-launcher, or the Proton's `wine` binary directly. | Decided, **confirmed by PP-S1** (F-16) | F-02: stdio reaches the worker, there is no `steam.exe` shim, and Proton computes `WINEDLLPATH`, `LD_LIBRARY_PATH` and the DXVK overrides for both the `dist/` and `files/` layouts (F-05). Calling `wine` directly would copy Proton's environment logic and break across versions. umu: F-15. |
| D-PP2 | Because `runinprefix` does not create the prefix, the launcher runs one setup command, `proton run wineboot.exe`, when the prefix stamp is missing or names another Proton version. | Decided, **confirmed by PP-S1** (target `wineboot.exe`) | F-02, F-16 |
| D-PP3 | **Always bare**: run `<root>/proton` on the host; never wrap in `_v2-entry-point`. `ProtonInstall.runtime` is discovered and logged only. | **Re-specced by the PP-S1 ruling** | F-16: the container returns exit 1 and no stdout on every Proton. Container stdio is FU-6. |
| D-PP4 | Use only a Proton that is already installed; no download. Look in every Steam library's `steamapps/common/Proton*`, in `<steam>/compatibilitytools.d/*` and in `/usr/share/steam/compatibilitytools.d/*`. Default order: newest GE-Proton, then Proton Experimental, then the newest numbered Valve Proton. The player can pick another one in Settings, from a native list (ids, never paths). | Decided; GE first is tested (PP-S1); the rest stays provisional until PP-S2 step C (no packet waits) | F-04, F-16. Downloading a runtime is OD-1. |
| D-PP5 | One private prefix per launcher state root: `STEAM_COMPAT_DATA_PATH = <state_root>/proton/compatdata` (Proton creates `pfx/` in it), plus the stamp `<state_root>/proton/compatdata/cimmeria-setup.json`. Never Steam's `steamapps/compatdata`. | Decided | A non-Steam program has no stable appid, and Steam may clean up compatdata it does not own. |
| D-PP6 | DXVK `d3d9` comes from the Proton prefix. Linux has no `graphics/d3d9.dll` resource: `Resources.graphics` stays `None`. | Decided | Proton installs DXVK into the prefix during setup (`proton` lines 1168-1181). |
| D-PP7 | The only prefix prerequisite is the PhysX 7.11.13 MSI, installed by the existing `cimmeria-prerequisite-worker.exe` through `runinprefix` on the first Play (and after a failed attempt), inside Launch preparation. There is no new operation kind. | Decided, **confirmed by PP-S1: PhysX is needed**. Acceptance stays strict (`probed` + `initialized_and_released` only); the probe fails under Proton, so PP-S2 step A then PP-08W fix it | F-07, F-20. One Play press does everything, with a `Preparing` observation and visible copy (PP-12). |
| D-PP8 | Carry the Windows pieces in the AppImage the way Windows does: **embedded in the shell binary** with compile-time SHA-256 pins, extracted to `<app-local-data>/payloads/<digest>/windows/`. Four pieces: launch worker, prerequisite worker, client-patches DLL and (optional) telemetry DLL. There is no first-run download. The loose-file guard in `build-desktop-linux.sh` stays. | Decided | F-08: one mechanism, already pinned, leased and self-repairing. The files land under `$HOME`, which pressure-vessel shares, whereas the AppImage mount under `/tmp/.mount_*` changes every start. Embedded bytes are not loose files, so the guard still refuses anything else. |
| D-PP9 | Keep the supervisor model: the host PID is the spawned process (the runtime entry point or the `proton` script), and the worker's JSON arrives on its stdout. Never signal a guest PID; never `wineserver -k` the game. The Linux environment starts empty and copies only an allow-list (contract C-4). | Decided | F-06 |
| D-PP10 | Never set `PROTON_LOG`; drop it from the allow-list. Logs: stderr to `logs/game/<op>/proton.log` (the existing 4 MiB cap) and DXVK's log through `DXVK_LOG_PATH`. | Decided | F-03 |
| D-PP11 | Linux game-telemetry tags are `["desktop-launcher", "linux", "proton"]`. Opt-in, the https-only rule (LI-09) and the session loop are unchanged. | Decided | F-11: no server change. |
| D-PP12 | Every game-folder path the launcher writes or tails on Linux goes through `cimmeria_patchset::resolve_existing_case`. | Decided | F-10 |
| D-PP13 | Display on Linux: **windowed only** (as macOS) until PP-S2 step F shows fullscreen renders; the frame cap is offered as on a Mac (`DXVK_FRAME_RATE`); the client profile lives in the prefix (`pfx/drive_c/users/steamuser/Documents/My Games/Firesky/SGWGame/`). An empty display list on Linux is accepted for now (FU-4). | Decided, **amended by the PP-S1 ruling** | F-13, F-19: `windowedMode` false gave a black window. |
| D-PP14 | `linux_proton/paths.rs` holds its own `guest()` (host path to `Z:\…`), copied from `mac_wine/paths.rs` with a comment naming its twin. | Decided | The macOS module cannot be compiled on this host. FU-5 merges the two. |
| D-PP15 | The selected Proton travels in `launch::Resources.proton: Option<ProtonSelection>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`). Admission and dispatch both re-check that the Proton still exists and that its `version` file is unchanged. | Decided | Admission already takes `Resources`, and an absent field keeps old plan digests. |
| D-PP16 | Two new failure codes: `proton_missing` (no usable Proton, or the selected one is gone or changed) and `proton_setup_failed` (prefix setup or PhysX failed). | Decided | PD-04 keeps one taxonomy with copy. |
| D-PP17 | **Ship:** one integration branch, `launcher-linux-play-proton/integration`, and **one final PR** after wave 4 (owner rule). Packets merge into it with `merge-noff.sh` in the order of § Packets, and each wave review runs there. Before the PR, `ops` runs the Windows and macOS launcher CI on the branch (`full-ci`), because this host cannot compile their `cfg` code. | Decided | Owner rule; F-01's gates are shared with macOS and Windows. |
| D-PP18 | Repair, game Update, prerequisite UI beyond PhysX, and launcher self-update stay `platform_unavailable` on Linux. | Decided | Play does not need them. FU-1..FU-3. |
| D-PP19 | No gamescope or Game Mode code in this campaign. Game Mode is a UAT row (U6). | Decided | A non-Steam shortcut runs the whole process tree under gamescope and Steam Input. Support level: OD-2. |
| D-PP20 | Linux Play puts the game's own `d3dx9_40.dll` and `xinput1_3.dll` into `pfx/drive_c/windows/syswow64/`, extracted natively from `.cimmeria-prerequisites/DX9.0c/` with size and SHA-256 pins on the DLL bytes, checked on every Play; the game's `WINEDLLOVERRIDES` gains `d3dx9_40=n;xinput1_3=n`. No `DXSETUP.exe`, no Wine `expand.exe`, no Microsoft bytes in the AppImage. | Decided (PP-S1 ruling) | F-17. Every Play re-checks, because a Proton prefix upgrade may rewrite `syswow64`. |
| D-PP21 | Add the pure-Rust `lzxd` crate to `cimmeria-launcher-core` for a single-member cab extract (C-8). The install path stays MSZIP/stored-only, and its LZX refusal and test stay. | Decided (PP-S1 ruling) | F-18. An LZX decoder by hand is a large, error-prone packet; the DLL pins catch a wrong decode. |
| D-PP22 | The display step writes the windowed profile into the prefix before every Linux Play (PP-17). The rule for a prefix with no profile yet comes from PP-S2 step B. | Decided (PP-S1 ruling); the first-Play rule waits on PP-S2 | F-19 |
| D-PP23 | Install progress names the real phase (C-7: `copy`, `check`, extraction stage `archive`/`cabinets`) with a rate and ETA for each; the forwarded summary schema (`TimedPhase`) is unchanged. | Decided (owner-approved follow-up) | F-21 |
| D-PP24 | Independent cabinet folders expand on up to 4 threads, assigned deterministically by compressed size, with positional reads on the one set of cabinet handles; one folder's blocks stay in order (MSZIP history). First error or cancel stops all workers. | Decided (owner said yes to parallel decompression) | F-21 |
| D-PP25 | The archive flow shows the game folder with a `Change…` button beside the archive button (the Settings helper, shared); no second dialog. | Decided | F-21 |
| D-PP26 | A local-archive install unpacks the staged copy without hashing it again (`ArchiveCheck::HashedWhileCopied`, C-7b); downloads, the seed cache and patches keep their hash before unpack, and the unused native-backend branch is unchanged. `seed_unpack_started check=…` logs which check ran. **One verification is removed:** the read-back of the staged copy on disk after the rename, which would have caught a storage fault in the written bytes or a change to the staged file in the launcher's folder before unpack. What still guards it: the pick-time hash, the copy's own identity/size/hash check of the bytes it writes, `sync_all`, and UnRAR's per-member CRC during extraction (library behaviour, no repo test). | Decided (OD-3, owner 2026-10-25) | F-23. Saves about 2 minutes on the Deck. PP-20 step 9. |
| OD-3 | The install reads the archive three times (pick-time verify, the hashed copy, and the re-read before unpack, about 2 min on the Deck). Drop the third read? | **Decided by the owner 2026-10-25: drop it** (D-PP26) | The archive is hash-checked before and during the copy. PP-20 removes the read instead of labelling it. |
| OD-1 | When no Proton is installed, should the launcher download a pinned GE-Proton build (about 500 MB, GitHub release, SHA-256 pin), or tell the player to install one through Steam or ProtonUp-Qt? | **Decided by the owner 2026-10-25: no download**; Play says how to install one | Third-party runtime redistribution and download size. This campaign ships the "tell the player" path (PP-12 copy); a download is a follow-up packet. It blocks no packet. |
| OD-2 | Is Desktop Mode enough for the first Linux Play release, or must Steam Game Mode (the launcher as a non-Steam game under gamescope) pass U6 before the final PR merges? | **Decided by the owner 2026-10-25: Desktop Mode gates the merge**; Game Mode (U6) is a follow-up | It decides the player-guide wording (PP-13) and whether U6 gates the merge. It blocks no code packet. |

## Packets

| Packet | Wave | Title | Agent | Status |
|---|---|---|---|---|
| PP-S1 | 0 | Proton Play spike on the Deck | hands-on (owner present) | Done ([ruling](worknotes/PP-S1-ruling.md)) |
| PP-01 | 1 | Proton discovery and ranking | packet-coder | Ready |
| PP-02 | 1 | Embed the Windows pieces in the Linux shell | rust-gameserver-dev | Ready |
| PP-03 | 1 | Linux release job carries the pinned pieces | packet-coder | BlockedDependency (PP-02) |
| PP-04 | 1 | Proton prefix, stamp and environment | packet-coder | BlockedDependency (PP-01) |
| PP-20 | 1B | Install progress names the real phase; no re-read of a copied archive | packet-coder | Ready |
| PP-21 | 1B | Install view copy, phase lines, rate and ETA | packet-coder | BlockedDependency (PP-20) |
| PP-22 | 1B | Install log lines: phase bytes, archive pick | packet-coder | BlockedDependency (PP-20) |
| PP-23 | 1B | Archive flow shows and changes the game folder | packet-coder | BlockedDependency (PP-21) |
| PP-24 | 1B | Upload log lines name scheme and host | packet-coder | Ready |
| PP-25 | 1B | Parallel expansion of cabinet folders | rust-gameserver-dev | Ready |
| PP-S2 | 2 | Second Deck spike (probe, first profile, other Protons) | hands-on (owner present) | Ready |
| PP-05 | 2 | Proton invocation and prefix setup (bare) | packet-coder | BlockedDependency (PP-04) |
| PP-06 | 2 | Linux launch admission and failure codes | rust-gameserver-dev | BlockedDependency (PP-05) |
| PP-07 | 2 | Proton launch preparation | rust-gameserver-dev | BlockedDependency (PP-06) |
| PP-15 | 2 | Single-member cab extract with LZX | rust-gameserver-dev | BlockedDependency (PP-20, PP-25) |
| PP-16 | 2 | Native DirectX in the Proton prefix | packet-coder | BlockedDependency (PP-07, PP-15) |
| PP-08 | 2 | PhysX in the Proton prefix | packet-coder | BlockedDependency (PP-05, PP-07) |
| PP-08W | 2 | Prerequisite worker probe under Proton | rust-gameserver-dev | BlockedDependency (PP-S2 ruling) |
| PP-09 | 2 | Shell: Linux Play status and resources | packet-coder | BlockedDependency (PP-02, PP-06) |
| PP-10 | 3 | Proton picker: preference and IPC | packet-coder | BlockedDependency (wave 2) |
| PP-11 | 3 | Linux telemetry tags and case | packet-coder | BlockedDependency (wave 2) |
| PP-17 | 3 | Linux display profile in the prefix | packet-coder | BlockedDependency (PP-07, PP-S2 ruling) |
| PP-12 | 3 | Frontend copy, picker and REPL UAT | packet-coder | BlockedDependency (PP-10) |
| PP-13 | 3 | Docs | documentation-writer | BlockedDependency (PP-10..PP-12) |
| PP-U | 4 | Deck UAT | owner + hands-on | BlockedDependency (wave 3) |
| PP-14 | 4 | Close-out | documentation-writer | BlockedDependency (PP-U) |

Merge order into the integration branch: W1 PP-01, PP-04, PP-02, PP-03; W1B
(beside W1 and W2) PP-24, PP-25, PP-20, PP-22, PP-21, PP-23; W2 PP-05, PP-06,
PP-07, PP-15, PP-16, PP-08, PP-09, then PP-08W; W3 PP-10, PP-11, PP-17, PP-12,
PP-13; W4 PP-14. PP-15 is cut after PP-20 and PP-25 have merged. PP-S1 and
PP-S2 commit only worknotes. Packets: W1B [work-packets-w1b.md](work-packets-w1b.md),
W2 additions [work-packets-w2b.md](work-packets-w2b.md). Status values: Ready / BlockedDependency /
BlockedDecision / Writing / Review / Integrated / UATPending / Done.

## UAT (PP-U, doc-update-map row 57)

| Row | Step and pass condition | Who |
|---|---|---|
| U1 | Build the AppImage with the pinned pieces; `--verify-embedded-payload` passes; no loose `.exe`/`.dll` in the AppDir. | hands-on |
| U2 | Desktop Mode, no Proton selected: the Play area names Proton and how to install it on the first view. With Proton installed, Settings lists every Proton of F-04 and the default of D-PP4. | owner |
| U3 | Install from `client.rar` (sibling U1, first half) in a **fresh launcher state root** (no prefix), press Play once: "Preparing Proton" shows at once; one worker, one `SGW.exe`; the login screen **draws without a focus change** on the first Play and on the second; `syswow64/d3dx9_40.dll` matches its pin. Press Play again while running: refused, no second game. | owner |
| U8 | Install quality (W1B): installing from `client.rar` shows "Copying your archive…", "Reading the archive index…", "Unpacking the archive…" and "Expanding game files…", each with a rate and ETA where it has one, and **no** "Checking the download…"; the log has `seed_unpack_started check="hashed_while_copied"` and the first extraction event follows it within seconds, not about 2 minutes (D-PP26); the game-folder line shows the target and `Change…` works on the first press; the log has `install_phase_finished` per phase with bytes, `seed_archive_picked`, and `log_upload_post scheme="https"` after an upload; the cabinet stage is faster than 3m46s. | owner |
| U4 | Log in, enter a world, open the Black Market (a patched feature). Quit normally: `process_exited`, Play is enabled again. | owner |
| U5 | Opt in to game diagnostics, play 2 minutes: `game-telemetry-status.json` says `attached`, `cimmeria-client-telemetry.log` exists beside `SGW.exe`, and rows with tag `proton` arrive over https (the `telemetry-triage` skill). | owner, then a triage agent |
| U6 | Game Mode: add the AppImage as a non-Steam game, Play reaches the login screen, the controller drives the launcher. Gates the merge only if OD-2 says so. | owner |
| U7 | `npm run uat:launch` and `npm run uat:proton` pass (REPL logic UAT, PP-12). | packet-coder |

## Follow-ups (not in this campaign)

- FU-1 Repair and game Update on Linux (`S/host/repair/mod.rs:30-46`, `S/host/game_update/dispatch.rs:65,167-183`).
- FU-2 Launcher self-update on Linux (`linux-x86_64` feed key, `E/storage/updater/`).
- FU-3 OD-1's managed Proton download, if the owner wants it.
- FU-4 A Linux display list (X11/Wayland) for window sizes and placement.
- FU-5 One `guest()` for macOS and Linux (D-PP14), on a host that compiles both.
- FU-6 Why `_v2-entry-point --verb=waitforexitandrun -- proton runinprefix` loses stdio (F-16); only needed for distros where bare Proton lacks libraries.
- FU-7 World entry took 2-3 minutes on GE11 (F-22): take a Windows baseline on the same server first.
- FU-8 Closed: OD-3 said yes; PP-20 drops the read (D-PP26). Left open, owner's call: a reused seed cache is hashed twice too (`install_seed.rs` `fetch_seed` ~127, then `verify_and_unpack`); D-PP26 does not cover it.
- FU-9 The `client-telemetry-build` artifact named by PP-S1 does not exist (its workflow uploads nothing); PP-03 takes the telemetry DLL from `windows-runtime-probe`.

## Close-out

PP-14: the campaign review loop, `docs/project-status.md` once,
`docs/guides/unified-uat.md` rows U2-U6, the sibling's U1 row reconciled
(PP-13), empty `.claude/state/active-campaign`, retire the packet worktrees
with `rm-worktree.sh --merged-into <PR>`.
