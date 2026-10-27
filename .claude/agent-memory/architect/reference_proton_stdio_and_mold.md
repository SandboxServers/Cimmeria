---
name: reference_proton_stdio_and_mold
description: Proton runinprefix keeps stdio, run goes through steam.exe, PROTON_LOG redirects stdout; mold 1.0.3 from Ubuntu 22.04 apt mislinks unrar_sys, link launchers with lld
metadata:
  type: reference
---

Learned in the launcher-linux-play-proton campaign (2026-10-25/26). Sources: `docs/analysis/launcher-linux-play-proton/worknotes/PP-S1.md` and `PP-S1-ruling.md` (F-16), `worknotes/PP-U.md` (attempts 3 and 4).

## Proton launch verbs (PP-S1, F-02, F-03, F-16)

- **Proton `runinprefix` keeps stdio.** A bare `<proton root>/proton runinprefix <Z:\ worker.exe>` passes the worker's stdin and stdout through, so the launcher reads the worker's JSON on the host process's stdout. Evidence: PP-S1 steps 2-6 on GE-Proton11-11, Experimental, 9.0 and 11.0.
- **`run` and `waitforexitandrun` go through `steam.exe`.** The worker's output does not reach the host. That is why the launcher uses `runinprefix` plus one `proton run wineboot.exe` to create the prefix (D-PP1, D-PP2).
- **`PROTON_LOG` redirects stdout.** Never set it; the launcher takes Proton's stderr into `logs/game/<op>/proton.log` instead (D-PP10).
- Wrapped in the Steam Linux Runtime `_v2-entry-point`, `runinprefix` exits 1 with no stdout (SLR_4 and sniper). Run the `proton` script on the host, bare (D-PP3).

## Linker: mold 1.0.3 breaks bundled UnRAR (PP-U-FIX, #2036)

- **mold 1.0.3 from Ubuntu 22.04 apt miscompiles/mislinks `unrar_sys`.** Linux launchers linked with it cannot unpack the release `client.rar` ("bad data" in 6 ms), including the released `launcher-20261024-c22f36c`. Debug and release builds of the same source, linked with mold 2.40 or lld, work.
- **Link launchers with lld** (`-C link-arg=-fuse-ld=lld`, lld 14 on ubuntu-22.04, g++ 11.4). The fix is in `launcher-desktop-linux.yml` and `launcher-desktop.yml` (native-engine, Linux); `build-desktop-linux.sh` runs `--self-test-rar` so a bad link fails the build.
- The compiler (g++ 11.4) and the SteamOS runtime were excluded by the PP-U matrix (runs A1-A5).
- Other workflows (`test.yml`, `wireclient.yml`, `spec-lint.yml`, `client-launch.yml`) still use mold; they ship no launcher.
