---
name: Wine guest process macOS application identity
description: Why a wrapper app cannot own a Wine game window, and what gives the real Wine process a bundle identifier
type: reference
---

# Wine guest process application identity — 2026-10-04

Observed on macOS 26.6.1 with the pinned WoWSilicon runtime (Wine fork commit
`37540b5d`), using native fixtures and Wine's Notepad from an isolated runtime
clone and throwaway prefix. Nothing here was observed with the game.

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
- **Two processes still share the identifier**, and `explorer.exe` registers
  first. A tool that takes the first running application for a bundle
  identifier gets no window.
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
