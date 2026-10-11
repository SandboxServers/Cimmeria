---
name: bootstrap-launcher-install-and-test-server-port
description: After LX-18/LX-20 setup.ps1 -WithLauncher downloads the released zip (no build); local HTTP test servers must take a free port (Docker owns 8765)
metadata:
  type: project
---

Since LX-20 (2026-10-11) `setup.ps1 -WithLauncher` runs `Install-CimmeriaLauncher`
(bootstrap/CimmeriaBootstrap/Public/), which downloads
`StargateWorlds-Launcher-windows-x64.zip` + `.sha256` from the `launcher-current`
release and unpacks into `<root>\launcher\` (gitignored as `/launcher/`). A 404 on
the `.sha256` means "not published yet" and returns without failing. Nothing builds a
launcher in the root workspace any more; the egui crate, `tools/launcher-release/build.sh`
and the 7za sidecar are gone, and the overlay pack is `build-desktop.sh overlay <url>`.

Test trap: a local `python -m http.server 8765` silently lost to Docker Desktop
(`com.docker.backend.exe` listens on 8765), so requests got Docker's 404 and the
"good release" case looked like an unpublished one. Pick a free port with
`[Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)` and poll the server
before the first request (bootstrap/tests/test-install-launcher.ps1 does both).

**Why:** the next person touching the bootstrap or writing a PowerShell HTTP test hits both.
**How to apply:** never hard-code a test port on this workstation; for launcher setup
questions, the released zip is the only path. See [[launcher-release-feed-traps]].
