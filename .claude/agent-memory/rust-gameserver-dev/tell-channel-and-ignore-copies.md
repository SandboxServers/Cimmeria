---
name: tell-channel-and-ignore-copies
description: Client /tell arrives on channel 10 while Rust CHAN_TELL still says 9; the Ignore list lives in three copies kept in step by one resync; test decoders must handle the 0xBD extended encoding for method index >= 61
metadata:
  type: project
---

Facts from SS-C1 (tells and Ignore, 2026-09-27):

- **The client sends `/tell` on channel byte 10** (the `EChannel` value; ORG-E1 Q5 / D-ORG14). Both Rust `CHAN_TELL` constants still say 9 until ORG-09 aligns them, and 10 is `CHAN_SPLASH` in `cimmeria_wire::cell::chat`. The tell path uses a local `dispatch::tell::TELL_CHANNEL = 10`. A tell that reaches the cell means the base branch was bypassed.
- **The Ignore list exists in three copies**: the DB rows (flags 301), `ConnectedClientState::ignore` (read by tells) and `CellEntity::ignore_names` (read by spatial chat). `contact_list::ignore::resync_ignore_cache` rewrites both caches. It runs at every `onClientReady`, which covers gate travel because that path creates a fresh cell entity and re-runs onClientReady, and inside the contact-list member ops. Any new writer of the Ignore list must go through those member ops, or the caches go stale until the next world entry.
- **Test decode trap**: a base-side test that decrypts `build_player_entity_method_packet` output and reads `body[0] & 0x7F` as the method index is wrong for index >= 61 (contact-list CM 85-89, for example). Those use marker `0xBD`, then the length, the entity id, and `sub_index` at `body[7]`, with args from `body[8]`.

**Why:** each of these fails silently: a tell renders as "channel not supported", an ignore does nothing until relog, and an assertion "never sees" a CM 87 that was sent.
**How to apply:** check these before touching chat channels, the Ignore list or base-side packet-decoding tests. See [[witness-entity-method-dual-fn]] for the idbase 61/62 split.
