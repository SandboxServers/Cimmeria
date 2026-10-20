# Cooked Castle dialog maps

> **Type:** Reference. **Audience:** Castle restoration authors and reviewers.
> **Last updated:** 2026-10-14. **Inspected:** configured QA client install.
> **Companions:** [Castle ledger](../README.md), [source reconciliation](reconciliation.md).

`SGWGame/SourceCache.en-us/CookedInteractionSet.pak` is a ZIP archive of 4,664
cooked interaction-set map records. Its `MetaData` entry contains the
little-endian version stamp `D7 19 00 00` (6615). The nine category-8 records
below agree with the shipped SQL dialog-set maps. The ZIP entry suffix is the
**map ID**, not the dialog-set ID.

| Entry / map ID | Dialog set | Dialog | Interaction flags | Topic |
|---|---:|---:|---:|---|
| `_5824` | 1571 | 4982 | 0 | Infirmary |
| `_5825` | 1571 | 4984 | 0 | Infirmary |
| `_5826` | 1571 | 4985 | 0 | Infirmary |
| `_5827` | 1571 | 4983 | 16777216 | Infirmary |
| `_5828` | 1571 | 0 | 1073741824 | Infirmary |
| `_5829` | 1572 | 0 | 1073741824 | Symbiote |
| `_5830` | 1572 | 4989 | 0 | Symbiote |
| `_5831` | 1572 | 4990 | 0 | Symbiote |
| `_5832` | 1572 | 4991 | 0 | Symbiote |

The `0` dialog values in the cooked records correspond to NULL dialog IDs in
the seed. A search across all 4,664 map records found neither moniker 8896
(Med Dispenser) nor 8897 (Symbiote Tank), nor their names. The companion
`SGWGame/SourceCache.en-us/CookedInteractions.pak` has 41 generic interaction
icon and cursor definitions; none references Castle, those monikers, sets
1571/1572, or maps 5825/5831.

**Boundary:** This confirms that references to success dialog IDs 4984 and
4990 were packaged in this interaction-map archive. It does not establish
that the dialog screens or text were packaged here. It identifies no actor,
spawn, trigger, game type, or server callback for either mission. The map IDs
cannot be used as actor IDs. A Kismet/server actor reference or an aligned
live interaction trace is still needed before G-MED or G-SYM can close.

## Romney's Files is a recipient topic

The same archive has `_2817`: map 2817, dialog set 512, dialog 2044, flags
536870912 and topic `Romney's Files`. The shipped seed has only that one
set-512 map (`db/resources/Dialogs/Seed/dialog_set_maps.sql:1933`). Dialog
2044 has no event set or mission acceptance; its one screen, 60089, has
speaker 968 (Copplemann) thanking the player for the files and no button row
(`dialogs.sql:3611`, `dialog_screens.sql:9473`). Existing disabled Harset
chains 6503/6504 use this topic for the step-4039 recipient
(`db/resources/Content/Seed/harset_opcore_chains.sql:337`–`397`). This is
turn-in evidence, with no Castle cabinet actor, pickup event or item-2698
grant. G-567 remains open.

## Other local indexes checked

The configured QA install has 21 cooked category PAKs. A bounded search of
`CookedDataKismetSeqEvent.pak` (1,974 entries) and
`CookedDataKismetSetEvent.pak` (676 entries) found no matching dispenser,
tank or cabinet moniker/name and no join to maps 5824–5832. Castle string
hits describe Cellblock, dialog voice and Stargate prefab sequences. The
symbiote hit sampled in the Kismet data belongs to a different world.
`CookedDataMissions.pak/_1520` confirms mission text/steps but contains no
actor binding.

Targeted name-table reads from `Castle-00080002.umap`,
`Castle-00080003.umap` and `Castle-00090003.umap` found the
`CA-SymbioteChamber00` prefab family, without a dispenser, tank or cabinet
quest join. The existing repo readers `tools/upk_parser.py`,
`tools/extract_actors.py` and `tools/kismet_extractor.py` can support a deeper
export/property trace. These checks are an absence boundary for likely
local indexes, not proof that no actor exists in the client.
