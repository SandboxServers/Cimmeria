# PT-E1 Worknotes

> Type: reference. Audience: pets-campaign coordinator (session cimmeria-b5).
> Companions: `README.md`, `work-packets.md`, `audit.md`, `research/client-static-re.md`,
> `research/code-map.md` — all read from `origin/docs/pets-campaign-plan` (this ledger directory
> does not exist on `main` yet, so this worknotes file is the only file this packet adds under
> `docs/analysis/pets/`).

## Contract

- **Packet:** PT-E1 — pet client contract, closing the static gaps.
- **Scope:** (1) confirm/correct the SGWPet client method wire indices; (2) find what fills
  `Unit.Pet1..Pet4` and name the exact server message(s); (3) the `SpeedPet` consumer, if
  statically findable; (4) write `pet-client-contract.md`, fix the two INT8 errors in
  `pet-wire-formats.md` and the "idx 0/1/2" claim in `pet-restoration.md`, add README index rows,
  add an SGWPet table to `client-method-dispatch-table.md` if indices confirmed.
- **Decisions in force:** none of D-PT01/02/03 gate this packet (client-side static RE only).
- **Depends on:** nothing (Wave 0, parallel with PT-01/PT-S).
- **Source revision/base:** `origin/main` @ `95366c59` (branch created from this commit; `origin/main`
  has since advanced to `09a880ba` in the shared repo; the packet's first pass made no code changes and
  needed no rebase. It was later rebased onto main by the coordinator; see the Log — matches the base the campaign's own `audit.md` was written against).
- **Owned paths (this session):**
  - `docs/reverse-engineering/findings/pet-client-contract.md` (new)
  - `docs/reverse-engineering/findings/pet-restoration.md` (correction only)
  - `docs/reverse-engineering/findings/pet-wire-formats.md` (correction only)
  - `docs/reverse-engineering/findings/README.md` (index row + count)
  - `docs/reverse-engineering/README.md` (index count/description)
  - `docs/protocol/client-method-dispatch-table.md` (new SGWPet section)
  - `docs/analysis/pets/worknotes/pt-e1.md` (this file)
- **Read set:** `PETS-WORKER-RULES.md`; the pets ledger on `docs/pets-campaign-plan`
  (`work-packets.md` §PT-E1, `README.md`, `audit.md` A-01..A-13, `research/client-static-re.md`
  §A/§E/§H, `research/code-map.md` §1.4-1.5); `entities/defs/SGWPet.def`,
  `entities/defs/enumerations.xml` (`EEntityFlags`, `GENERICPROPERTY_PetOwnerId`);
  `docs/protocol/client-method-dispatch-table.md` (SGWMob table); `docs/reverse-engineering/findings/
  dialog-portrait-lookup.md` (the `FUN_00c67bd0` slot-mapping family this packet extends);
  `crates/mercury/src/channel_bundle/idbase.rs`. No Rust source outside `entities/defs/` was edited
  or needed — this packet is static Ghidra RE plus docs.
- **Tools used:** Ghidra MCP only (decompile/disassemble/xrefs/rename/comment against the live
  `SGW.exe` project). No x64dbg attach — everything below was closed statically, so the
  packet's fallback ("if (2) cannot be closed statically, list the non-freezing capture recipe")
  did not need to be exercised.

## Hypothesis / evidence

### 1. Wire method indices — derivation stands; handler identity cross-checked

`code-map.md §1.5` had already derived 29/30/31 from the `.def` + BigWorld flattening rule and
flagged "one verification step" as outstanding. What I actually closed is narrower than a full
index verification: each of the three GamePet CME handlers
(`GamePet__OnPetAbilityListChanged` @ `0x00d39eb0`, `GamePet__OnPetStanceListChanged` @
`0x00d3a070`, `GamePet__OnPetStanceUpdateChanged` @ `0x00d3a260`) keys its internal
property-list parse on the literal strings `"aAbilityList"`, `"aStanceList"`, `"aStance"` — an
exact match to `SGWPet.def`'s three `<ArgName>` values. **That confirms which method each
handler implements** (so the three renames above are correctly assigned to each other), **not
the numeric wire index 29/30/31 itself** — the indices still rest entirely on the `.def` parse
order plus the BigWorld flattening rule; I did not cross-check them against a live
`EntityDescription` dump or dispatch table.

### 2. `Unit.Pet1..4` binding mechanism — CONFIRMED

Full call chain traced and cross-confirmed two independent ways (see
`pet-client-contract.md` §2 for the complete evidence and code excerpts):

- **Slot IDs**: `Unit.Pet1..4` = GameEntityManager slots **10, 11, 12, 13**. Confirmed via (a) a
  Lua numeric-constant table at `0x01b160c0`+ read directly with `read_memory`, and (b)
  independently via the actual slot-bind loop in `GamePet__SyncLocalOwnerPetSlots` (`0x00d39880`,
  was `FUN_00d39880`), which loops `slot = 10..13` calling the same `GameEntityManager` slot-map
  function already documented in `dialog-portrait-lookup.md` (`0x00c67bd0`).
- **Trigger**: the pet's `ownerID` generic property (`GENERICPROPERTY_PetOwnerId = 5`, per
  `enumerations.xml:1727`) changing. `GamePet__OnOwnerIdChanged_ValuePushed` (`0x00d39a10`) is
  gated on `ENTITYFLAG_Pet` (bit `0x400` = 1024) and calls `GameBeing__AddPetId`/
  `GameBeing__RemovePetId` (`0x00e007b0`/`0x00e00270`) on the resolved owner's `GameBeing`,
  which is the array `GamePet__SyncLocalOwnerPetSlots` later reads from
  (`GameBeing__GetPetIdAt`, `0x00dffa80`, over `GameBeing+0x144..0x148`). A second handler
  (`LAB_00d39ae0`, inline in `GamePet__SubscribeEvents` @ `0x00d3a3d0`) re-reads that same
  property via `GameEntityBase__GetGenericPropertyInt32(this, 5)` (renamed from `FUN_00e6dd30`,
  confirmed as a generic-property-map getter by decompile) to decide whether to run the slot
  refresh.
- **The exact readiness gate**: three bytes on the `GamePet` object gate the refresh
  (`+0x170`/`+0x171`/`+0x172`, all must be non-zero). `+0x170` is a constant "IsPet" marker set
  unconditionally in `GamePet__ctor` — not a real gate. `+0x171` is set by the ownerID handler
  above — the only handler that actually adds the pet id to the owner's pet vector — and
  *also*, as an unrelated side effect, inside `GamePet__OnPetAbilityListChanged`'s
  successful-parse path; that side effect does **not** itself establish ownership
  (`GamePet__OnPetAbilityListChanged` never calls `GameBeing__AddPetId`/`RemovePetId`), so
  `onPetAbilityList` cannot substitute for the ownerID property — `GamePet__SyncLocalOwnerPetSlots`'s
  own re-check still needs the ownerID property to already be valid when it runs. `+0x172` is
  set **only** by `GamePet__OnPetStanceListChanged`. So the real requirement is:
  **`onEntityProperty(GENERICPROPERTY_PetOwnerId=5, owner)` AND `onPetStanceList`** (the
  ability-list message is separately required to fill the ability bar, but is not
  interchangeable with the ownerID property for slot binding). The owner property must reach
  the client **after** `onEntityFlags` has set `ENTITYFLAG_Pet`: the owner handler reads the
  entity's current flag state when the property event fires, so a flag set later is ignored.
  `onPetStanceUpdate` does **not** gate this at all — it only updates the cached current-stance
  byte (`+0x173`) and fires a UI event. This directly answers PT-01's
  `pet_create_on_client_events` message-sequence question (`code-map.md §1.4` item 2) and
  resolves one of `pet-restoration.md`'s open questions (does `onPetStanceUpdate` matter for
  basic pet recognition? — no).

Confidence: HIGH for the slot IDs and the ownerID mechanism (each traced to a specific
decompiled handler with a literal `.def`-matching string or constant). MEDIUM for the exact
semantic label "readiness flag" on `+0x171`/`+0x172` — confirmed as gating bytes, not proven to
have no other consumer elsewhere in the object.

### 3. `SpeedPet` (16384) — NOT resolved, matches prior pass

No named function or string references `SpeedGrenade`/`SpeedDeploy`/`SpeedAttack`/`SpeedPet`
anywhere in the binary (pure bitfield values, no RTTI/string trace). The consumer is the shared
ability cast-time/warmup function, which was not located this pass (out of budget — it's not
adjacent to any GamePet-specific code and is a much larger, unrelated search surface). Left as
an open item exactly as `client-static-re.md §E` already stated; D-PT10's design default is
unaffected.

## Design decisions

- **Fixed the byte-offset transcription bug in the ctor read.** `pet-restoration.md`'s Ghidra
  notes read `GamePet__ctor`'s `param_1[0x5c] = 1` as raw byte offset `0x5c`. `param_1` is
  `undefined4*` in that decompile, so pointer arithmetic `param_1 + 0x5c` is byte offset
  `0x5c * 4 = 0x170`. Documented the correct offset (`0x170`) in the new finding and explained
  the transcription source of the old number, rather than silently overwriting it, so a future
  reader isn't confused by the discrepancy between the two docs.
- **Applied Ghidra renames and plate comments** for every standalone function in the traced call
  chain (11 renames — full list in `pet-client-contract.md` §"Ghidra annotations applied this
  session"), so the next investigator doesn't have to re-derive `FUN_00d39880` et al. from
  scratch. Did **not** attempt to rename the three inline `LAB_00d39ae0`/`LAB_00d39600`/
  `LAB_00d39d80` labels — they're compiler-generated callback thunks inside
  `GamePet__SubscribeEvents`'s body, not addressable as independent functions without first
  splitting them out (a heavier, riskier Ghidra operation this packet's scope didn't call for).
- **Did not correct `dialog-portrait-lookup.md`'s DialogSpeaker/Dialog slot-value swap.** The
  same Lua constant table that gave me `Pet1..4` also gives `Dialog = 17` and `DialogSpeaker =
  18`, which conflicts with that doc's "slot 17 = DialogSpeaker" claim. Flagged in
  `pet-client-contract.md` §5 as a side finding for whoever next touches dialog UI work, rather
  than editing a file outside this packet's ownership (dialog UI is other campaign work, not
  pets).
- **Did not touch `pet-restoration.md`'s open-questions list** beyond the one line the packet
  named (the "idx 0/1/2" correction). Question 6 there ("onPetStanceList element meaning") is
  already resolved in the not-yet-merged `client-static-re.md §B`, but rewriting that list is
  outside this packet's explicit scope and risks conflicting with whatever PT-01/coordinator
  does when the ledger merges.

## Deferred / gaps

- `SpeedPet` consumer (item 3) — open, needs the shared ability-resolution/warmup decompile.
- Exact semantics of `LAB_00d39600` (owner-destroyed reaction) and `LAB_00d39d80` (manipulates a
  second array at `+0x17c`, plausibly `toggledAbilities`) — briefly examined, not fully traced;
  noted as open questions in `pet-client-contract.md` since they don't affect the `Unit.PetN`
  bind question this packet was scoped to close.
- The `dialog-portrait-lookup.md` slot-value discrepancy (§5 of the new finding) is flagged, not
  fixed.

## No runtime code; one tooling fix

The RE itself is static analysis plus documentation, and no runtime Rust was written. One tooling
change came out of the #863 review: `tools/wire_decoder_codegen.py` now stops at the first
per-entity dispatch section, so the SGWPet rows 30/31 no longer generate duplicate
`decode_30`/`decode_31` next to SGWPlayer's. It was verified by regenerating
`crates/wire-log/src/wire_log/decoders/generated.rs` (byte-identical to the committed file after
`cargo fmt`) and running `cargo check` plus the 24 tests of `cimmeria-wire-log`. `PETS-WORKER-RULES.md`'s "regression guard must fail when reverted"
requirement does not apply to a documentation-only packet; the closest analogue is that every
claim in `pet-client-contract.md` cites a specific address and, where a decompiled snippet is
shown, is directly re-checkable in Ghidra by anyone who doubts it.

## Log

- **2026-09-27** — Read the worker rules, fetched the ledger docs from
  `origin/docs/pets-campaign-plan` (work-packets.md, README.md, audit.md, client-static-re.md,
  code-map.md — none of these exist on `main` yet). Read `SGWPet.def`, the SGWMob dispatch
  table, and `dialog-portrait-lookup.md` for the `FUN_00c67bd0` slot-mapping pattern. In Ghidra:
  traced the `Unit.PetN` Lua-constant registration table back to a contiguous dword array at
  `0x01b160c0`+ (read its raw values directly); found and decompiled the exact slot-bind loop
  (`GamePet__SyncLocalOwnerPetSlots`); traced the ownerID property-change handler chain
  (`GamePet__OnOwnerIdChanged_ValuePushed`, `GameBeing__AddPetId`/`RemovePetId`,
  `GameEntityBase__GetGenericPropertyInt32`); decompiled all three `GamePet__SubscribeEvents`
  handlers with `.def`-matching property keys to confirm each handler's identity (the numeric
  indices 29/30/31 rest on the `.def` declaration order, not on this trace) and isolate the
  three-byte readiness gate; decompiled `GamePet__ctor` directly to correct the byte-offset
  transcription bug. Applied 11 Ghidra renames + 2 plate comments. Searched for a `SpeedPet`
  consumer; came up empty, matching the existing static-RE doc. Wrote
  `pet-client-contract.md`; fixed `pet-wire-formats.md`'s two INT8 errors and
  `pet-restoration.md`'s "idx 0/1/2" line; added the SGWPet table to
  `client-method-dispatch-table.md`; updated both `docs/reverse-engineering/` README index files
  (summary counts corrected to 78 docs after the Copilot review; the directory holds 78 findings). Wrote this worknote. `cargo fmt`/`clippy`/build/test were not run — no Rust
  changed. Committed and pushed.
- **2026-09-27 (coordinator)** — Rebased onto `main` after the ledger PR #879 merged. The README counts are now 81, because main had added findings meanwhile. Per the Copilot re-review, the "No runtime code; one tooling fix" section now records the `wire_decoder_codegen.py` change, and the Ghidra log entry now scopes the handler trace to handler identity only.
