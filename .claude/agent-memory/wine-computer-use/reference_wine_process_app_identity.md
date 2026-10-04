---
name: Wine guest process macOS application identity
description: Why a wrapper app cannot own a Wine game window, and what gives the real Wine process a bundle identifier
type: reference
---

# Wine guest process application identity — 2026-10-04

Observed on macOS 26.6.1 with the pinned WoWSilicon runtime (Wine fork commit
`37540b5d`), using native fixtures and Wine's Notepad from an isolated runtime
clone and throwaway prefix. Only the item dated with the game was observed with
the game.

- **Windows belong to the Wine guest process.** A stock guest registers with
  Launch Services as `wine`: no bundle identifier, no bundle URL. A bundled
  foreground wrapper that starts Wine as a child gets the identifier and zero
  windows; its accessibility window list cannot even be read.
- **The loader path is fixed by where `ntdll.so` really is.** `init_paths` sets
  `wineloader` to `wine` in the `realpath` directory of the loaded `ntdll.so`,
  and every child process executes it. `bin/wine` is only a first-stage wrapper.
  `WINELOADER` is not consulted. A linked loader or `ntdll.so` therefore
  resolves back to the runtime.
- **macOS takes the main bundle from the executable's directory.** An executable
  in `<name>.app/Contents/MacOS/`, started directly rather than through Launch
  Services, gets that bundle's identifier. A flat `Info.plist`
  beside an executable is ignored; `Contents/Info.plist` beside it is honoured
  but the bundle path is then that directory, not an `.app`.
- **Flat loader directory works.** With real copies of `lib/wine/x86_64-unix/*`
  in `Contents/MacOS/`, Wine falls back to its flat layout: it needs
  `Contents/MacOS/x86_64-unix -> .`, links for the two PE directories, and
  `<bundle>/share` for its data. `WINESERVER` covers the missing `bin/`.
- **`LSUIElement` is required.** Without it the windowless `explorer.exe`
  desktop host becomes a second Foreground application with the same
  identifier. With it, only a process that shows a window is promoted.
- **Staged loader alone: two processes share the identifier**, and
  `explorer.exe` registers first. Seen with the game on 2026-10-04; the
  computer-use tool's `getApp` then timed out by identifier and by path.
- **The desktop host takes the loader of whoever asks first.** Wine starts
  `explorer.exe /desktop` from the first process that needs the desktop window
  (`get_desktop_window`). A stock-loader keeper started before the staged
  launch leaves exactly one application with the identifier, the window owner.
- **A desktop closes one second after its last user leaves**
  (`remove_desktop_user`). A keeper blocked on its input therefore needs no
  cleanup: closing the pipe ends it and Wine ends the session. An
  `explorer.exe /desktop` started directly never closes if no user ever joins.
- **Keeper that works:** `cmd /d /c "rundll32.exe && echo READY && pause"` from
  the stock loader. Bare `rundll32` creates a hidden window and exits; `cmd /c`
  waits for it. `start /wait X && echo` still echoes when X is missing. A
  console program started by `explorer.exe /desktop <command>` gets a visible
  console window, not the caller's pipes.
- **Creating or updating a prefix starts the desktop host** from the first
  loader (its progress window), whatever else is done. A test of the keeper
  must initialise the prefix first or it passes without the keeper.
- **A window on an inactive Space is missing from the accessibility window
  list.** With another application's full-screen Space active, the game and a
  fixture Notepad both listed zero windows; Notepad listed one once visible.
- **`lsregister -f` on a bundle under `/tmp` does not make it resolvable** by
  identifier; under the user Library it does.
- **The runtime tree digest forbids adding files to the runtime**, so an
  `Info.plist` cannot be dropped into the managed runtime itself.
- With `ROSETTA_X87_PATH` set, Wine executes that program in place of the loader
  for 32-bit processes. What the window owner's executable path is then has not
  been observed.

Implementation and remaining gates:
`crates/launcher/desktop/docs/launch.md` ("Optional Mac application identity for
the game window") and
`docs/analysis/playtests/2026-10-03-macos-wine/worknotes/wine-computer-use.md`.
