# Cooked Data PAK File Format

BigWorld cooked data files (`.pak`) are ZIP archives containing XML entries that the client
uses for UI display: item descriptions, mission text, dialog screens, ability tooltips, etc.
The server serves these to clients via the `versionInfoRequest` / `onVersionInfo` /
`elementDataRequest` / `resourceFragment` protocol.

## Container Format

All `.pak` files are standard **ZIP archives** containing:
- **`MetaData`** — 4-byte little-endian integer (version stamp for cache invalidation)
- **`_<id>`** — One entry per record, named by database ID (e.g., `_622` for mission 622)

The client sends `versionInfoRequest(categoryId)`, the server responds with
`onVersionInfo(categoryId, version, ...)` where `version` is the MetaData value.
If the client's cached version differs, it requests individual entries via
`elementDataRequest(categoryId, key)`.

## Three Known Sources

| Source | Date | Origin | Notes |
|--------|------|--------|-------|
| **QA Build** | 2009-06-30 | Shipped with QA client | Most complete; SOAP namespace XML format |
| **Server Build** | 2014-02-04 | Came with C++ server source | Same entry count but compact XML; some MetaData versions differ |
| **Discord Build** | 2026-02-24 | Community contribution | Severely incomplete; most files are stubs |

**Recommendation**: Use **QA Build** PAK files for both server and client. The QA data matches
what the client was built to display. Using Server Build PAKs causes missing items (question
marks), broken mission UI, and dialog failures because the MetaData versions trigger cache
updates that overwrite the client's working data with the server's slightly different content.

## XML Format Differences

Both QA and Server builds contain the same records (same entry counts, same ID ranges).
The difference is serialization style:

### QA Build (2009) — SOAP-namespaced, child elements
```xml
<?xml version="1.0" encoding="UTF-8"?>
<COOKED_ITEM xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/"
  xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xmlns:xsd="http://www.w3.org/2001/XMLSchema"
  xmlns:CookedData1="SGW"
  IsReverseEngineerable="false" IsResearchable="false"
  IsElementaryComponent="true" IsKicker="false" TechComp="0"
  IconLocation="set:CoreWidgets image:IconMissing" Tier="1"
  AppliedScienceID="0" QualityID="2000"
  Description="The severed head of a gopher."
  Name="Gopher Head" ID="10">
  <InventorySet IsDeletable="true" .../>
</COOKED_ITEM>
```

### Server Build (2014) — Compact, no namespaces, alphabetical attributes
```xml
<?xml version="1.0" encoding="UTF-8"?><COOKED_ITEM
  AppliedScienceID="0" Description="The severed head of a gopher."
  ID="10" IconLocation="set:CoreWidgets image:IconMissing"
  IsElementaryComponent="true" IsKicker="false"
  IsResearchable="false" IsReverseEngineerable="false"
  Name="Gopher Head" QualityID="2000" TechComp="0" Tier="1"
  ItemFlags="34816">
  <InventorySet IsDeletable="true" IsSellable="false" MaxStackSize="8" />
</COOKED_ITEM>
```

Key differences:
- QA has SOAP namespace declarations; Server does not
- QA uses non-alphabetical attribute ordering; Server uses alphabetical
- QA has a leading newline after the XML declaration; Server does not
- Server sometimes has extra fields not in QA (e.g., `ItemFlags` on items)
- Both are valid XML and parse identically by any standard parser

### CookedCharCreation — Special Case

The Server Build restructured this file from 1 monolithic entry to 23 individual entries
(one per character definition). The QA Build has all char creation data in a single `_1` entry.
The Server Build entries use a different XML schema (`<Defs>` root instead of `<COOKED_CHAR_CREATION>`).

## Complete Inventory

### Category 0: CookedBehaviorEvents

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Exists | No | No | Yes (stub) |
| Size | — | — | 120 B |
| Entries | — | — | 0 |
| MetaData | — | — | 1 |

Stub file (empty ZIP with only MetaData). No source has real behavior event data.

### Category 1: CookedBlueprints

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 209 KB | 156 KB | 156 KB |
| Entries | 498 | 498 | 498 |
| MetaData | 2315 | 2315 | 2315 |
| ID Range | 1–565 | 1–565 | 1–565 |

Identical content across all three (Server = Discord). QA is larger due to SOAP namespaces.

### Category 2: CookedCharCreation

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 10.6 KB | 22.3 KB | 13.0 KB |
| Entries | 1 | 23 | 1 |
| MetaData | 7648 | 7648 | 7648 |

Server Build restructured to 23 entries (one per chardef). QA and Discord have 1 monolithic entry.

### Category 3: CookedDataAbilities

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 1.05 MB | 1.05 MB | 51 KB |
| Entries | 1886 | 1886 | 85 |
| MetaData | 8031 | 8058 | 8064 |
| ID Range | 34–3498 | 34–3498 | 523–2910 |

QA and Server have same entries. Discord has only 85 abilities (4.5% coverage).

### Category 4: CookedDataContainers

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 6.0 KB | 3.7 KB | 3.7 KB |
| Entries | 20 | 20 | 20 |
| MetaData | 3600 | 3600 | 3600 |
| ID Range | 1–20 | 1–20 | 1–20 |

Identical across all three. Small file — inventory container definitions.

### Category 5: CookedDataDialogs

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | **2.7 MB** | 2.1 MB | 5.6 KB |
| Entries | **5405** | **5405** | 9 |
| MetaData | 7660 | 7670 | 7696 |
| ID Range | 11–6427 | 11–6427 | 5354–5894 |

QA and Server have same entries. Discord has only 9 dialogs. **MetaData differs** (7660 vs 7670) — server version is newer, which triggers cache update on client.

### Category 6: CookedDataEffects

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | **1.5 MB** | 1.2 MB | 120 B (stub) |
| Entries | **3216** | **3216** | 0 |
| MetaData | 6819 | 6819 | 6824 |
| ID Range | 77–5309 | 77–5309 | — |

QA and Server have same entries. Discord is a stub.

### Category 7: CookedDataItems

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | **3.5 MB** | 2.9 MB | 174 KB |
| Entries | **6059** | **6059** | 368 |
| MetaData | 7538 | 7542 | 7562 |
| ID Range | 10–8951 | 10–8951 | 19–8403 |

Same entries in QA and Server. Discord has 6% coverage. **MetaData differs** (7538 vs 7542).

### Category 8: CookedDataKismetSeqEvent

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 679 KB | 546 KB | 4.0 KB |
| Entries | 1772 | **1973** | 16 |
| MetaData | 7455 | 7478 | 7478 |
| ID Range | 3–3027 | 3–**10186** | 3–10158 |

**Server has MORE entries** (1973 vs 1772) and a wider ID range (up to 10186). This is the only
category where the Server Build has more data than QA. The extra entries (IDs 3028–10186) were
likely added for the 2014 server development.

### Category 9: CookedDataKismetSetEvent

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 288 KB | 223 KB | 846 B |
| Entries | 660 | **675** | 2 |
| MetaData | 7454 | 7470 | 7470 |
| ID Range | 3–1542 | 3–**10013** | 570–1025 |

Server has more entries and wider ID range (similar to KismetSeqEvent above).

### Category 10: CookedDataMissions

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | **749 KB** | 637 KB | 1.9 KB |
| Entries | **1040** | **1040** | 3 |
| MetaData | 7538 | 7543 | 7576 |
| ID Range | 38–1826 | 38–1826 | 622–1559 |

Same entries. Discord has only 3 missions. **MetaData differs** (7538 vs 7543).

### Category 11: CookedDataStargates

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 11.7 KB | 8.6 KB | 8.6 KB |
| Entries | 28 | 28 | 28 |
| MetaData | 4568 | 4583 | 4583 |
| ID Range | 1–28 | 1–28 | 1–28 |

Same entries. Server and Discord identical. MetaData differs slightly.

### Category 12: CookedDisciplines

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 33.8 KB | 24.8 KB | 24.8 KB |
| Entries | 78 | 78 | 78 |
| MetaData | 2311 | 2313 | 2313 |
| ID Range | 1–96 | 1–96 | 1–96 |

Same entries. Server and Discord identical.

### Category 13: CookedInteractionSet

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | **1.6 MB** | 1.1 MB | 120 B (stub) |
| Entries | **4661** | **4663** | 0 |
| MetaData | 6615 | 6617 | 6623 |
| ID Range | 25–7621 | 20–1000000 | — |

Server has 2 more entries and a much wider ID range (up to 1,000,000). Discord is a stub.

### Category 14: CookedInteractions

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 15.8 KB | 11.7 KB | 11.7 KB |
| Entries | 40 | 40 | 40 |
| MetaData | 1404 | 1404 | 1404 |
| ID Range | 1–63 | 1–63 | 1–63 |

Identical across all three.

### Category 15: CookedParadigm

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 1.6 KB | 981 B | 981 B |
| Entries | 5 | 5 | 5 |
| MetaData | 2167 | 2167 | 2167 |
| ID Range | 1–5 | 1–5 | 1–5 |

Identical across all three.

### Category 16: CookedSciences

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 1.3 KB | 866 B | 866 B |
| Entries | 4 | 4 | 4 |
| MetaData | 2202 | 2202 | 2202 |
| ID Range | 1–4 | 1–4 | 1–4 |

Identical across all three.

### Category 17: CookedWorldInfo

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 29.9 KB | 20.0 KB | 19.8 KB |
| Entries | 91 | 91 | 90 |
| MetaData | 5959 | 5962 | 5964 |
| ID Range | 1–92 | 1–92 | 1–92 |

Discord is missing 1 world entry. MetaData differs slightly across all three.

### Category 18: ErrorStrings

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 75.4 KB | 51.0 KB | 812 B |
| Entries | 216 | 216 | 3 |
| MetaData | 5833 | 5833 | 5836 |
| ID Range | 0–10009 | 0–10009 | 0–20001 |

QA and Server identical. Discord has only 3 error strings (includes custom ID 20001).

### Category 19: SpecialWords

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 1.6 KB | 1.5 KB | 1.5 KB |
| Entries | 1 | 1 | 1 |
| MetaData | 1967 | 1967 | 1967 |

Profanity filter word list. QA larger due to SOAP namespaces.

### Category 20: TextStrings

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | **9.7 MB** | 6.9 MB | 4.5 KB |
| Entries | **29126** | **29126** | 19 |
| MetaData | 5802 | 5802 | 5802 |
| ID Range | 0–4294947296 | -20999–29747 | 6968–26986 |

Largest file. Same entries in QA and Server. Discord has 19 strings.
QA has unsigned ID parsing (large max), Server has signed (negative IDs).

## MetaData Version Comparison

Categories where MetaData differs between QA and Server — these trigger
cache updates when the server serves data to the QA client:

| Category | QA | Server | Delta |
|---|---|---|---|
| CookedDataAbilities | 8031 | 8058 | +27 |
| CookedDataDialogs | 7660 | 7670 | +10 |
| CookedDataItems | 7538 | 7542 | +4 |
| CookedDataKismetSeqEvent | 7455 | 7478 | +23 |
| CookedDataKismetSetEvent | 7454 | 7470 | +16 |
| CookedDataMissions | 7538 | 7543 | +5 |
| CookedDataStargates | 4568 | 4583 | +15 |
| CookedDisciplines | 2311 | 2313 | +2 |
| CookedInteractionSet | 6615 | 6617 | +2 |
| CookedWorldInfo | 5959 | 5962 | +3 |

When the server reports a higher version than the client's cache, the client
requests fresh data, overwriting its local copy. If the server's data is
incompatible (different XML schema, missing fields), the client breaks.

Categories with **matching** MetaData (safe to use either source):
CookedBlueprints, CookedCharCreation, CookedDataContainers, CookedDataEffects,
CookedInteractions, CookedParadigm, CookedSciences, ErrorStrings,
SpecialWords, TextStrings.

## Recommendation

Use **QA Build PAK files** exclusively for both server `data/cache/` and client
`SourceCache.en-us/`. The QA data:
1. Was built alongside the client binary — guaranteed schema compatibility
2. Has the most complete data in all categories
3. Avoids MetaData version mismatches that trigger unwanted cache updates

The only exception is **KismetSeqEvent** and **KismetSetEvent** where the Server Build
has additional entries (IDs above 3027/1542). These may be needed for custom server
content. If so, merge the extra entries into the QA PAK files rather than replacing them.
