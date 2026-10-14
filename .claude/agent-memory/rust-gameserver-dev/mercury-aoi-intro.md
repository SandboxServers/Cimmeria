# First-login AoI intro packet shape

Verified in the server composer, client receive-path finding and wire tests
on 2026-10-14; live-client validation of the mitigation is pending.

`compose_create_entity_base_body` returns one 37-byte `IntroRecord`, but it
contains two Mercury messages: 11-byte `CREATE_ENTITY` (0x09, word length)
and 26-byte `UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR` (0x10). The client
`Bundle::iterator` can begin with a stale next-request offset. A collision
on the cursor of a later message in a one-packet burst misparses it and
discards the rest of that bundle. The 889-byte, 24-NPC first-login phase-1
burst had exactly this observed failure at cursor 752.

For the default `cinematic_hold_release`, split each message into its own
reliable packet, with all creates/avatars before the phase-2 cascade. An
ordinary `on_client_ready` flush and CH-07 Pack/Shift fault probes keep their
packed phase-1 shape. See
`docs/reverse-engineering/findings/client-mercury-receive-path.md` and
`crates/base-world-entry/src/base/world_entry/cell_dispatch/intro_bundles.rs`.

The split can send more than the Channel's 32-entry TX window. The
`unsent_packets` deque stores reliable bookkeeping for packets already sent,
up to 1024; ACKs of earlier entries promote queued entries into the window.
Tests must assert wire sequence and both deques, then process selective ACKs.
The mitigation has a live-client proof requirement; the fragmented phase-2
first fragment remains a separate potential collision point.
