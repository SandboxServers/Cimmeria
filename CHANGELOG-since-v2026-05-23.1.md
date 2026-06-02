# What's changed since the May 23 release

**Reference point:** `v2026-05-23.1`, published 2026-05-23.
**This survey:** 12 merged PRs + 1 open PR (#374), through 2026-05-24.

The work splits cleanly into four buckets. Player-facing items come first.

---

## 1. In-game fixes (players will notice)

These are bugs that affected how the game looked or worked. They are all merged.

### Slappacks (and every other consumable) work again — PR #371

**Symptom:** Right-clicking a slappack in your inventory did nothing. No heal, no animation, the stack stayed full. Same shape applied to mission items and any other consumable.

**What was happening:** The server's "is this a weapon?" check was looking at the wrong field. Every consumable was getting mis-classified as a weapon, so the server tried to move it into your bandolier — which the inventory rules then correctly refused. The actual "use this item" event never fired, so the heal chain never ran.

**Fix:** The server now uses the right field to tell weapons from consumables. Every consumable in the game is fixed by the same one-line change.

---

### Quest kills count from auto-fire — PR #369

**Symptom:** Killing a quest-objective NPC with auto-fire (the F-key toggle, or queued first-shot, or set-auto-cycle) didn't advance the kill counter. Killing the same NPC with a manual right-click worked. The same bug affected the auto-cycle loop, the immediate-fire-on-toggle path, and the queued-attack-while-holstered path.

**Fix:** All player-attack paths now go through one shared kill-credit helper, so the quest counter advances no matter which way you fired the killing blow.

---

### Bandolier ammo type updates correctly when you equip mid-game — PR #373

**Symptom:** When you equipped a weapon during play (drag-to-bandolier or via a mission grant) the ammo-type indicator on the bandolier didn't always refresh, so the UI showed stale information until the next reload or relog.

**Fix:** The server now emits the ammo-type update on equip and on chain-driven grants, not just at login.

---

### Castle CellBlock drone encounter — PR #368

**Symptom:** The "Prisoner Retrieval Unit" drone in Castle CellBlock did nothing when the player grabbed the Ambernol vial. It was supposed to wake up, lock onto the player, and shoot an Energy Shock projectile. Instead it sat idle, and Net'an's reaction dialog never played.

**What was fixed:**

- The "set this NPC to hostile" content action now actually drives combat behavior (it was previously stored in a field nothing read).
- NPCs now load the ability list their template specifies instead of every NPC firing a Pistol Shot regardless of template.
- The combat AI now picks abilities based on cooldown state instead of always trying to fire the same one.
- The mission chain now generates threat on the drone (focusing it on the player who grabbed the vial) and plays Net'an's reaction line, in the order the original Python source intended.

This is one of three end-game touches needed to make the Castle CellBlock mission play through cleanly.

---

## 2. Server stability and resilience (you should feel these as fewer disconnects)

These are network-layer fixes. They don't change what's in the game, but they change how often you get kicked back to the login screen mid-play.

### The Mercury TX-window relief work (#354 / #356 / #360) — PRs #357, #361, #363, #365

**Background:** The game protocol gives the server only 32 "slots" for unacknowledged messages to each client. When the server tries to send more than 32 packets faster than the client can confirm them, packets get silently un-tracked — and if any of those is lost in transit, the client never recovers and the session eventually times out. This was the "lomiada" failure mode: a single transatlantic packet drop could end the session.

**What landed:**

- **Bundle everything.** The server now packs multiple in-game events (entity creates, appearance updates, dialog, sequences, etc.) into the minimum number of network packets per tick instead of one packet per event. ~30 different call sites across world entry, AoI, holster, chat, progression, and teleport were migrated.
- **Wait for the client to be ready.** Before the client signals "I'm done loading the map," the server now queues up entity creates instead of firing them into a buffer the client can't yet acknowledge. The queue flushes the instant the client says ready.
- **Never silently drop reliability.** When the 32-slot window is full, packets now go into a deferred queue and drain as the client acknowledges — instead of being silently sent unreliably with no retransmit safety net.

Together these eliminate (or recover from) the overflow that previously bit world-entry and player-defeat bursts.

---

## 3. Testing infrastructure (so these classes of bugs don't come back)

You won't see these in-game, but they're what keeps the above fixes from regressing.

### Mercury loopback session harness — PR #370

22 new paired-channel tests run two real Mercury channels against each other in-process and exercise the full session lifecycle — handshake, encryption, fragmentation, reliable delivery, retransmit timing, keepalive, and several adversarial scenarios — without ever touching a UDP socket. Deterministic, fast, and means we can pin behavior before it becomes a wire-format regression.

---

### Transport abstraction — PR #358

A `Transport` trait that lets tests substitute a recording fake for the real UDP socket. Used by the new harness and by the chaos apparatus in PR #374. Existing tests now assert on the exact bytes the server would have sent, instead of relying on "did it run without panicking."

---

### Holster animation regression guard — PR #362

The holster-on-fire / reload / toggle work shipped in the previous release (#338) but one regression guard was missing — the test only checked that the server-side state changed, not that the broadcast to other players actually fired. This PR adds the missing assertion so a future refactor can't silently break "other players see me draw / holster my weapon."

---

### Network chaos test apparatus — PR #374 (open, intended)

**Intent:** Simulate hostile network conditions in tests — packet drops, duplicates, reordering, latency — and replay real packet captures (the original lomiada session is the canonical fixture).

**What it adds:**

- **Layer 1 (Mercury):** new primitives in the test policy — probabilistic packet drop, multi-packet reorder, one-shot N-duplicate; plus 8 named chaos scenarios (asymmetric loss, burst loss, sustained 5% loss, etc.).
- **Layer 2 (Services):** a `BidirectionalTransport` trait + `LossyTransport` wrapper with named presets (LAN, Domestic, Transatlantic, Mobile). The connect-loop now routes through it.
- **Layer 3 (Replay):** loads real pcap captures and replays the bytes back through the server, validated against the lomiada fixture.

This is infrastructure, not a fix for any specific in-game bug — but it's the regression net that catches the lomiada bug shape and its cousins before they ship.

---

## 4. Developer and reverse-engineering onboarding — PR #364

A bundled toolchain installer (Ghidra + MCP server + companion tools) and an onboarding guide for contributors who need to read the original 2009 client binary to verify wire-format claims. No runtime impact; it just lowers the bar for new contributors who want to help with the deeper protocol archaeology.

---

## Counts

| Bucket | Count |
|---|---|
| In-game fixes (players notice) | 4 |
| Server stability (fewer disconnects) | 4 |
| Testing infrastructure | 4 (3 merged + 1 open) |
| Developer onboarding | 1 |
| **Total** | **13** (12 merged + 1 open) |
