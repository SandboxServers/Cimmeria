# Owner decisions: launcher-preflight-diagnostics

Binding rulings from the project owner. These override any open question in README.md or the work packets.

## D1: The launcher installs dependencies on the user's behalf (2026-10-24)

When preflight finds a missing dependency (for example Rosetta 2 on macOS, the Wine runtime, or a Windows runtime), the launcher **installs it for the user** rather than only telling them how.

Implications for the design:

- Before installing, the launcher clearly states what is missing and what it is about to install (name, source, size where known). It doesn't install silently.
- If an install needs elevation or an OS consent prompt (for example `softwareupdate --install-rosetta --agree-to-license`, or a Windows admin prompt), the launcher triggers the prompt itself and explains why it's needed.
- If an install fails, the launcher states which dependency failed, why (the captured error), and a concrete manual fix (the exact command or download link), then stops cleanly with no stuck operation.
- After installing, it re-runs the check to confirm the dependency now works before it continues.

Source: owner chat, 2026-10-24, after the Rosetta 2 / stuck-repair incident on macOS.

## D2: The game must not open in exclusive fullscreen (2026-10-24)

On macOS under Wine, `SGW.exe` started, played audio and connected to the server, but showed only a black window. Diagnosis (2026-10-24): `SGWEngine.ini` already had `StartupFullscreen=False` at 1024x768, so the ini wasn't requesting fullscreen. The renderer is DXVK on MoltenVK (`d3d9=n`). The game window existed but was `onscreen=0` at window layer 2147483630, inside macOS's captured-display shield band (shield level 2147483628), so Wine probably treated a display as captured (a dual-display setup: 3840x1080 plus the Retina panel). The log's last line was `EULA.layout`, so the EULA screen was drawing off-screen. This is unconfirmed: `WINEDEBUG=-all` and stderr isn't captured. The launcher therefore has to prevent display capture or fullscreen under Wine, not just set the ini key, and it must capture Wine/DXVK logs per launch. The owner wants the launcher to start the game in windowed or borderless mode instead of exclusive fullscreen.

Implications for the design:

- The launcher enforces the windowed setting at launch time, through the client's display ini key or a launch argument, whichever the client honours (to be confirmed by research). It must survive the client rewriting its ini on exit, and survive repair.
- **Owner confirmed (2026-10-24):** launcher Settings gets a user-facing display mode option (fullscreen / windowed, plus borderless and resolution if the client and Wine support them) that the launcher applies on every launch. Research: `research-display-client.md` (the client's knobs) and `research-display-wine.md` (Wine/macOS knobs and launcher integration) in this folder.
- If the game process is alive but the launcher can detect no visible window, it should report that as a failure with a fix hint (ties to requirement 1/3), not stay at "Game process started".
- The architect should decide whether the default is windowed on every platform or only on macOS/Wine, and flag it if that's unclear.
- **Observed 2026-10-24 (single display):** even with `StartupFullscreen=False`, the game took over the display in exclusive fullscreen (switched the display to 1280x800, shield window on-screen). Cmd+Tab away from it froze the game. Acceptance criterion: in the default mode on macOS, the user can Cmd+Tab away and back without a freeze or black screen, on single and dual displays.

Source: owner chat, 2026-10-24, black-screen incident.

## D3: The launcher owns display settings (2026-10-24)

Evidence: `observation-windowed.md`. With `windowedMode=true` the game stayed a normal window (layer 0, 1024x768 client area) on dual displays (3840x1080 ultrawide + Retina), survived Cmd+Tab twice, and exited cleanly. Exclusive fullscreen was the cause.

- **Default:** windowed at 1024x768 on every platform where that's safe, and always on macOS.
- **The launcher is the source of truth.** It writes `windowedMode`/`resolution` (`SavedSystemOptions.xml`) and `[WinDrv.WindowsClient] StartupFullscreen/StartupResolutionX/Y` (user `SGWEngine.ini`) before **every** launch, and sets `DXVK_FORCE_WINDOWED=1` on macOS. Changes made in the game's own display options get overwritten at the next launch; the UI says so (a launcher note, and a client hint if feasible without a client patch, which needs a maintainer decision).
- The launcher settings expose windowed on/off and resolution. On macOS, "fullscreen" means borderless (`research-display-wine.md`); exclusive fullscreen is never offered there.
- Note: the client did NOT rewrite `SavedSystemOptions.xml` on a clean exit (12:31:52), but did around the 12:10 crash, so rewrite-on-exit isn't guaranteed either way. Applying settings on every launch covers both cases.

Source: owner chat, 2026-10-24 (chose option (a)).

## D4: Mac-player display extras, all in scope (2026-10-24)

The owner approved all four:

1. **Borderless + safe mode:** borderless "fullscreen" on macOS (a frameless window at the display's size in points, `DXVK_FORCE_WINDOWED=1`; medium-high confidence, untested). If the game process is alive but no on-screen window appears within N seconds, the launcher offers a relaunch in safe windowed 1024x768.
2. **Retina-aware resolution list + display picker:** sizes that fit the chosen display, plus "match display" for borderless. Research Wine `RetinaMode` (`HKCU\Software\Wine\Mac Driver`) for HiDPI. Choose which monitor the game opens on, remembered per display setup.
3. **Frame-rate cap:** 30 / 60 / display rate, replacing the fixed `DXVK_FRAME_RATE=30`.
4. **Mouse lock + launch logs:** confine the cursor to the window during play, released on focus loss or Cmd+Tab (check feasibility under winemac.drv). Per-launch logs: `WINEDEBUG=+display` (or a minimal set), `DXVK_LOG_PATH` to a per-launch dir, Wine stderr captured, and an "Open logs" button in the launcher.

The architect sizes these into packets and flags any that research shows aren't feasible, rather than dropping them silently.

## D5: Recommended defaults accepted (2026-10-24)

The owner said "yes to all" to the revision pass's open questions:

- D-PD5: the raw `os_error` integer is allowed in local launcher events.
- D-PD4: new failure codes map onto the existing 18 summary telemetry values; no wire change.
- D-PD6: Windows VC++/DirectX preflight is deferred until an incident.
- D-PD22: reverse-engineer the client first (PD-38). A client-patch cursor-lock hook (PD-39) is approved only if the client has no switch of its own.
- D-PD23: Wine RetinaMode is off by default; an opt-in experimental switch after UAT-PD10.
- Execution host: stay on macOS (hands-on sessions plus CI gates), per owner choice (b).

Source: owner chat, 2026-10-24.

Source: owner chat, 2026-10-24.
