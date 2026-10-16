# Native slash palette: entries 28–54

This reference describes the disabled, typed forms built by DW-25B from patch 012. The source inventory is `data/client-patches/012-gm-slash-commands/commands.toml`; the per-command evidence is [DW-25B-E](evidence/DW-25B-E.md). The Rust slice is `crates/lab/src/workbench/slash_descriptors_1b.rs`. Every entry remains unavailable until the native client event-to-method binding, serialization and accepted receipt are proved. Patch `access = "p"` is a client filter; the server checks GameMaster authority separately. A submitted line never establishes server acceptance or effect.

| Words | Palette category | Source-backed behavior or blocker |
|---|---|---|
| `/gmgiverespawner`, `/gmgiveracialparadigmlevels` | Grant | No implemented handler; target and state change unknown. |
| `/gmgiveexpertise`, `/gmgiveappliedsciencepoints` | Crafting grant | Caller-only, durable Base grants after cell forwarding. The first may create a discipline and caps expertise at 100; the second saturates the points balance. Compare DB rows after Base result, not after enqueue. |
| `/gmsetgodmode` | Combat | Caller-only transient toggle; Boolean native encoding and independent combat proof pending. |
| `/gmsetnoxp`, `/gmsetnodamage`, `/gmsetnoaggro`, `/gmsetspeed` | Progression, combat, movement | No handlers. The two zero-argument forms have no invented Boolean input. Speed range and target are unknown. |
| `/gmsethealth`, `/gmsethealthmax`, `/gmsetfocus`, `/gmsetfocusmax` | Stats | Explicit same-space `TargetId`; zero means caller. TOML Integer is not proof of cell INT64 encoding. Compare live current/max stats, accounting for clamping. |
| `/gmsetflag`, `/gmsetlevel` | State, progression | No safe handler contract; counted flags and derived level state cannot be guessed from primitives. |
| `/gmresetabilities`, `/gmgiveallabilities` | Abilities | Durable caller-character changes. Capture ability/provenance/tree state before Base result. Reset is not the precise inverse of an earlier grant. |
| `/gmrespec`, `/gmsetmobstance` | Abilities, NPC | No GM handler; player trainer reset and AI state are separate mechanisms. |
| `/gmsettarget` | Target | Candidate server handler accepts numeric text only, despite TOML `NameOrID`; zero clears caller target. |
| `/gmsetmobabilityset` | NPC | TOML's `abilitySetId` is optional without a default; candidate server handler requires INT32 bytes. Existing entity readback cannot verify known abilities or set ID. |
| `/gmdhd`, `/gmgotolocation` | Travel | GateTravel can persist world, position and learned gate IDs after arrival. Immediate feedback is only a request-stage observation. |
| `/gmgoto`, `/gmsummon`, `/gmgotoxyz` | Travel | Same-space moves can persist player position conditionally. `TargetName` is numeric ID only in the candidate server handlers; NPC summon has no player DB row. |
| `/gmreloadorganizations` | Organizations | Caller Team/Command client-state re-push, potentially partial. It changes no organization DB row but is a transient effect, not a read. Check recipient view or outgoing pushes independently. |

All 27 forms preserve patch parameter names, order, primitive types and required flags. The optional ability-set ID has no fabricated default. Unsupported forms have a command-specific reason, impact and before/after recipe where the candidate server handler supports one, and no claimed readback where it does not. No raw command box or dot-command substitution is provided for this slice.
