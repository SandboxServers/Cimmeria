# Native launch ownership read fix

> Date: 2026-10-04
> Branch: `launcher/launch-lock-fix`
> Worktree: `.claude/worktrees/launcher-launch-lock-fix`
> Base: `4513fd8d9e5ec80d9815ff373a51db985a7a224c`

## Change

Native launch preparation now reads saved installation identity using
`storage::read_open` on the exclusively locked owner handle. Reopening the file
through `storage::read` used a second handle, which Windows byte-range locks
exclude even in the owning process. The retained `Ownership` still holds the
lock through the helper lifetime. The private native preparation function is
shared by Windows dispatch and the portable fixture; platform admission is
unchanged.

The regression test calls the actual dispatch preparation entry point on
Windows. An inert PE32 fixture exercises login-server writing and ASLR setup,
checks the prepared request, verifies a competing lock is excluded, and verifies
release after dropping ownership. Other hosts exercise the same native
preparation function. No game or helper is spawned.

## Validation

- Pinned-toolchain desktop formatting check: passed.
- Native macOS targeted regression: 1 passed.
- Native macOS launch module: 10 passed.
- Native macOS engine library/tests clippy with `-D warnings`: passed.
- All compiling Cargo commands used `tools/build-lane/lane.sh`.
- Native Windows run and fail-on-revert proof: pending coordinator CI.
- No frontend behavior changes; no JS REPL or visual UAT in this packet.
- No real Windows process spawn, injection, login, renderer or gameplay proof.

## Reproduce native Windows proof

Run from the registered Windows checkout with the pinned toolchain:

```bash
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine native_preparation_reads_locked_owner_and_retains_exclusion --lib --locked
```

After committing, temporarily replace `let saved: InstallIntent = read_open(&owner)?;`
with `let saved: InstallIntent = read(&owner_path)?.ok_or(StorageError::Corrupt)?;`
in `crates/launcher/desktop/engine/src/storage/launch/worker.rs`. The same test
must fail on Windows. Restore that file from the committed fix and rerun to
confirm green. Unix permits the second-handle read, so a Unix green result
cannot establish this revert proof. No Windows artifacts were crosscompiled.
