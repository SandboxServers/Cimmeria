# RESUME: launcher-linux-play-proton

Ledger: README.md. Packets: work-packets.md, -w1b.md, -w2b.md, -w3.md.

## State (2026-10-25, waves 0-3 closed)

- Integration tip 94f2e0af9, pushed. Draft PR #2032 (integration -> main).
  Full CI also dispatched (run 40236283045). Not for merge until PP-U passes
  and the campaign review is ship.
- All packets of waves 1, 1B, 2 and 3 are merged and reviewed ship.
- Wave 3 ruling (Linux frame cap 30 or 60): wave3-ruling-1.md.
- Next: fix PR CI failures (RESUME length; native-engine macOS and Windows),
  then PP-U on the PR's AppImage (owner authorised hands-on; artifact
  desktop-launcher-linux-dry-run), then PP-14 close-out (split launch.md, over
  the 700-line cap), campaign review, one update of project-status.md.

## Owner rules

- One final PR. Packets merge-noff into the integration branch. No Wine for
  cabinet expansion. No client patch beyond what macOS does.
- OD-1 to OD-3 as decided; OD-4 reversed: draft PR early for the AppImage.
- Packet worktrees from the integration tip need a hands-on agent.

## Open items

- PhysX under Proton is unproven until PP-U (i686 worker built by the PR jobs).
- PP-02 telemetry-DLL wording (optional vs required) unconfirmed.
- Follow-ups FU-13..FU-17 are in the ledger README (stale Play button
  corrupt_state, uat:launch cannot run on Linux, scratch folders, SYSTEM_TOOLS
  dup, directx log DLL name).
- Wave-1B Deck UAT timing check not yet scheduled with the owner.
- Owner: colo check of the 53 uploaded log lines? Fresh launcher release?
- First-campaign cleanup (branches, li-* worktrees, build output) needs
  hands-on. Never run git worktree prune.
- Flake seen once: prefix::tests::second_claim_is_busy_while_the_first_is_held;
  adoption::light_update_tests::repair_of_a_light_adopted_install... (not
  reproduced in about 260 runs).
