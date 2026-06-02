# CAT-G — Mail (server-authority audit)

## Trust posture summary

The SGW mail surface is **functionally unimplemented** server-side for every
mutating operation that touches currency or items: `sendMailMessage`,
`takeCashFromMailMessage`, `takeItemFromMailMessage`, `payCODForMailMessage`,
and `returnMailMessage` are all stubbed (`tracing::info!("UNIMPLEMENTED: …")`)
in `crates/services/src/cell/cell_methods/mail.rs`. Only header listing
(`requestMailHeaders`), body reads (`requestMailBody`), `archiveMailMessage`,
and `deleteMailMessage` execute against the DB.

This is a security-relevant state for two reasons:

1. The dispatcher's no-op stubs return `true` (handled), so the client gets
   no `sendMailResult` failure path. A future implementer who fills in the
   handler bodies inherits a wire surface that already accepts every
   client-asserted field (recipient list, cash, COD flag, item_id, item
   quantity, mail_id-on-take, mail_id-on-pay-COD) **with no validation
   plumbed in**. The decoder structure in `cell/cell_methods/mail.rs`
   reads `mail_id` / `container_id` / `slot_id` straight out of the LE
   bytes and forwards them; there is no model for "validate the mail
   belongs to the caller before the take/pay/return executes" anywhere
   in the file. Whoever wires the send/take/COD paths next will, by
   default, build a TOCTOU-shaped duplication exploit unless they
   re-derive the invariants from scratch.
2. The implemented surface (headers / body / archive / delete) correctly
   scopes by `character_id = $player_id` where `player_id` is
   server-resolved via `resolve_mail_player_id()` (cell/mail.rs:20-28),
   which refuses to fall back to 0 — that defends against the "send
   ops with player_id=0 sentinel" foot-gun. The `entity_id` carried in
   the cell-method 4-byte prefix is parsed but immediately discarded in
   favor of `player_eid` from the connected-client session record
   (base/connect_loop/cell_arms.rs:64-89,121-138), so a client cannot
   route a mail op against another player's mailbox via spoofed
   entity_id.

The dominant finding for this category is therefore "an entire mail vertical
will land as an exploit chain the moment it is implemented" — the audit
flags the missing-validation surface so it gets caught at implementation
time rather than after merge. Two secondary findings cover the unread-time
UPDATE shape and the `ToText` field mis-fill.

---

### CAT-G-01 — sendMailMessage handler is unimplemented but dispatcher reports "handled"

**Severity**: High (latent — becomes Critical when handler body is filled in without validation)
**Class**: Missing handler / silent client trust
**Wire surface**: `Event_NetOut_SendMailMessage` (CellMethod index 44)
**Demonstrable / Likely-theoretical**: Demonstrable (the absence of any server-side `INSERT INTO sgw_gate_mail` path is observable)

**Trust violation**
The client emits `Event_NetOut_SendMailMessage` carrying eight client-asserted
fields: `RecipientFlags: INT32`, `Recipients: ARRAY<WSTRING>`, `Subject:
WSTRING`, `Body: WSTRING`, `Cash: INT32`, `bCOD: UINT8`, `ItemId: INT32`,
`ItemQuantity: INT32` (per `entities/defs/interfaces/SGWMailManager.def:56-66`).
The server-side dispatcher accepts the method (returns `true`), logs
`UNIMPLEMENTED: sendMailMessage`, and silently drops the bundle. There is
no server-side debit of sender cash, no inventory removal, no recipient-
name resolution, no recipient-mailbox-full check, no bounded recipient-array
length check, no `INSERT INTO sgw_gate_mail`. The next implementer inherits
a wire shape where every field is whatever the client wrote.

**Evidence**
- Ghidra: `019bf2e0` / `019caf68` — string `Event_NetOut_SendMailMessage`; client emits via `SGWNetworkManager::EventHandler<Event_NetOut_SendMailMessage>` (vtable at `01e2bb20`); the field list comes from the entity def (authoritative for wire shape since it is the data the BigWorld code-gen reads).
- Client behavioral log: n/a (mail send is a UI path; no log emitted by the QA client without a UI-triggered send).
- Cross-ref to Rust handler (for fix author): `crates/services/src/cell/cell_methods/mail.rs:32-35` — `SEND_MAIL_MESSAGE` arm is a single `tracing::info!("UNIMPLEMENTED: sendMailMessage")` plus `return true`.

**Attack scenario** (once implemented without validation)
1. Modified client crafts `Event_NetOut_SendMailMessage` with `Recipients = ["self_alt_char"]`, `Cash = i32::MAX`, `ItemId = <equipped weapon's item_id>`, `ItemQuantity = 1`, `bCOD = 0`.
2. Server inserts the mail row and (in a naive implementation) credits the recipient on take without ever debiting the sender, or removes the item from the sender then writes the recipient row, but uses `type_id` instead of `item_id` and grants the alt a clone of the source weapon.
3. Observable effect: cash dupe, item dupe, or cross-account transfer with no audit trail.

**Suggested remediation (one line)**
Before implementing `sendMailMessage`, file the validation contract: (a) recipient names resolved via DB, not client; (b) `Cash` clamped non-negative and atomically debited from `sgw_player.naquadah` in the same transaction as the `INSERT INTO sgw_gate_mail`; (c) attachment item moved by `item_id` (the row PK), never by `type_id`; (d) `Recipients` array bounded to e.g. 10 entries; (e) per-recipient mailbox-full check before insert; (f) atomic-or-rollback semantics on partial failure; (g) send a `sendMailResult` packet on every code path (success and per-recipient failure list).

**Would benefit from x64dbg trace?**
Yes — to lock down the exact wire byte layout (in particular WSTRING length-prefix encoding and the ARRAY<WSTRING> serialization for `Recipients`) before implementation, so the eventual decoder doesn't drift from the client.

---

### CAT-G-02 — takeCashFromMailMessage handler is unimplemented; no atomicity model defined

**Severity**: High (latent — becomes Critical when filled in without atomicity)
**Class**: Missing handler / future TOCTOU on currency mutation
**Wire surface**: `Event_NetOut_TakeCashFromMailMessage` (CellMethod index 49)
**Demonstrable / Likely-theoretical**: Demonstrable (no withdraw path exists)

**Trust violation**
The client emits `Event_NetOut_TakeCashFromMailMessage` with a single
client-supplied `MailId: INT32` (per `SGWMailManager.def:85-88`). The server
logs `UNIMPLEMENTED: takeCashFromMailMessage` and returns. There is no
ownership check, no debit/credit, no zeroing of the `cash` column to prevent
re-take, and no transactional guard against the take-cash + take-item
double-claim window (see CAT-G-04).

**Evidence**
- Ghidra: `019bf380` — string `Event_NetOut_TakeCashFromMailMessage`; registered NetOut handler at `00d68150` family.
- Client behavioral log: n/a.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/mail.rs:68-74`.

**Attack scenario** (once implemented without atomicity)
1. Client sends `takeCashFromMailMessage(mail_id=N)` twice in rapid succession before the server's "set cash=0" UPDATE has committed.
2. Server reads `cash=500` twice, credits player twice (`+1000`), then both UPDATEs zero the row.
3. Observable effect: cash dupe equal to the attachment value × concurrency.

**Suggested remediation (one line)**
Implement as a single `UPDATE sgw_gate_mail SET cash=0 WHERE mail_id=$1 AND character_id=$2 AND cash>0 RETURNING cash` and credit the player only if `rows_affected == 1` and `RETURNING cash > 0` — the row-level lock + conditional `cash>0` makes the second concurrent request a no-op.

**Would benefit from x64dbg trace?**
No — the wire shape is trivially a single INT32; the work is purely server-side design.

---

### CAT-G-03 — takeItemFromMailMessage handler is unimplemented; no inventory-full / ownership / item_id checks

**Severity**: High (latent — Critical when implemented naively)
**Class**: Missing handler / future item dupe via TOCTOU
**Wire surface**: `Event_NetOut_TakeItemFromMailMessage` (CellMethod index 50)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The client sends `MailId: INT32`, `ContainerId: INT32`, `SlotId: INT32`
(per `SGWMailManager.def:89-94`). The dispatcher decodes all three from
client bytes and logs UNIMPLEMENTED. `ContainerId` and `SlotId` are
client-asserted destination coordinates for the moved item — these MUST
be validated against the caller's actual container/slot table and against
inventory-full at the destination, not blindly used. The mail row's
`item_id` is the authoritative source PK and must be the key for the
move, not the (mail_id, type_id) pair (see the canonical TOCTOU pattern
recorded for inventory mutations — bandolier same-type swap).

**Evidence**
- Ghidra: `019bf3a8` — string `Event_NetOut_TakeItemFromMailMessage`; registered NetOut handler at `00d68210`.
- Client behavioral log: n/a.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/mail.rs:75-89`.

**Attack scenario** (once implemented without item_id-keyed move + inventory-full check)
1. Client sends `takeItemFromMailMessage(mail_id, container_id=<bandolier>, slot_id=<an occupied slot of same type>)`.
2. Naive implementation overwrites the destination slot's item record using `type_id` matching, losing or cloning the equipped weapon's item row.
3. Observable effect: ammo dupe / item overwrite / item dupe.

**Suggested remediation (one line)**
Move the item by `sgw_gate_mail.item_id` (DB row PK) into a `(container_id, slot_id)` validated against the caller's owned containers and against destination slot vacancy in a single transaction; null out `sgw_gate_mail.item_id` in the same UPDATE; consult `social-systems-engineer` for the mail-item state machine and item-lock invariants shared with trade/auction.

**Would benefit from x64dbg trace?**
Yes — to confirm whether the client's `ContainerId` is an int matching `sgw_player_containers.container_id` or a logical bandolier slot index, since the spec doesn't disambiguate.

---

### CAT-G-04 — Mail attachment double-take: take-cash + take-item flows are not coupled

**Severity**: High (latent — emerges as soon as both handlers are filled in independently)
**Class**: TOCTOU / cross-handler race on the same row
**Wire surface**: `Event_NetOut_TakeCashFromMailMessage` + `Event_NetOut_TakeItemFromMailMessage`
**Demonstrable / Likely-theoretical**: Likely-theoretical (both are unimplemented today; the dispatcher decoders treat them as two unrelated handlers; if implemented as two independent UPDATEs they will race)

**Trust violation**
The two take-paths share one DB row (`sgw_gate_mail.cash` + `sgw_gate_mail.item_id`).
A correctly-implemented mail system must coordinate them: either by
zeroing both columns in a single UPDATE on first-take, or by an
explicit "claim" semaphore (e.g., `flags` bit "attachments_collected").
The current dispatcher decodes the two as unrelated indices (49 vs 50)
and forwards independently; nothing in the codebase yet ties them
together. A future implementer who writes one path and then the other
without re-reading both at the same time will produce a race in which
a fast client sends both back-to-back and the server processes them
against two reads of the same row.

**Evidence**
- Ghidra: `Event_NetOut_TakeCashFromMailMessage` at `019bf380`, `Event_NetOut_TakeItemFromMailMessage` at `019bf3a8` — two distinct client events with no shared client-side guard against double-emission. Client code path is per-attachment UI button.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/mail.rs:68-89` — two independent arms; no shared `with_mail_lock(mail_id, ...)` pattern.

**Attack scenario** (after independent implementation)
1. Mail has both cash=500 and item_id=X attached.
2. Client sends both `takeCashFromMailMessage(N)` and `takeItemFromMailMessage(N, c, s)` in the same Mercury bundle.
3. If handlers commit in two transactions and the cash-zero UPDATE races with the item-null UPDATE, one or both can succeed without the other; worse, a third take-cash request in the same bundle could find cash still set.

**Suggested remediation (one line)**
Both handlers should `BEGIN; SELECT ... FOR UPDATE; UPDATE; COMMIT` on the same row, and the per-attachment fields should be cleared in the same statement — or introduce a `flags` bit `ATTACHMENTS_COLLECTED` that gates both takes and is set in the same UPDATE that moves the last attachment.

**Would benefit from x64dbg trace?**
No — pure server-side state-machine design.

---

### CAT-G-05 — payCODForMailMessage handler is unimplemented; client can later supply $0 COD

**Severity**: High (latent — Critical if implementer trusts a client-supplied amount)
**Class**: Missing handler / price-from-client-not-from-row
**Wire surface**: `Event_NetOut_PayCODForMailMessage` (CellMethod index 51)
**Demonstrable / Likely-theoretical**: Likely-theoretical (handler unimplemented today, but the def shows the client only sends `MailId`)

**Trust violation**
The wire-defined args list for `payCODForMailMessage` is `MailId: INT32`
only (per `SGWMailManager.def:95-98`) — meaning the COD amount itself
is NOT client-supplied; it must come from the row's `cash` column
(re-purposed as COD when the `bCOD` flag was set at send time). If
the future implementer instead reads any payload-supplied price (e.g.
by widening the args list to match a "natural" expectation), the
client can claim attachments for free.

**Evidence**
- Ghidra: `019bf3d0` — string `Event_NetOut_PayCODForMailMessage`; registered NetOut handler at `00d68230`. Entity-def args list (single INT32) at `entities/defs/interfaces/SGWMailManager.def:95-98`.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/mail.rs:90-95`.

**Attack scenario** (after implementation that trusts client price)
1. Modified client sends `payCODForMailMessage(mail_id)` with a widened payload that includes `client_cod_amount=0`.
2. If the server reads the client-supplied amount instead of `sgw_gate_mail.cash`, the claimer pays $0 and receives the attachment; the original sender is credited $0.
3. Observable effect: COD bypass / fraud across the player economy.

**Suggested remediation (one line)**
The COD amount must be read from `sgw_gate_mail.cash WHERE mail_id=$1 AND character_id=$caller AND (flags & COD_FLAG) <> 0` and used as the authoritative price; the caller's cash must be debited and the sender's credited atomically with the attachment move, in one DB transaction with row-level locks; reject if the claimer has insufficient cash before any write.

**Would benefit from x64dbg trace?**
No — the wire shape (single INT32) is fixed by the def.

---

### CAT-G-06 — returnMailMessage handler is unimplemented; future bug-shape is attachment-loss or original-sender substitution

**Severity**: Medium (latent — Critical only if implementer ignores `sender_id` resolution)
**Class**: Missing handler / future trust-on-client-for-sender
**Wire surface**: `Event_NetOut_ReturnMailMessage` (CellMethod index 47)
**Demonstrable / Likely-theoretical**: Demonstrable that the handler is missing.

**Trust violation**
`returnMailMessage(MailId)` reads only the client-supplied mail_id (per
`SGWMailManager.def:75-78`). The destination of the return is the row's
`sender_id` column — never a client-supplied identifier. A future
implementer who reaches for `sgw_gate_mail.sender_name` (a free-form
WSTRING that may be stale or never validated at send-time, depending on
how Send is implemented) instead of `sender_id` will route the return
to the wrong character. Additionally, if the return path moves
attachments back to the original sender without atomicity (item move +
cash credit + delete original row), the attachments can be lost on a
partial failure.

**Evidence**
- Ghidra: `019bf360` — string `Event_NetOut_ReturnMailMessage`; registered NetOut handler at `00d681d0`.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/mail.rs:52-58`.

**Attack scenario**
1. Player A receives mail from Player B (cash+item attached). A modifies their client to send `returnMailMessage(N)`.
2. If the return implementation creates a new mail row addressed to `sender_name` (string-keyed) and another character with that name (or a near-collision via Unicode normalization) exists, the attachments can be redirected.
3. Observable effect: cross-account attachment redirection if `sender_name` is taken as authority instead of `sender_id`.

**Suggested remediation (one line)**
Implement return as `INSERT ... SELECT` keyed on `sender_id` (integer PK) of the original mail row, never on `sender_name`; wrap the original-row delete and the new-row insert in a single transaction; reject if `sender_id IS NULL` (system mail).

**Would benefit from x64dbg trace?**
No — wire shape is single INT32.

---

### CAT-G-07 — RequestMailBody read_time UPDATE lacks character_id filter (defense-in-depth)

**Severity**: Low
**Class**: Defense-in-depth gap on a multi-statement query
**Wire surface**: `Event_NetOut_RequestMailBody` (CellMethod index 48)
**Demonstrable / Likely-theoretical**: Likely-theoretical (mitigated upstream by the SELECT, but the UPDATE alone is unscoped)

**Trust violation**
In `crates/services/src/base/world_entry/methods/mail/mod.rs:191-200`
the read-time stamp UPDATE is:

```sql
UPDATE sgw_gate_mail SET read_time = $1 WHERE mail_id = $2 AND read_time = 0
```

— **no `character_id = $player_id` filter**. The UPDATE is currently
gated by the preceding SELECT (which IS scoped by `character_id`)
returning `Some`, so today this is unreachable for foreign mailboxes.
But it's a TOCTOU-shaped gap: if the SELECT scoping is ever refactored
away (e.g. a future change reads "headers cache" first), the UPDATE
will silently mark other players' mail as read.

**Evidence**
- Decoder in `cell/cell_methods/mail.rs:59-67` accepts `mail_id` from client bytes with no validation.
- Cross-ref to Rust handler: `crates/services/src/base/world_entry/methods/mail/mod.rs:155-200`.

**Attack scenario** (regression-shaped)
1. A future refactor moves the SELECT into a cache lookup or drops the `AND character_id = $2` clause.
2. Client iterates `mail_id` from 1..N and force-marks every player's unread mail as read.
3. Observable effect: cross-mailbox griefing (unread-flag denial of service) — does not leak content, just resets the visual unread indicator.

**Suggested remediation (one line)**
Add `AND character_id = $3` to the UPDATE and bind `player_id` — the UPDATE then defends itself, independent of the SELECT.

**Would benefit from x64dbg trace?**
No.

---

### CAT-G-08 — onMailRead `ToText` filled with caller's own name (info leak nil, but spec drift)

**Severity**: Low (information-quality drift, not a security issue)
**Class**: Spec/wire mismatch
**Wire surface**: `onMailRead` server→client response (ClientMethod index 78)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The wire `onMailRead.ToText` field is documented as "recipient name"
in `entities/defs/interfaces/SGWMailManager.def:42` (and in
`crates/services/src/cell/mail.rs:182-201`). The handler at
`base/world_entry/methods/mail/mod.rs:202-209` fills it from
`lookup_player_name()`, which returns the **reader's** name (i.e.
the caller's own player_name), not the recipient the mail was
originally addressed to. For self-read mail the value is correct
incidentally. For any future cross-account or system-mail case
where reader != recipient, the field is wrong.

This is not a security finding by itself (the reader is shown their
own name in their own mail UI; no foreign data leaks). It is filed
because a future implementer will look at this code, conclude
"ToText is already wired", and not notice that it doesn't actually
reflect the original `Recipients` array. When the client UI uses
ToText for "Reply To" semantics, the wrong field can populate the
reply target.

**Evidence**
- Entity def: `entities/defs/interfaces/SGWMailManager.def:42` says `ToText` is recipient.
- Cross-ref to Rust handler: `crates/services/src/base/world_entry/methods/mail/mod.rs:202-209` — uses caller's own name.

**Attack scenario**
None directly exploitable today.

**Suggested remediation (one line)**
Populate `ToText` from the original mail row's stored recipient name
once `sendMailMessage` persists `Recipients` (add column `recipient_name`
to `sgw_gate_mail` or use the existing `character_id`→`sgw_player.player_name`
join), and revisit the field's intended meaning with social-systems-engineer.

**Would benefit from x64dbg trace?**
No.

---

## Not Filed

- **`b_archive` not applied as filter in RequestMailHeaders query.** Headers SELECT does not filter by archive flag — returns all mail regardless of client's `bArchive` byte. This is a functional/UX bug (archive folder shows inbox content), not a security issue: it doesn't leak foreign-player data because `character_id = $player_id` is enforced.
- **`onMailHeaderInfo` always emits empty `MessageAttachments` array.** `serialize_on_mail_header_info` hard-codes `attachments count = 0u32` regardless of the row's `cash` / `item_id`. Feature gap (attachments UI cannot show on header), not a security issue.
- **`if args.len() >= 4` decoder silently drops malformed cell-method args.** Every short-payload mail handler returns without acknowledging the bad packet to the client. Not exploitable — the client has no observable signal it can act on; only operator-side logging is affected, and the dispatcher's framing layer upstream already enforces packet integrity.
- **`lookup_player_name` two-mutex lookup race.** `entity_to_addr` is released before `connected` is acquired. Worst case is an empty player_name string in `onMailRead.ToText` if disconnect interleaves. Not exploitable; covered by CAT-G-08 from a separate angle.
- **`sgw_gate_mail` lacks a `recipient_name` column or any uniqueness on `(character_id, sent_time)`.** Schema-level gap that will matter for sendMailMessage but is upstream of any current handler. Out of scope for category G's per-handler audit; route to the social-systems-engineer when send is implemented.
- **Read_time `cash` truncation warning at headers query** (`mod.rs:100-107`). DB column is `bigint`, wire is `i32`. The clamp+warn path is correct; no exploit.
- **Entity-id from client decoded then discarded.** `cell_arms.rs:87-88` reads `entity_id_from_client` but `cell_arms.rs:121-138` forwards `player_eid` from the connected session. Server-authority is intact; the read is for diagnostic logging only. Filing the absence-of-bug for context, not as a finding.
