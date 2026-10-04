---
name: reference-launcher-ui-verification-traps
description: "Traps hit while visually checking the launcher redesign on Windows (2026-10-03): the prototype renders blank under Python's http.server, PowerShell's `bash` is WSL, and debug launchers reject the live manifest."
metadata:
  type: reference
---

- **The prototype at `e70b076a9` renders blank** under `python -m http.server` on Windows. The server sends `state.mjs` as `text/plain` (the registry MIME map), so the module script never runs. Serve it with `SimpleHTTPRequestHandler.extensions_map['.mjs'] = 'text/javascript'`. Headless Edge (`msedge --headless=new --screenshot=… --virtual-time-budget=3000`) then captures `?variant=A|B|C`.
- **From PowerShell, `bash` is WSL's bash.** `bash tools/build-lane/lane.sh …` run from PowerShell builds inside WSL, and its log path is a `/home/...` path, not the Windows target dir. Run the lane from Git Bash, as the rest of the build rules assume.
- **A debug launcher cannot load the live content manifest.** Debug and test builds verify against the dev key, while `content-current` is signed with the release key, so the UI shows "Could not load the game content list". That is expected locally. It is not a bug in the manifest or the redesign.
- **The launcher reads `launcher-config.json` beside its exe.** For a screenshot run, copy the exe into a scratch folder with a config whose `install_path` is a scratch path. Otherwise the default `%LOCALAPPDATA%\Stargate Worlds` is used, and the writability probe creates that folder.
