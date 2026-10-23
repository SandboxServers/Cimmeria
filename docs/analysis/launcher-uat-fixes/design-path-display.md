# Windows path display follow-up

2026-10-23: Owner screenshots of launcher 2026.10.2302 show the canonical Windows `\\?\` prefix in Local install directory and the completed adoption message. Adoption itself works.

The existing frontend `displayPath` handles drive and UNC paths correctly. Apply it only when writing visible folder strings: Settings, adoption status/replacement backup copy, and Update/Repair/Uninstall confirmation directories. Preserve native snapshots, stored preferences, plan digests, identity comparisons and command arguments exactly. Relative inventory paths and unsupported device paths retain their spelling.

Use DOM unit regression tests for the reported views and adjacent confirmation surfaces, with verbatim drive and UNC fixtures. Verify consent saves and refresh/reopen retain exact native directory strings. Temporarily remove display calls and demonstrate failures, restore, then run all frontend tests, TypeScript/build and a sequential JS logic UAT. Visual verification supplements this; report native picker, packaged application and game limitations explicitly. No release is authorized by this fix.

Doc-map: desktop launcher workflows/player-visible behavior; update desktop README, player guide and committed project memory. No IPC, storage schema, release, macOS-specific or updater policy change.
