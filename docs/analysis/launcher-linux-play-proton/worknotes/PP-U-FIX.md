# PP-U-FIX worknote (coder session 1)

Status: blocked, reason context. Nothing committed. Last check status: none run.

## Done (uncommitted)
- `crates/launcher/desktop/shell/src/self_test_rar/fixture.rar` (96 bytes), a stored RAR 4 archive with one file `SELF-TEST.TXT` and the lock flag. Generator: `/tmp/claude-1000/-home-deck-Projects-Cimmeria/8579b1db-de0b-4c77-866a-aae70f7932ef/scratchpad/mkfinal.py` (no comment block).

## Findings
- An old-style main-header comment (MHD_COMMENT plus a 0x75 comment block) was rejected by UnRAR: "The archive comment is corrupt". I tried comment CRC variants (CRC-32 low and high half, inverted, zeroed) and the block layouts. None passed, so the comment is dropped. This deviates from the spec.
- `unrar t` (system CLI 7.23) reports "Total errors: 1" with no message, even for a flagless variant of the same bytes. The file itself reads OK. The cause is unexplained. Check the fixture with the launcher's own UnRAR (library path) before trusting it.
- The shell's `headless_probe` (payload_startup.rs) is Windows-only. The `--self-test-rar` hook must sit at the top of `main()` in `shell/src/main.rs`, before the Tauri builder.
- `UnpackSink` needs a `CancellationToken`. `tokio-util` is only a dev-dependency of the shell, so move it to `[dependencies]` in `shell/Cargo.toml` (no Cargo.lock change expected).
- `unpack` and `UnpackSink` are reachable as `cimmeria_launcher_engine::unpack` (engine lib.rs re-exports the launcher-core module).
- Local toolchain: `mold` is only the shim in `~/.local/opt/cimmeria-cc-bin`. There is no `ld.lld`, `lld`, `clang`, `actionlint` on PATH, so the lld flag cannot be exercised locally.
- Linux launcher builds that link with mold: `.github/workflows/launcher-desktop-linux.yml` (build step, reached from launcher-release.yml `desktop-linux` and from launcher-desktop.yml), `.github/workflows/launcher-desktop.yml` native-engine ubuntu leg, and `.github/workflows/client-launch.yml` linux job (nextest on cimmeria-launcher-core, mold installed at line 125).

## Left to do, in order
1. Settle the fixture against the library path (`unpack::unpack` through the shell) and drop the comment claim unless it passes.
2. Add `src/self_test_rar.rs` (`run()`, `requested()`, unit test that extracts the fixture), hook it in `main()`, move tokio-util in `shell/Cargo.toml`.
3. lld: apt line plus `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS: "-C link-arg=-fuse-ld=lld"` in launcher-desktop-linux.yml (build step), launcher-desktop.yml native-engine (Linux only), client-launch.yml linux job. Keep glibc floor (ubuntu-22.04). Leave `.cargo/config.toml` alone.
4. `tools/launcher-release/build-desktop-linux.sh` `stage_verify`: run `"$exe" --self-test-rar`.
5. `tools/launcher-release/test_workflow_policy.py`: assert the lld flag in the three workflows and `--self-test-rar` in stage_verify.
6. `crates/launcher/desktop/docs/release.md` per the ruling, plus the doc-update-map row 57 (CRLF).
7. Checks: `cargo fmt --check`, clippy `-D warnings`, tests for cimmeria-launcher-desktop, actionlint (absent), `python3 -m unittest tools/launcher-release/test_workflow_policy.py`.
8. Commit `fix(launcher): link Linux builds with lld, mold 1.0.3 breaks bundled UnRAR (PP-U-FIX)`.

## Next step
Step 1: build the shell through `bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop` after step 2 exists, and see whether the fixture passes.
