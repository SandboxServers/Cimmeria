# CH-07 filler design for the flush shift mode

> Packet: [CH-07](../work-packets.md#ch-07-flush-modes), design step. No code
> changes. Grades: **static** is read from the decompile or a doc, **inferred**
> is read from code shape, **open** needs a live check.

## Verdict

Keep `Shift` and `ShiftSweep`. Fillers: `updateFrequencyNotification` (0x02)
for length 2, `voiceData` (0x33) for lengths 3 to 36, and **two** messages
(0x02 + 0x33) for 37 and 38. Length 1 is impossible and is never needed. Widen
the registry bound for `Shift { len }` from `0 | 2..=36` to `0 | 2..=38`.

## 1. Are the candidates harmless, and what bytes

Both are connection-level messages with no entity id, so they sit outside
CREATE_ENTITY's per-entity transaction (`deferred_flush.rs:152-157`) and
cannot be held or dropped by it. Both ids have a handler in the client's
interface table (`docs/protocol/message-dispatch-table.md:65,134`), so the
loop does not stop on "unknown message id" (receive-path finding, "The bundle
message loop" step 1).

| Filler | Bytes | Length | Effect | Grade |
|---|---|---|---|---|
| `updateFrequencyNotification` | `02 0A` (`UPDATE_FREQUENCY_HZ` = 10, the value the connect bundle already sends, `crates/wire/src/mercury/protocol/session.rs:70-71`) | CONSTANT 1, so 2 total (`message-dispatch-table.md:65`; `space-viewport-wire-formats.md:32-40`) | Re-states the tick resolution the client already holds. The same value is a no-op. The handler is an unnamed `ClientMessageHandler<updateFrequencyNotificationArgs>` (RTTI `0x01e520e0`) and "sets `s_updateFrequency_`" comes from a BigWorld-source doc (`post-auth-sequence.md:67`), not a decompile here. Sending the unchanged value makes that moot. | static (layout), inferred (effect) |
| `voiceData` | `33 nn nn` + `n` zero bytes, `nn nn` = `n` as u16 LE (WORD_LENGTH, header 3 bytes, same shape as CREATE_ENTITY `09 ll ll`) | 3 + n | `ServerConnection_voiceData` (`0x00dd68b0`, `decompiled/14_standalone_named.c:301786-301801`) tests `this+0x168`; null logs through a stubbed logger and returns. Non-null forwards `(param_1, param_2)` to a voice handler. Docs say SGW registers none (`space-viewport-wire-formats.md:455-463`). | static (decompile), **open** (that `this+0x168` is 0 at this point in login) |

Why the iterator stays in step: `next()` advances by the declared length
whatever the handler consumed (finding, "Bundle message loop" step 5), so a
handler that ignores the body cannot desync the loop.

Open live check for the CH-07 acceptance run: read `ServerConnection+0x168`
with `client_mem_read` once in the world; 0 settles voiceData. Until then
voiceData is "harmless by decompile, unconfirmed by memory".

## 2. Is every residue reachable

A collision needs `cursor == R` at a message start in the **first packet**
(finding, "Only the first packet of a chain is exposed"). With filler length
`L` the record `j` has create at `1 + L + 37j` and avatar update at
`12 + L + 37j` (record = 11 + 26 = 37 bytes, finding "Why 752 hits exactly one
message"). The filler itself sits at cursor 1 and collides only for `R = 1`, not
a multiple of 8.

Fix `R` and a family:

- Create family: `L = (R - 1) mod 37`, `j = (R - 1 - L) / 37`.
- Avatar family: `L = (R - 12) mod 37`, `j = (R - 12 - L) / 37`, needs `R >= 12`.

So one `L` in 0..=36 per `(R, family)`. A given record `j` reaches only
`R` in `[1 + 37j, 36 + 37j]` (create); "move every cursor onto any R" is
satisfied by choosing the record, not by one `L` serving all records.

Length 1 is needed only when `(R - 1) mod 37 == 1` (create) or
`(R - 12) mod 37 == 1` (avatar). For multiples of 8 that is `R` in
{224, 520, 816, 1112} (create) and {272, 568, 864, 1160} (avatar). The two
families' `L` differ by 11 mod 37, so they are never both 1. Two ways out:

1. Use the other family for that `R`. The observed 752 case is an avatar
   collision with `L = 0` and needs nothing. Not every test can switch family
   (a create-collision test at R = 224 cannot), hence (2).
2. Use `L + 37`: `L = 38` with record `j - 1` lands on the same cursor
   (`1 + 38 + 37(j-1) = 1 + 1 + 37j`). 38 = 2 + 36 is two fillers (0x02 +
   0x33 with n = 33). Needs `j >= 1`, true for every bad `R` above (the
   smallest is 224).

`R = 8` is create `L = 7`, `j = 0` (the avatar family needs `R >= 12`).
Total length 3 (`33 00 00`, a zero-length body) is the one filler whose body
is empty: the iterator rules accept it (body fits trivially) but no live run
has shown it, so the acceptance run should exercise `len = 3` once. If it
misbehaves, the lengths that need it (create `R = 4 mod 37`, e.g. 152) fall back
to the avatar family.

Upper bound on `R`: the first packet. `deferred_flush.rs:159-160` treats
`FRAGMENT_BODY_SIZE` = 1300 as the packet cap for a bundle body, so a
single-packet bundle holds `(1300 - 1 - L) / 37` = at most 34 records and
`R <= 1 + L + 37 * 33 + 11` = about 1270 (the observed residues are all below
1113, the 24-record packet is 889 bytes). The 1472 socket cap
(`client-mercury-receive-path.md`, "Socket receive cap") is not the limit
here. `Pack { records }` allows 1..=255 in the registry; the flush must clamp
or refuse above the single-packet fit, and say so in the log.

## 3. Interaction with #1341 and `Pack`

- **Shift is meaningless unless the first packet holds more than one
  message.** Under the #1341 mitigation (one message per packet, rule 1 of
  the finding's "Server-side rule") every message is at cursor 1 and the filler
  would be a separate 2-byte packet. Today's tree (this worktree) still sends
  one bundle (`deferred_flush.rs:352-378`), so `Shift` works on current shape;
  after the mitigation lands it needs `Pack`.
- Therefore **`Shift` and `ShiftSweep` imply the `Pack` bundle shape**: build
  the single-packet bundle, then prepend the filler. Do not make `Shift`
  silently inert when the mitigation is on. If the registry keeps `Shift`
  and `Pack` as exclusive flush modes, `apply` should treat `Shift` as
  "pack all records, then shift" and log `flush_shift_packs`.
- A shifted bundle is a #1341 repro only if `R == cursor` of some record. The
  run needs `R` known: the lab reads it from `client.mercury.request_misparse`
  or from the x64dbg probe in the finding ("Decisive probe"); `ShiftSweep` is the
  search for an unknown `R` and takes at most 37 distinct `L` values (with 38 for
  the class-1 residue) to hit every residue class mod 37 for one family.
- The filler goes through `BundleManifest::append_raw_message` with
  `entity_id = 0` (`bundle_manifest.rs:46`), so `aoi.bundle_manifest` offsets
  stay true and show the filler's `msg_id` 0x02/0x33.
- The filler must be first in phase-1 only, never in phase-2 (the cascade's
  later fragments see `+0x14 = 0`, so a shift there tests nothing).

## 4. Recommendation

- `Shift { len }`: `len` in `0 | 2..=38`. 0 = none; 2 = `02 0A`; 3..=36 =
  `33` + u16 `len - 3` + zeros; 37 = `02 0A` + voiceData(n = 32); 38 =
  `02 0A` + voiceData(n = 33). Update the registry bound in CH-04
  (`Shift { len }` row) and the `lab_chaos` docs in the same PR.
- `ShiftSweep { seed }`: a seeded draw from `{0} ∪ [2, 38]` (never 1), logged
  with the drawn `len`. Add a deterministic helper
  `shift_for_cursor(R, family, records)` returning `(L, j)` for tests and the
  lab; `None` when `R` is outside the first packet.
- Keep `Pack`: it is the #1341 pre-mitigation shape and the precondition for
  any shift.
- Regression tests to add: `shift_38_lands_class_1_residue` (R = 224, create
  family, record 6, bytes `1 + 38 + 37*5 = 224`); `shift_len_1_is_refused`;
  `shift_filler_is_two_messages_above_36`; filler bytes pinned for 2 and 36.
- Residual risk: voiceData's null handler is decompile-grade only. If the
  `+0x168` read is non-zero, drop voiceData and cap `Shift` at `0 | 2` plus
  repeated 0x02 (every even length via k copies of `02 0A`, odd lengths lost).
