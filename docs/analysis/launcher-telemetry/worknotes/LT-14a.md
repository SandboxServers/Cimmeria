# LT-14a worknote

Status: done, committed on `launcher-telemetry/lt-14-install` (see commit subject `LT-14a: core install event codes and retargets`).

## Done

- New `crates/launcher-core/src/install_codes.rs`: `InstallError::code()`, `InstallError::http_status()`, `LocalSeedError::code()`, plus four unit tests.
- `crates/launcher-core/src/lib.rs`: `mod install_codes;` (rustfmt sorted it between `pub mod install;` and `pub mod install_layout;`).
- `crates/launcher-core/src/install.rs`: retargeted the four statements (bundled PAKs moved, patch skipped, patch failed, SGW.exe missing) to `target: "launcher.install"` with closed `event` names.
- `crates/launcher-core/src/install_seed.rs`: retargeted the seed copy warn to `seed_copy_failed` with `reason = error.code()`.

## Left

Nothing in LT-14a. LT-14b (engine `error_codes.rs`, `operation_numbers`, install worker events) runs next on the same branch.

## Next step

Start LT-14b from `docs/analysis/launcher-telemetry/work-packets-install.md` § LT-14b.

## Last check

`[lane] status=ok exit=0` for `cargo test --locked -p cimmeria-launcher-core install` (64 passed).
