---
name: cargo-artifact-hardlink-cp-trap
description: Cargo's target/<triple>/<profile>/*.dll is a hardlink to deps/; cp over it poisons the deps copy and a rebuild silently restores the wrong bytes
metadata:
  type: project
---

Seen 2026-09-29 while revert-proofing the telemetry DLL's lab-bridge
check: `cp lab.dll target/i686-pc-windows-msvc/debug/cimmeria_client_telemetry.dll`
wrote through the hardlink into `deps/`, so the next `cargo build` said
fresh and re-linked the *lab* bytes. `ls -la` shows link count 2.

A cdylib's `deps/` name has no metadata hash, so a default build and a
`--features lab-bridge` build of the same crate also overwrite each other
(`tools/testhost/stage.sh` copies the default one out first for that reason).

**How to apply:** to swap in a test artifact, `rm` the file first, never
`cp` over it; to recover, `touch` a source file of the crate and rebuild.
For a revert proof, prefer copying into a separate staging dir (the
`testhost/` dir `stage.sh` fills is plain copies). See [[dll-boot-testhost-traps]].
