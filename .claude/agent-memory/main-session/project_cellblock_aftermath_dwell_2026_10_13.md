# Cellblock aftermath dialog dwell (2026-10-13)

Source: issue #1721 and the Cellblock completion UAT server log at 2026-10-14T01:35:34Z.

- On `mission_completed 686`, chain 1161 defers dialog 2516 for 10100 ms, after the roughly 10-second Straegis Matinee. Chain 1172 previously deferred blurb 5859 for 10600 ms. The live server measured only 422 ms between their displays and logged `dialog_replaced_too_fast`; the client could not read 2516.
- Dialog 2516 is about fifty words and both dialogs are button-less. Chain 1172 now waits 26100 ms from completion, giving 2516 about 16 seconds on screen before 5859. The live-DB chain-replay guard checks both seeded delays and at least a 15-second gap.
- The separate single-open-dialog pin warning may still occur if 5859 replaces an unclosed 2516. Packet DU-08 owns that behavior. This timing change addresses readability.
- Live lab verification on the changed seed is required before the PR merges.
