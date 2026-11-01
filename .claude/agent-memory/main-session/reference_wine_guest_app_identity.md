---
name: Wine guest macOS application identity
description: Why a wrapper app cannot own a Wine game window and what gives the real Wine process a bundle identifier
metadata:
  type: reference
---

Observed 2026-10-04 on macOS with the pinned runtime, using native fixtures and Wine's Notepad, never the game. Verify against `crates/launcher/desktop/docs/launch-mac.md` ("Optional Mac application identity for the game window") and `docs/analysis/playtests/2026-10-03-macos-wine/worknotes/wine-computer-use.md`.

- Windows belong to the Wine guest process, which registers as `wine` with no bundle id. A bundled wrapper that starts Wine as a child gets the id and zero windows.
- `wineloader` is `wine` in the `realpath` directory of the loaded `ntdll.so`; `WINELOADER` is not consulted. A linked loader resolves back to the runtime.
- macOS takes the main bundle from the executable's directory: an executable in `<name>.app/Contents/MacOS/`, started directly, gets that bundle's id. A flat `Info.plist` is ignored.
- A flat loader directory works with real copies of `lib/wine/x86_64-unix/*` plus links for the PE directories and `<bundle>/share`; `WINESERVER` covers the missing `bin/`.
- `LSUIElement` is required, else the windowless `explorer.exe` becomes a second Foreground app with the same id; `explorer.exe` still registers first, so a tool taking the first app for an id gets no window.
- `lsregister -f` resolves a bundle by id only outside `/tmp`. The runtime tree digest forbids adding files to the managed runtime.
- With `ROSETTA_X87_PATH` set, the window owner's executable path is unobserved.

INVALIDATED WHEN: the pinned Wine runtime changes.
checked: unverified 2026-11-01
