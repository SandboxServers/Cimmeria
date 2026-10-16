# Native slash palette: typed metadata slices

This reference covers two disabled, typed slices from `data/client-patches/012-gm-slash-commands/commands.toml`: entries 28–54 from [DW-25B-E](evidence/DW-25B-E.md) in `crates/lab/src/workbench/slash_descriptors_1b.rs`, and entries 109–135 from [DW-27A-E](evidence/DW-27A-E.md) in `crates/lab/src/workbench/slash_descriptors_3a.rs`. Every entry remains unavailable until the native client event-to-method binding, serialization and accepted receipt are proved. Patch `access = "p"` is a client filter; the server checks GameMaster authority separately. A submitted line never establishes server acceptance or effect.

## Entries 28–54 (DW-25B)

| Words | Palette category | Source-backed behavior or blocker |
|---|---|---|
| `/gmgiverespawner`, `/gmgiveracialparadigmlevels` | Grant | No implemented handler; target and state change unknown. |
| `/gmgiveexpertise`, `/gmgiveappliedsciencepoints` | Crafting grant | Caller-only, durable Base grants after cell forwarding. The first may create a discipline and caps expertise at 100; the second saturates the points balance. Compare DB rows keyed by caller player_id and discipline_id after Base result, not after enqueue. |
| `/gmsetgodmode` | Combat | Caller-only transient toggle; Boolean native encoding and independent combat proof pending. |
| `/gmsetnoxp`, `/gmsetnodamage`, `/gmsetnoaggro`, `/gmsetspeed` | Progression, combat, movement | No handlers. The two zero-argument forms have no invented Boolean input. Speed range and target are unknown. |
| `/gmsethealth`, `/gmsethealthmax`, `/gmsetfocus`, `/gmsetfocusmax` | Stats | Explicit same-space `TargetId` resolved from the argument, never from selection; zero means caller. Amount is bounded to signed INT32; TOML Integer is not proof of cell INT64 TargetId encoding. Compare live current/max stats, accounting for clamping. |
| `/gmsetflag`, `/gmsetlevel` | State, progression | No safe handler contract; counted flags and derived level state cannot be guessed from primitives. |
| `/gmresetabilities`, `/gmgiveallabilities` | Abilities | Durable caller-character changes. Capture ability and tree state plus sgw_player_ability_grants rows keyed by player_id and ability_id, including source_kind/source_id, before Base result. Reset is not the precise inverse of an earlier grant. |
| `/gmrespec`, `/gmsetmobstance` | Abilities, NPC | No GM handler; player trainer reset and AI state are separate mechanisms. |
| `/gmsettarget` | Target | Candidate server handler accepts nonnegative signed INT32 numeric text only, despite TOML `NameOrID`; zero clears caller target. The typed form rejects names and out-of-range IDs. |
| `/gmsetmobabilityset` | NPC | TOML's `abilitySetId` is optional without a default; candidate server handler requires INT32 bytes. Existing entity readback cannot verify known abilities or set ID. |
| `/gmdhd`, `/gmgotolocation` | Travel | GateTravel can persist world, position and learned gate IDs after arrival. The typed WorldName must be nonempty after trimming. Immediate feedback is only a request-stage observation. |
| `/gmgoto`, `/gmsummon`, `/gmgotoxyz` | Travel | Same-space moves can persist player position conditionally. `TargetName` is a required numeric ID string resolved from the argument, never from selection, in the candidate handlers; NPC summon has no player DB row. |
| `/gmreloadorganizations` | Organizations | Caller Team/Command client-state re-push, potentially partial. It changes no organization DB row but is a transient effect, not a read. A client organization-state capture is pending; DB memberships and push feedback alone cannot verify that the client applied the state. |

All 27 forms preserve patch parameter names, order, primitive types and required flags. The optional ability-set ID has no fabricated default. Unsupported forms have a command-specific reason, impact and before/after recipe where the candidate server handler supports one, and no claimed readback where it does not. No raw command box or dot-command substitution is provided for this slice.

## Entries 109–135 (DW-27A)

The fourth 27-word patch slice is implemented in `slash_descriptors_3a.rs` and grounded in [DW-27A-E](evidence/DW-27A-E.md). Every form is typed but disabled. No event-to-method binding, native Boolean omission/encoding, accepted server receipt or effect readback is proven. No slash form routes to a dot command.

| Words | Scope and operator consequence | Evidence limit |
|---|---|---|
| `/gmregeneratecoverlinks`, `/gmchangecoverweight`, `/gmchangecoverstanceweight` | Potentially disruptive cover graph or scoring change across an unproved world/space scope. No inverse can restore prior links or weights. | Methods 223–225 have no server handlers or graph/weight readback. `maxLinks` is UINT32 at the method surface with no safe bound; stance weights are TOML Float but method WSTRING. |
| `/gmhelpfull`, `/gmtestsequence`, `/gmdebugflash`, `/gmshowlog` | Client help, test, Flash and log effects are either unknown or require direct UI observation. | No server state read proves a client-only display. Optional `/gmshowlog` Boolean has no established omitted default. |
| `/gmrepairitem`, `/gmgiveammo`, `/gmgiveinventory`, `/gmgivegearset` | Possible item-instance or bulk-inventory changes, with no safe grant/repair recipe. | Repair method 40 is a stub; working vendor repair and dot ammo/item commands are different executors. |
| `/gmgivefaction`, `/gmsetfaction`, `/gminvokeability`, `/gmsettechskill`, `/gmgiveblueprint` | Recipient, amount/ID or server effect is unknown. | No target is inferred from current selection and no readback is advertised. |
| `/gmdebugminigameinstance` | The three required signed INT32 values identify a caller debug request. | Method 23 is GM gated but only logs an unimplemented stub; that log does not prove an instance opened. |
| `/gmsetinfiniteammo`, `/gmsetinvulnerable`, `/gmsetomnipotent`, `/gmsetignorefocus`, `/gmsetignorehealth` | Combat/debug names do not establish a server toggle or reversible state. | Dot `.infiniteammo` and method `gmSetGodMode` are distinct; optional Booleans have unknown omission semantics. |
| `/gmsetfly`, `/gmsetghost` | Caller-local physics can pair with one transient server movement-validation bypass. Both share method 221. | Native Boolean-to-wire polarity is unproved; method byte 0 disables validation and byte 1 restores it. Current entity readback does not expose the bypass flag. |
| `/gmsetspectator`, `/gmsetnotarget` | Client and observer effects are unknown. | No proven AI, AoI, camera or targetability change. |
| `/gmsetpvp` | At most a player flag request; it does not grant attack rights. Duel combat remains gated by an engaged `DuelRegistry` pair. | `setPvPFlag` is internal, not GM callable. No accepted slash handler or authoritative PvP setter receipt exists. |

The typed inputs preserve all patch parameter names, types, order and optional flags. The three cover forms are marked potentially irreversible, with no compensation recipe. Every row is unsupported with a command-specific reason; no row claims a verified readback from a generic entity or DB query.
