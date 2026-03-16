# Cooked Data PAK File Format

BigWorld cooked data files (`.pak`) are ZIP archives containing XML entries that the client
uses for UI display: item descriptions, mission text, dialog screens, ability tooltips, etc.
The server serves these to clients via the `versionInfoRequest` / `onVersionInfo` /
`elementDataRequest` / `resourceFragment` protocol.

## Container Format

All `.pak` files are standard **ZIP archives** containing:
- **`MetaData`** -- 4-byte little-endian integer (version stamp for cache invalidation)
- **`_<id>`** -- One entry per record, named by database ID (e.g., `_622` for mission 622)

The client sends `versionInfoRequest(categoryId)`, the server responds with
`onVersionInfo(categoryId, version, ...)` where `version` is the MetaData value.
If the client's cached version differs, it requests individual entries via
`elementDataRequest(categoryId, key)`.

## Three Known Sources

| Source | Date | Origin | Notes |
|--------|------|--------|-------|
| **QA Build** | 2009-06-30 | Shipped with QA client | Most complete; SOAP namespace XML format |
| **Server Build** | 2014-02-04 | Came with C++ server source | Same entry count but compact XML; some MetaData versions differ |
| **Discord Build** | 2026-02-24 | Community contribution | Severely incomplete; most files are stubs or tiny subsets |

**Recommendation**: Use **QA Build** PAK files for both server and client. The QA data matches
what the client was built to display. Using Server Build PAKs causes missing items (question
marks), broken mission UI, and dialog failures because the MetaData versions trigger cache
updates that overwrite the client's working data with the server's slightly different content.

---

## Summary Table

| PAK Name | QA Size | QA # | Server Size | Srv # | Discord Size | Disc # |
|---|--:|--:|--:|--:|--:|--:|
| CookedBehaviorEvents | --- | --- | --- | --- | 120 STUB | 0 |
| CookedBlueprints | 209,713 | 498 | 156,469 | 498 | 156,469 | 498 |
| CookedCharCreation | 10,615 | 1 | 22,343 | 23 | 13,035 | 1 |
| CookedDataAbilities | 1,052,598 | 1886 | 1,052,306 | 1886 | 51,481 | 85 |
| CookedDataContainers | 6,036 | 20 | 3,729 | 20 | 3,729 | 20 |
| CookedDataDialogs | 2,702,739 | 5405 | 2,145,519 | 5405 | 5,590 | 9 |
| CookedDataEffects | 1,509,266 | 3216 | 1,166,463 | 3216 | 120 STUB | 0 |
| CookedDataItems | 3,548,002 | 6059 | 2,905,067 | 6059 | 173,955 | 368 |
| CookedDataKismetSeqEvent | 679,258 | 1772 | 545,642 | 1973 | 3,984 | 16 |
| CookedDataKismetSetEvent | 287,554 | 660 | 223,438 | 675 | 846 | 2 |
| CookedDataMissions | 749,028 | 1040 | 637,196 | 1040 | 1,908 | 3 |
| CookedDataStargates | 11,670 | 28 | 8,618 | 28 | 8,618 | 28 |
| CookedDisciplines | 33,812 | 78 | 24,811 | 78 | 24,811 | 78 |
| CookedInteractionSet | 1,627,165 | 4661 | 1,116,171 | 4663 | 120 STUB | 0 |
| CookedInteractions | 15,811 | 40 | 11,707 | 40 | 11,707 | 40 |
| CookedParadigm | 1,566 | 5 | 981 | 5 | 981 | 5 |
| CookedSciences | 1,314 | 4 | 866 | 4 | 866 | 4 |
| CookedWorldInfo | 29,856 | 91 | 19,986 | 91 | 19,782 | 90 |
| ErrorStrings | 75,407 | 216 | 50,964 | 216 | 812 | 3 |
| SpecialWords | 1,555 | 1 | 1,466 | 1 | 1,466 | 1 |
| TextStrings | 10,095,453 | 29126 | 6,906,914 | 29126 | 4,516 | 19 |
| **Totals** | **~22.6 MB** | **~57,166** | **~16.9 MB** | **~57,168** | **~484 KB** | **~1,296** |

### Stub Files (120 bytes, MetaData only)

Discord build only:
- `CookedBehaviorEvents.pak` (MetaData = 1) -- not present in QA or Server at all
- `CookedDataEffects.pak` (MetaData = 6824) -- QA/Server have 3216 entries
- `CookedInteractionSet.pak` (MetaData = 6623) -- QA/Server have 4661/4663 entries

Discord also contains `covernodes_local.pak` (22 bytes) -- an empty ZIP with no entries and no MetaData.

---

## XML Format Differences

### Three Format Generations

The three PAK sources represent different stages of the data cooking pipeline. Every single
entry differs at the byte level between QA and Server -- zero entries are byte-identical --
but the differences are systematic and predictable.

### QA Build (2009) -- SOAP-Namespaced, Newline-Separated

```xml
<?xml version="1.0" encoding="UTF-8"?>
<COOKED_ITEM xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/"
  xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xmlns:xsd="http://www.w3.org/2001/XMLSchema"
  xmlns:CookedData1="SGW"
  IsReverseEngineerable="false" IsResearchable="false"
  IsElementaryComponent="true" IsKicker="false" TechComp="1"
  IconLocation="set:ItemIcon001 image:Spray_Injector" Tier="1"
  AppliedScienceID="0" QualityID="2000"
  Description="Ambernol counteracts stasis sickness."
  Name="Ambernol Vial" ID="19">
  <InventorySet IsDeletable="true" IsSellable="true" MaxStackSize="100"></InventorySet>
  <RequirementsSet IsUnique="false"></RequirementsSet>
  <ItemEventSet AbilityID="1374" EventID="5"></ItemEventSet>
  <ContainerSet>2</ContainerSet>
</COOKED_ITEM>
```

### Server Build (2014) -- Compact, Alphabetical Attributes

```xml
<?xml version="1.0" encoding="UTF-8"?><COOKED_ITEM AppliedScienceID="0"
  Description="Ambernol counteracts stasis sickness." ID="19"
  IconLocation="set:ItemIcon001 image:Spray_Injector"
  IsElementaryComponent="true" IsKicker="false" IsResearchable="false"
  IsReverseEngineerable="false" Name="Ambernol Vial" QualityID="2000"
  TechComp="1" Tier="1" ItemFlags="35840">
  <InventorySet IsDeletable="true" IsSellable="true" MaxStackSize="1" />
  <RequirementsSet IsUnique="false" />
  <ItemEventSet AbilityID="1374" EventID="5" />
  <ContainerSet>2</ContainerSet>
</COOKED_ITEM>
```

### Systematic Format Differences (QA vs Server)

| Feature | QA Build | Server Build |
|---|---|---|
| XML declaration | Followed by newline | Immediately followed by root element |
| SOAP namespaces | 5 `xmlns:*` on root element | Absent (except CookedDataAbilities -- see below) |
| Attribute order | Arbitrary (DB column order) | Alphabetical |
| Empty child elements | Explicit close tags: `<Moniker MonikerID="123"></Moniker>` | Self-closing: `<Moniker MonikerID="123" />` |
| Newline encoding | `&#xA;` | `&#13;&#10;` |
| Float formatting | Original precision: `"1.64600003"`, `"0"`, `"2"` | Normalized: `"1.646"`, `"0.0"`, `"2.0"` |

### Data Value Differences (QA vs Server)

Beyond format, some actual data values changed between 2009 and 2014:

| Category | Difference | Example |
|---|---|---|
| **CookedDataItems** | `ItemFlags` attribute added (not in QA) | `_19`: no ItemFlags -> `ItemFlags="35840"` |
| **CookedDataItems** | `MaxStackSize` values changed | `_19`: `MaxStackSize="100"` -> `"1"` |
| **CookedDisciplines** | `Name` field simplified | `_1`: `"TestDisciplineMaterials.Common1.Basketweaving.01"` -> `"Basketweaving"` |
| **CookedDataStargates** | Float precision normalized | `xPos="87.0749969"` -> `"87.074997"`, `yaw="-1.57099998"` -> `"-1.571"` |
| **SpecialWords** | Word values double-quoted in Server | `SpecialWord="ASS"` -> `SpecialWord=""ASS""` |
| **CookedDataEffects** | Stray quote in Server data | `name="Burst Abillity""` (double quote at end) |

### CookedDataAbilities -- SOAP Namespace Anomaly

Unique among Server Build categories, CookedDataAbilities **retains** the SOAP namespace
declarations but relocates them to the end of the attribute list (alphabetized):

```xml
<!-- Server Build abilities entry -->
<?xml version="1.0" encoding="UTF-8"?><COOKED_ABILITY AbilityId="523"
  AbilityName="Concussive Grenade" ... MaxRange="2500"
  xmlns:CookedData1="SGW"
  xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/"
  xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/"
  xmlns:xsd="http://www.w3.org/2001/XMLSchema"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
```

All other Server Build categories dropped SOAP namespaces entirely. This suggests the
abilities cooker was updated at a different time or by a different process than the others.

As a result, CookedDataAbilities has mixed size behavior: QA is bigger for 957 entries
(where SOAP+newline overhead exceeds float expansion), but Server is bigger for 929 entries
(where `"0"` -> `"0.0"` float padding plus retained SOAP namespaces outweigh format savings).

### CookedCharCreation -- Structural Divergence

| Feature | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Entries | 1 (`_1`) | 23 (`_1` through `_23`) | 1 (`_1`) |
| `_1` size | 168,876 bytes | 7,224 bytes | 155,714 bytes |
| `_1` contains | All 23 `<Defs>` elements | Only CharDefId=1 | All 23 `<Defs>` elements |
| XML wrapper | `<COOKED_CHAR_CREATION>` root | No wrapper (bare `<Defs>`) | `<COOKED_CHAR_CREATION>` root |
| XML declaration | Present | **Missing** (Server `_1` starts with `<Defs`) | Present |

The Server Build split the monolithic entry into per-character-definition entries (CharDefId 1-23),
each containing a single `<Defs>` element without a wrapper. This is the only category where
the Server Build has MORE entries than QA.

---

## Entry-Level Identity Analysis

### File-Level Hash Comparison

No PAK file is byte-identical between any two sources. Every `.pak` differs due to ZIP
metadata (timestamps, compression settings) even when the contained entries are identical.

### Server vs Discord -- Entry-Level

Where both sources have the same entry, the XML content is **byte-identical** in all but
two special cases:

| Category | Common Entries | Identical | Differ | Only in Server | Only in Discord |
|---|--:|--:|--:|--:|--:|
| CookedBlueprints | 498 | **498** | 0 | 0 | 0 |
| CookedCharCreation | 1 | 0 | **1** | 22 | 0 |
| CookedDataAbilities | 85 | **85** | 0 | 1801 | 0 |
| CookedDataContainers | 20 | **20** | 0 | 0 | 0 |
| CookedDataDialogs | 9 | **9** | 0 | 5396 | 0 |
| CookedDataItems | 368 | **368** | 0 | 5691 | 0 |
| CookedDataKismetSeqEvent | 16 | **16** | 0 | 1957 | 0 |
| CookedDataKismetSetEvent | 2 | **2** | 0 | 673 | 0 |
| CookedDataMissions | 3 | **3** | 0 | 1037 | 0 |
| CookedDataStargates | 28 | **28** | 0 | 0 | 0 |
| CookedDisciplines | 78 | **78** | 0 | 0 | 0 |
| CookedInteractions | 40 | **40** | 0 | 0 | 0 |
| CookedParadigm | 5 | **5** | 0 | 0 | 0 |
| CookedSciences | 4 | **4** | 0 | 0 | 0 |
| CookedWorldInfo | 90 | 89 | **1** | 1 | 0 |
| ErrorStrings | 2 | **2** | 0 | 214 | **1** |
| SpecialWords | 1 | **1** | 0 | 0 | 0 |
| TextStrings | 19 | **19** | 0 | 29107 | 0 |

**Key finding**: The Discord build is a strict subset of the Server Build. Every Discord entry
that also exists in Server is byte-identical to the Server version. The Discord build was
produced by the same cooking tool as the Server Build, just with far less data fed in.

**Two exceptions**:
1. **CookedCharCreation `_1`**: Discord has the monolithic format (155KB with `<COOKED_CHAR_CREATION>` wrapper),
   Server has the split format (7KB bare `<Defs>`). Same underlying data, different structure.
2. **CookedWorldInfo `_2`**: `ClientMap` changed -- Server has `"sandbox"`, Discord has `"Harset_CmdCenter"`.

**Discord-only data**:
- `ErrorStrings _20001`: `"Invalid character name"` (custom error not in QA or Server)

### QA vs Server -- Entry-Level

Every single entry differs at the byte level (0 identical across all categories). This is
because the format differences (SOAP namespaces, attribute ordering, self-closing tags,
newline after XML declaration) affect every entry systematically.

For most categories, QA entries are larger (SOAP overhead). The only mixed case is
CookedDataAbilities where retained SOAP namespaces in Server combined with float expansion
creates a near-even split (957 QA-bigger vs 929 Server-bigger).

### Server vs Discord -- Missing and Extra Data

| Category | Discord has only | Entries missing vs Server |
|---|---|---|
| CookedDataEffects | 0 (stub) | 3,216 |
| CookedInteractionSet | 0 (stub) | 4,663 |
| CookedDataDialogs | 9 | 5,396 |
| CookedDataItems | 368 | 5,691 |
| TextStrings | 19 | 29,107 |
| CookedDataAbilities | 85 | 1,801 |
| CookedDataKismetSeqEvent | 16 | 1,957 |
| CookedDataMissions | 3 | 1,037 |

### QA vs Server -- Where Server Has MORE Data

Two categories where the Server Build has entries not present in QA:

| Category | QA Entries | Server Entries | QA ID Range | Server ID Range |
|---|---|---|---|---|
| CookedDataKismetSeqEvent | 1772 | **1973** (+201) | 3-3027 | 3-**10186** |
| CookedDataKismetSetEvent | 660 | **675** (+15) | 3-1542 | 3-**10013** |
| CookedInteractionSet | 4661 | **4663** (+2) | 25-7621 | 20-**1000000** |

The Server Build extended the Kismet event IDs into the 10000+ range and added interaction
set entries including one at ID 1,000,000 (`"Plant Listening Device"`, DialogSetID=689).

### CookedWorldInfo Discrepancy

- Server has 91 entries (IDs 1-92). Discord has 90 entries (IDs 1-92 minus `_89`).
- **Missing from Discord**: `_89` = World "Temple" (`ClientMap="Temple"`, Flags=1)
- **Content difference in `_2`**: Server maps WorldID 2 to `ClientMap="sandbox"`;
  Discord maps WorldID 2 to `ClientMap="Harset_CmdCenter"`.

---

## MetaData Version Stamp Comparison

The MetaData value is a monotonically increasing version counter. Higher = newer data.

| Category | QA | Server | Discord | QA->Srv | Srv->Disc |
|---|--:|--:|--:|---|---|
| CookedBehaviorEvents | --- | --- | 1 | | |
| CookedBlueprints | 2315 | 2315 | 2315 | = | = |
| CookedCharCreation | 7648 | 7648 | 7648 | = | = |
| **CookedDataAbilities** | 8031 | 8058 | 8064 | **+27** | **+6** |
| CookedDataContainers | 3600 | 3600 | 3600 | = | = |
| **CookedDataDialogs** | 7660 | 7670 | 7696 | **+10** | **+26** |
| CookedDataEffects | 6819 | 6819 | 6824 | = | **+5** |
| **CookedDataItems** | 7538 | 7542 | 7562 | **+4** | **+20** |
| **CookedDataKismetSeqEvent** | 7455 | 7478 | 7478 | **+23** | = |
| **CookedDataKismetSetEvent** | 7454 | 7470 | 7470 | **+16** | = |
| **CookedDataMissions** | 7538 | 7543 | 7576 | **+5** | **+33** |
| CookedDataStargates | 4568 | 4583 | 4583 | **+15** | = |
| CookedDisciplines | 2311 | 2313 | 2313 | **+2** | = |
| CookedInteractionSet | 6615 | 6617 | 6623 | **+2** | **+6** |
| CookedInteractions | 1404 | 1404 | 1404 | = | = |
| CookedParadigm | 2167 | 2167 | 2167 | = | = |
| CookedSciences | 2202 | 2202 | 2202 | = | = |
| CookedWorldInfo | 5959 | 5962 | 5964 | **+3** | **+2** |
| ErrorStrings | 5833 | 5833 | 5836 | = | **+3** |
| SpecialWords | 1967 | 1967 | 1967 | = | = |
| TextStrings | 5802 | 5802 | 5802 | = | = |

### Impact of Version Mismatches

When the server reports a MetaData version higher than the client's cached version, the client
requests fresh data and overwrites its local cache. If the server's data has:
- Different XML schema (missing fields, renamed attributes) -- client UI breaks
- Fewer entries than the client expects -- missing content shows as `???` or crashes
- Different float precision -- minor visual/functional differences

**10 categories have QA != Server versions** -- these will trigger cache updates when a QA client
connects to a server serving Server Build versions.

**Categories safe to serve from either source** (matching MetaData):
CookedBlueprints, CookedCharCreation, CookedDataContainers, CookedDataEffects,
CookedInteractions, CookedParadigm, CookedSciences, ErrorStrings, SpecialWords, TextStrings.

---

## TextStrings ID Anomaly

The QA Build contains 1,000 entries with unsigned 32-bit overflow IDs (e.g., `_4294946297`).
These represent negative signed IDs: `4294946297 = 0xFFFFADF9 = -20999` as int32.

The Server Build stores these as signed strings (`_-20999` through `_-1`), with the same
1,000 entries mapping to the same text content. The remaining 28,126 entries use positive IDs
in range 0-29747 and are identical across both sources.

---

## Side-by-Side XML Comparisons

### CookedDataItems `_19` (Ambernol Vial)

**QA Build** (824 bytes):
```xml
<?xml version="1.0" encoding="UTF-8"?>
<COOKED_ITEM xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/"
  xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xmlns:xsd="http://www.w3.org/2001/XMLSchema"
  xmlns:CookedData1="SGW"
  IsReverseEngineerable="false" IsResearchable="false"
  IsElementaryComponent="true" IsKicker="false" TechComp="1"
  IconLocation="set:ItemIcon001 image:Spray_Injector" Tier="1"
  AppliedScienceID="0" QualityID="2000"
  Description="Ambernol counteracts stasis sickness."
  Name="Ambernol Vial" ID="19">
  <InventorySet IsDeletable="true" IsSellable="true" MaxStackSize="100"></InventorySet>
  <RequirementsSet IsUnique="false"></RequirementsSet>
  <ItemEventSet AbilityID="1374" EventID="5"></ItemEventSet>
  <ContainerSet>2</ContainerSet>
</COOKED_ITEM>
```

**Server/Discord Build** (556 bytes, byte-identical):
```xml
<?xml version="1.0" encoding="UTF-8"?><COOKED_ITEM AppliedScienceID="0"
  Description="Ambernol counteracts stasis sickness." ID="19"
  IconLocation="set:ItemIcon001 image:Spray_Injector"
  IsElementaryComponent="true" IsKicker="false" IsResearchable="false"
  IsReverseEngineerable="false" Name="Ambernol Vial" QualityID="2000"
  TechComp="1" Tier="1" ItemFlags="35840">
  <InventorySet IsDeletable="true" IsSellable="true" MaxStackSize="1" />
  <RequirementsSet IsUnique="false" />
  <ItemEventSet AbilityID="1374" EventID="5" />
  <ContainerSet>2</ContainerSet>
</COOKED_ITEM>
```

Changes: SOAP namespaces removed, attributes alphabetized, self-closing tags, `ItemFlags="35840"` added, `MaxStackSize` changed from 100 to 1.

---

### CookedDataDialogs `_5354` (SGC Intro Dialog)

**QA Build** (2076 bytes):
```xml
<?xml version="1.0" encoding="UTF-8"?>
<COOKED_DIALOG xmlns:SOAP-ENV="..." xmlns:CookedData1="SGW"
  DialogFlags="0" KismetEventSetID="0" UIScreenType="2" DialogID="5354">
  <Screens SpeakerID="2609"
    Text="At ease, and welcome to Stargate Command."
    ScreenID="107261"></Screens>
  <Screens SpeakerID="0"
    Text="Thank you, sir."
    ScreenID="107262"></Screens>
  <!-- ... 9 more screens ... -->
</COOKED_DIALOG>
```

**Server/Discord Build** (1835 bytes, byte-identical):
```xml
<?xml version="1.0" encoding="UTF-8"?><COOKED_DIALOG DialogFlags="0"
  DialogID="5354" KismetEventSetID="0" UIScreenType="2">
  <Screens SpeakerID="2609" ScreenID="107261"
    Text="At ease, and welcome to Stargate Command."></Screens>
  <Screens SpeakerID="0" ScreenID="107262"
    Text="Thank you, sir."></Screens>
  <!-- ... 9 more screens ... -->
</COOKED_DIALOG>
```

Changes: SOAP namespaces removed, root attributes alphabetized, child attribute order changed
(ScreenID before Text in Server). Dialog `<Screens>` elements keep explicit close tags in both.

---

### CookedDataMissions `_622` (Arm Yourself!)

**QA Build** (1643 bytes):
```xml
<?xml version="1.0" encoding="UTF-8"?>
<COOKED_MISSION xmlns:SOAP-ENV="..." xmlns:CookedData1="SGW"
  AwardXP="true" ShowFactionChangeIcon="false" ShowInstanceIcon="false"
  ShowPVPIcon="false" IsShareable="true" NumRepeats="1" CanAbandon="false"
  CanRepeatOnFail="true" CanFail="false" Level="1" IsHidden="false"
  IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1"
  MissionLabel="General" MissionDefn="Arm Yourself!" MissionID="622">
  <HistoryText>Arm Yourself!</HistoryText>
  <Steps StepEnabled="false" StepID="2113" AwardXP="false" Difficulty="1">
    <StepDisplayLogText>Search the nearby corpses to locate a weapon.</StepDisplayLogText>
    <Objectives IsOptional="true" ObjectiveID="3238" ...>
      <Tasks TaskType="1" TaskID="5686" ...></Tasks>
      <Tasks TaskType="1" TaskID="5687" ...></Tasks>
      <Tasks TaskType="1" TaskID="5691" ...></Tasks>
    </Objectives>
    <Objectives IsOptional="false" ObjectiveID="2452" ...>
      <Tasks TaskType="1" TaskID="2935" ...></Tasks>
      <Tasks TaskType="1" TaskID="3928" ...></Tasks>
    </Objectives>
  </Steps>
</COOKED_MISSION>
```

**Server/Discord Build** (1371 bytes, byte-identical):
```xml
<?xml version="1.0" encoding="UTF-8"?><COOKED_MISSION AwardXP="true"
  CanAbandon="false" CanFail="false" CanRepeatOnFail="true" Difficulty="1"
  IsAStory="true" IsEnabled="true" IsHidden="false"
  IsOverrideMission="false" IsShareable="true" Level="1"
  MissionDefn="Arm Yourself!" MissionID="622" MissionLabel="General"
  NumRepeats="1" ShowFactionChangeIcon="false"
  ShowInstanceIcon="false" ShowPVPIcon="false">
  <HistoryText>Arm Yourself!</HistoryText>
  <Steps AwardXP="false" Difficulty="1" StepEnabled="false" StepID="2113">
    <Objectives AwardXP="false" Difficulty="1" IsEnabled="false"
      IsHidden="false" IsOptional="false" ObjectiveID="2452">
      <Tasks AwardXP="false" Difficulty="1" IsEnabled="true" TaskID="3928" TaskType="1" />
      <Tasks AwardXP="false" Difficulty="1" IsEnabled="true" TaskID="2935" TaskType="1" />
    </Objectives>
    <Objectives AwardXP="false" Difficulty="1" IsEnabled="false"
      IsHidden="true" IsOptional="true" ObjectiveID="3238">
      <Tasks AwardXP="false" Difficulty="1" IsEnabled="true" TaskID="5691" TaskType="1" />
      <Tasks AwardXP="false" Difficulty="1" IsEnabled="true" TaskID="5686" TaskType="1" />
      <Tasks AwardXP="false" Difficulty="1" IsEnabled="true" TaskID="5687" TaskType="1" />
    </Objectives>
    <StepDisplayLogText>Search the nearby corpses to locate a weapon.</StepDisplayLogText>
  </Steps>
</COOKED_MISSION>
```

Changes: SOAP namespaces removed, all attributes alphabetized (including children),
self-closing `<Tasks />`, child element ORDER changed (Objectives reordered: non-optional
first in Server), `<StepDisplayLogText>` moved to end of `<Steps>` in Server.

---

## Complete Per-Category Inventory

### Category 0: CookedBehaviorEvents

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Exists | No | No | Yes (stub) |
| Size | -- | -- | 120 B |
| Entries | -- | -- | 0 |
| MetaData | -- | -- | 1 |

Stub file (empty ZIP with only MetaData). No source has real behavior event data.

### Category 1: CookedBlueprints

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 209,713 | 156,469 | 156,469 |
| Entries | 498 | 498 | 498 |
| MetaData | 2315 | 2315 | 2315 |
| ID Range | 1-565 | 1-565 | 1-565 |

All three have identical entry content (Server = Discord byte-for-byte). QA is larger
due to SOAP namespaces. MetaData matches -- safe to use any source.

### Category 2: CookedCharCreation

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 10,615 | 22,343 | 13,035 |
| Entries | 1 | 23 | 1 |
| MetaData | 7648 | 7648 | 7648 |
| Structure | Monolithic | Per-chardef | Monolithic |

Server Build split the data into per-chardef entries. QA and Discord both use a single
`_1` entry wrapping all 23 `<Defs>` elements. MetaData matches.

- QA `_1`: 168,876 bytes, has `<COOKED_CHAR_CREATION>` wrapper + SOAP namespaces
- Server `_1`: 7,224 bytes, bare `<Defs>` (no XML declaration, no wrapper)
- Discord `_1`: 155,714 bytes, has `<COOKED_CHAR_CREATION>` wrapper, no SOAP namespaces

### Category 3: CookedDataAbilities

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 1,052,598 | 1,052,306 | 51,481 |
| Entries | 1886 | 1886 | 85 |
| MetaData | 8031 | 8058 | 8064 |
| ID Range | 34-3498 | 34-3498 | 523-2910 |

QA and Server have same entries. Discord has only 85 abilities (4.5% coverage).
**Uniquely, Server retains SOAP namespace declarations** (alphabetized to end of attributes).
MetaData differs across all three -- all trigger cache updates.

### Category 4: CookedDataContainers

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 6,036 | 3,729 | 3,729 |
| Entries | 20 | 20 | 20 |
| MetaData | 3600 | 3600 | 3600 |
| ID Range | 1-20 | 1-20 | 1-20 |

All three have identical data. Server = Discord byte-for-byte. Small file (inventory
container definitions like MAIN, EQUIPPED, BANK). MetaData matches.

### Category 5: CookedDataDialogs

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 2,702,739 | 2,145,519 | 5,590 |
| Entries | 5405 | 5405 | 9 |
| MetaData | 7660 | 7670 | 7696 |
| ID Range | 11-6427 | 11-6427 | 5354-5894 |

QA and Server have same entries. Discord has only 9 dialogs. **MetaData differs across
all three** -- highest impact category for cache-update breakage because dialog data is
critical for NPC interactions.

### Category 6: CookedDataEffects

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 1,509,266 | 1,166,463 | 120 (stub) |
| Entries | 3216 | 3216 | 0 |
| MetaData | 6819 | 6819 | 6824 |
| ID Range | 77-5309 | 77-5309 | -- |

QA and Server have same entries with matching MetaData. Discord is a stub with a
higher MetaData (6824) that would trigger cache updates sending zero data.
Note: Server has a data bug -- entry `_77` has `name="Burst Abillity""` (stray double quote).

### Category 7: CookedDataItems

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 3,548,002 | 2,905,067 | 173,955 |
| Entries | 6059 | 6059 | 368 |
| MetaData | 7538 | 7542 | 7562 |
| ID Range | 10-8951 | 10-8951 | 19-8403 |

Same entries in QA and Server. Discord has 6% coverage. **MetaData differs**. Server added
the `ItemFlags` attribute and changed some `MaxStackSize` values vs QA.

### Category 8: CookedDataKismetSeqEvent

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 679,258 | 545,642 | 3,984 |
| Entries | 1772 | **1973** | 16 |
| MetaData | 7455 | 7478 | 7478 |
| ID Range | 3-3027 | 3-**10186** | 3-10158 |

**Server has 201 more entries** than QA, with IDs extending to 10186. These were added for
2014 server development. Discord has Server-matching MetaData.

### Category 9: CookedDataKismetSetEvent

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 287,554 | 223,438 | 846 |
| Entries | 660 | **675** | 2 |
| MetaData | 7454 | 7470 | 7470 |
| ID Range | 3-1542 | 3-**10013** | 570-1025 |

**Server has 15 more entries** than QA, with IDs extending to 10013.

### Category 10: CookedDataMissions

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 749,028 | 637,196 | 1,908 |
| Entries | 1040 | 1040 | 3 |
| MetaData | 7538 | 7543 | 7576 |
| ID Range | 38-1826 | 38-1826 | 622-1559 |

Same entries. Discord has only 3 missions. **MetaData differs across all three**.

### Category 11: CookedDataStargates

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 11,670 | 8,618 | 8,618 |
| Entries | 28 | 28 | 28 |
| MetaData | 4568 | 4583 | 4583 |
| ID Range | 1-28 | 1-28 | 1-28 |

All entries present in all three. Server = Discord byte-for-byte. MetaData differs
(QA 4568 vs Server/Discord 4583).

### Category 12: CookedDisciplines

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 33,812 | 24,811 | 24,811 |
| Entries | 78 | 78 | 78 |
| MetaData | 2311 | 2313 | 2313 |
| ID Range | 1-96 | 1-96 | 1-96 |

All entries present. Server = Discord byte-for-byte. QA has verbose discipline names
(e.g., `"TestDisciplineMaterials.Common1.Basketweaving.01"` vs Server's `"Basketweaving"`).

### Category 13: CookedInteractionSet

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 1,627,165 | 1,116,171 | 120 (stub) |
| Entries | 4661 | **4663** | 0 |
| MetaData | 6615 | 6617 | 6623 |
| ID Range | 25-7621 | 20-**1000000** | -- |

Server has 2 more entries, including `_1000000` ("Plant Listening Device", DialogSetID=689,
InteractionFlags=1073741824) and `_20` (a low ID not in QA). Discord is a stub.

### Category 14: CookedInteractions

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 15,811 | 11,707 | 11,707 |
| Entries | 40 | 40 | 40 |
| MetaData | 1404 | 1404 | 1404 |
| ID Range | 1-63 | 1-63 | 1-63 |

All entries present. Server = Discord byte-for-byte. MetaData matches -- safe.

### Category 15: CookedParadigm

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 1,566 | 981 | 981 |
| Entries | 5 | 5 | 5 |
| MetaData | 2167 | 2167 | 2167 |
| ID Range | 1-5 | 1-5 | 1-5 |

Identical across all three. 5 racial paradigm definitions (Common, Human, Jaffa, Goa'uld, Asgard).

### Category 16: CookedSciences

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 1,314 | 866 | 866 |
| Entries | 4 | 4 | 4 |
| MetaData | 2202 | 2202 | 2202 |
| ID Range | 1-4 | 1-4 | 1-4 |

Identical across all three. 4 applied science definitions (Biomedical Engineering, etc.).

### Category 17: CookedWorldInfo

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 29,856 | 19,986 | 19,782 |
| Entries | 91 | 91 | **90** |
| MetaData | 5959 | 5962 | 5964 |
| ID Range | 1-92 | 1-92 | 1-92 |

Discord is missing entry `_89` (World "Temple", ClientMap="Temple", Flags=1).
Discord's `_2` differs from Server: `ClientMap="Harset_CmdCenter"` instead of `"sandbox"`.
MetaData differs across all three.

### Category 18: ErrorStrings

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 75,407 | 50,964 | 812 |
| Entries | 216 | 216 | 3 |
| MetaData | 5833 | 5833 | 5836 |
| ID Range | 0-10009 | 0-10009 | 0-20001 |

QA and Server match (MetaData = 5833). Discord has only 3 entries but introduces a
custom one: `_20001` ("Invalid character name" / ERROR_InvalidCharacterName) not present
in either original source.

### Category 19: SpecialWords

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 1,555 | 1,466 | 1,466 |
| Entries | 1 | 1 | 1 |
| MetaData | 1967 | 1967 | 1967 |

Profanity filter word list. Server = Discord byte-for-byte. QA larger due to SOAP namespaces.
Note: QA stores words without quotes (`SpecialWord="ASS"`); Server/Discord double-quotes them
(`SpecialWord=""ASS""`). QA also includes `"BRODY"` as first entry (before the alphabetical list).

### Category 20: TextStrings

| | QA Build | Server Build | Discord Build |
|---|---|---|---|
| Size | 10,095,453 | 6,906,914 | 4,516 |
| Entries | 29126 | 29126 | 19 |
| MetaData | 5802 | 5802 | 5802 |
| ID Range | 0-4294947296* | -20999-29747 | 6968-26986 |

Largest file. Same entries in QA and Server. Discord has 19 strings. MetaData matches.

*QA stores negative IDs as unsigned 32-bit values (e.g., `_4294946297` = -20999 as int32).
Server stores them with a signed prefix (`_-20999`). Both have the same 1,000 negative-ID entries.

---

## Recommendation

Use **QA Build PAK files** exclusively for both server `data/cache/` and client
`SourceCache.en-us/`. The QA data:
1. Was built alongside the client binary -- guaranteed schema compatibility
2. Has the most complete data in all categories
3. Avoids MetaData version mismatches that trigger unwanted cache updates

The only exception is **KismetSeqEvent** and **KismetSetEvent** where the Server Build
has additional entries (IDs above 3027/1542). These may be needed for custom server
content. If so, merge the extra entries into the QA PAK files rather than replacing them.

The **Discord Build** is a strict subset of the Server Build with no unique game data
(only one custom error string). It should not be used as a primary data source.
