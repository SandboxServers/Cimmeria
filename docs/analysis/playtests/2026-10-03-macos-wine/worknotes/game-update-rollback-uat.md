# Signed game rollback verification

The bounded portable verification uses production `NativeHost` dispatch with
isolated temporary storage and loopback signed ZIP downloads. Test-only dispatch
selects portable engine entry points; production trust and platform gates are
unchanged. Neither archive contains a runnable game.

## Verified behavior

- Confirmed Update reconstructs the new release; signed rollback reconstructs the
  previous release through the same preparation/commit path.
- Rollback preserves the immutable installation owner byte-for-byte and retains
  two distinct backups: the original modified tree and the newer modified tree.
  Neither tree's local modifications are merged into the reconstructed game.
- Duplicate rollback admission produces one rollback download. Reopened native
  storage retains the rollback target and original installation owner.
- A held partial download can be cancelled, then explicitly discarded without
  removing the old game. An interrupted precommit handoff reopens as reconciliation
  required; native host abandonment and discard preserve that old game.
- Mounted Effect/DOM UAT reviews both signed identities, applies Update, loses its
  response, inspects completion, reopens, confirms actual rollback, loses its
  response, reopens and inspects without redispatch, then cleans the latest backup.
- A frontend regression rejects changed rollback release identity even when the
  native operation revision is unchanged.

## Validation commands

From the project root, compiling Rust commands use the build lane:

```sh
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop game_update --no-fail-fast
bash tools/build-lane/lane.sh cargo clippy --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --all-targets -- -D warnings
```

From `crates/launcher/desktop/frontend`, run `npm test`, `npm run check`, and
`npm run uat:game-update-apply` with `GAME_UPDATE_UAT_BINARY` pointing to the shell
test binary printed in the lane log. Build the frontend first in a fresh checkout
because Tauri embeds `frontend/dist`.

The scoped shell suite passes eight tests, with two opt-in bridge tests ignored.
Frontend tests pass 61. The mounted native-persistence UAT, strict shell all-target
clippy, TypeScript check, Rust format check and diff whitespace check pass.

## Remaining gates

The DOM UAT does not provide visual webview verification and does not exercise its
cancel/abandon controls; those routes are covered by the Rust host tests. Real
client execution, Wine helper execution, Windows locks/renames, and hardware
power-loss durability remain unverified here. No user installation or external
service was used. The interrupted-preparation fixture deliberately loses the
engine handoff; it does not kill a live operating-system process.
