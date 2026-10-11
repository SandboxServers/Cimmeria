# Testhost staging

| Entry point | What it does |
|---|---|
| `stage.sh [out-dir]` | Build everything the `sgw-testhost` boot tests load, for i686, into one directory (default `<target dir>/testhost`): `sgw-testhost.exe`, `sgw-start32.exe`, `cimmeria_client_patches.dll`, and both builds of `cimmeria_client_telemetry.dll` (default and `lab-bridge`) |

Run it through the lane: `bash tools/build-lane/lane.sh bash tools/testhost/stage.sh`. CI runs it in `client-dll-boot.yml` before the tests in `crates/sgw-testhost/tests/`. What those tests check: [client-telemetry.md](../../docs/architecture/client-telemetry.md).
