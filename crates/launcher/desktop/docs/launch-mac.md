# Launch contract: macOS (Wine and DXVK)

> **Type:** Reference
> **Audience:** Launcher integrators and playtesters
> **Last updated:** 2026-10-26
> **Companions:** [Native game launch contract](launch.md), [prerequisites](prerequisites.md), [wine-validation](wine-validation.md)

The macOS half of the launch contract: graphics and prerequisite boundaries, per-launch Wine and DXVK logs, and the optional application identity for the game window. The shared admission, lifecycle and resource rules are in [launch.md](launch.md).

## Mac graphics and prerequisite boundaries

Launch reopens the exact successful generation under `game-prefixes`, never the
headless extraction prefix. It verifies the cached managed runtime while holding
its cache lock. Wine's game environment enables its graphics driver and sets
`d3d9=n`; the pinned `Graphics.d3d9` is staged beside the actual `SGW.exe`.
An existing matching DLL is reused. An existing different DLL is refused rather
than silently overwritten. `CX_FWD_COMPAT_GL_CTX=1` preserves the experimentally
established Mac OpenGL compatibility workaround. Game preparation also sets
`VK_DRIVER_FILES` to `lib/vulkan/icd.d/MoltenVK_icd.json` inside that verified
runtime, so D9VK discovers the bundled MoltenVK surface implementation. A missing,
non-file or redirected descriptor fails preparation before any helper spawn.
This selection is native-owned; the webview cannot supply a driver path.
`DXVK_FORCE_WINDOWED=1` makes the DXVK `d3d9.dll` present windowed whatever the
client asks for: exclusive fullscreen would change the display mode, which
winemac.drv turns into a capture of every display (black screen, Cmd+Tab
freeze).
Extraction and prerequisite operations retain their headless environment.

`Graphics.rosetta_x87` optionally binds an independently pinned accelerator and
its adjacent companion library; it sets `ROSETTA_X87_PATH`. `None` uses stock
Rosetta without an extra accelerator. Supplying resources is not a license,
redistribution or performance claim. The managed Wine archive alone contains
neither the chosen D3D9 overlay nor the optional x87 accelerator. The integrator
must stage exact artifacts, record provenance and notices and validate them with
the real client. Successful PhysX/module prerequisites do not demonstrate device
creation, DLL patch hooks, login or world entry.

### Per-launch Wine and DXVK logs

Each Mac Play writes its own logs to `logs/game/<operation>/` under the launcher
state root (PD-28): `wine.log` is the Wine helper's stderr, at most 4 MiB
(`STDERR_CAP`; the rest is read and dropped, so Wine never blocks on a full
pipe), and DXVK writes its `*_d3d9.log` there through `DXVK_LOG_PATH` (the
folder's `Z:\` guest path) with `DXVK_LOG_LEVEL=info`. With logs on,
`WINEDEBUG` is `err+all,fixme-all,+display`: Wine's errors plus winemac.drv's
display code. Creating the folder prunes `logs/game/` to 5 operation folders:
the new one and the 4 newest others by folder mtime. If the folder can't be
created, Play starts without logs and with `WINEDEBUG=-all`, as before.

These logs contain local paths, so they stay on the computer: the debug log
upload reads only the `launcher.*.log` files in the app-data `logs/` folder (a
sibling of `state/`, not this one) and the game's own logs, never
`state/logs/game/`. Windows Play is unchanged; its helper's stderr is discarded.

## Optional Mac application identity for the game window

macOS automation that selects an application by name, bundle path or bundle
identifier cannot select a stock Wine game. The process that owns the game
window registers with Launch Services as `wine`, with no bundle identifier and
no bundle. A separate wrapper application does not change that: windows and
their accessibility tree belong to the process that created them, and a Wine
guest process is never the wrapper.

A Wine process takes its identity from the directory its loader really runs
from. The pinned runtime's loader sets `wineloader` to `wine` beside the
`ntdll.so` it loaded, after `realpath`
([`init_paths`](https://github.com/WineAndAqua/wine/blob/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8/dlls/ntdll/unix/loader.c#L387-L404)),
and every child process, including `SGW.exe`, executes that path
([`preloader_exec`](https://github.com/WineAndAqua/wine/blob/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8/dlls/ntdll/unix/loader.c#L443-L466)).
macOS gives a process a main bundle when that path is
`<name>.app/Contents/MacOS/`. Links do not count, because the loader resolves
them first.

Setting `CIMMERIA_WINE_APP_IDENTITY=1` in the launcher's own environment makes
Play stage `wine-app-identity/Stargate Worlds.app` under the launcher state root
and start the launch worker with the loader inside it. The webview cannot set
this. Without the variable Play starts `bin/wine` from the runtime exactly as
before and writes nothing.

| Bundle entry | Content |
|---|---|
| `Contents/Info.plist` | `CFBundleIdentifier` `app.cimmeria.stargate-worlds`, `CFBundleName` `Stargate Worlds`, `CFBundleExecutable` `wine`, `LSUIElement` true |
| `Contents/MacOS/<file>` | A copy of every regular file in the runtime's `lib/wine/x86_64-unix/`, including the loader and `ntdll.so`, each checked against the bytes it was copied from |
| `Contents/MacOS/libvulkan.1.dylib` | Link to the same runtime file the runtime's own relative link names |
| `Contents/MacOS/x86_64-unix` | Link to `.`, where Wine looks for a builtin's unix library |
| `Contents/MacOS/x86_64-windows`, `i386-windows` | Links to the runtime's PE directories |
| `share` | Link to the runtime's `share`, where Wine looks for its data |

Staging runs after the runtime tree digest is verified and while its cache lock
is held. The bundle is rebuilt on every Play in a staging directory and swapped
in; an old bundle is removed without following links, and a bundle path or
state directory that is a link is refused. The bundle is then registered with
`lsregister -f` so lookups by name or identifier can find it. If anything
fails before the worker starts, Play uses the stock loader and prints the
reason to the launcher's standard error. The environment, the
`DXVK_FRAME_RATE` limit (the player's frame cap, § The frame cap), the helper protocol and host supervision are the same
on both paths: the host PID is still the real Wine process.

`LSUIElement` matters. Every Wine process of the session shares the bundle.
Wine's Mac driver promotes a process to a regular application only when it
shows a window
([`cocoa_app.m`](https://github.com/WineAndAqua/wine/blob/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8/dlls/winemac.drv/cocoa_app.m#L312));
without the key the windowless desktop host `explorer.exe` also becomes a
foreground application with the same identifier.

What was observed on macOS 26.6.1 with the pinned runtime, never with the game:

| Case | Window owner as Launch Services sees it |
|---|---|
| Stock loader, Wine Notepad | name `wine`, no bundle identifier, no bundle |
| Foreground wrapper application starting a separate window-owning process (native fixture) | wrapper has the identifier and zero windows; the child owns the window and has no identifier |
| Staged bundle loader, 32-bit `cmd` starting 32-bit Notepad | Notepad process is `Stargate Worlds`, `app.cimmeria.stargate-worlds`, type Foreground, one accessibility window; `explorer.exe` has the same identifier, type UIElement, no accessibility window |
| `lsregister -f` on a fixture bundle under the user Library | `NSWorkspace` resolves the identifier to the bundle path; the same bundle under `/tmp` does not resolve |

The last case is the ignored engine test
`real_wine_window_owner_carries_the_bundle_identity`. It needs
`CIMMERIA_WINE_RUNTIME_CLONE` set to a copy of the managed runtime directory,
uses a throwaway prefix, shows a Notepad window and stops only that prefix's
wineserver:

```bash
CIMMERIA_WINE_RUNTIME_CLONE=<copy of the runtime directory> \
  bash tools/build-lane/lane.sh cargo test --locked \
  --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine \
  --lib app_identity -- --ignored
```

Still unproved, and the reason this is opt-in:

- The game itself under the staged loader: D9VK and MoltenVK rendering, patch
  injection, login. A failure after the worker starts has no fallback.
- The optional x87 accelerator. Wine executes `ROSETTA_X87_PATH` in place of the
  loader for 32-bit processes; whether the process that ends up owning the
  window still runs from the bundle has not been observed.
- Whether a given automation tool binds the window owner. Two running
  processes share the identifier and the windowless one registers first. A tool
  that takes the first match instead of the regular application gets no window.

To roll back, start the launcher without the variable. The staged bundle is
inert; delete `wine-app-identity/` under the state root and run
`lsregister -u` on the bundle path to drop the registration. Opening the bundle
directly runs the loader with no arguments, which prints its usage and exits
without creating a prefix.
