---
title: "Combat Formulas - What the Client Artifacts Prove"
type: reference
audience: engineers
last_updated: 2026-09-18
---

# Combat Formulas - What the Client Artifacts Prove

> **Type**: reference (evidence ledger)
> **Question**: is ANY original Stargate Worlds combat formula or constant recoverable from the *client* install?
> **Answer in one line**: no formula and no cover/armor/scaling *constant* is recoverable. What the client does carry is (a) the **unit conventions** the designers used (stat points to QR), written in shipped XML comments, and (b) **designer-authored magnitudes** in the cooked ability/effect/item descriptions. Neither is executable logic.
> **Companions**: [combat-damage-analysis.md](combat-damage-analysis.md) (prior "server-side only" conclusion, re-checked here), [cover-system.md](cover-system.md), [stat-scaling-formulas.md](stat-scaling-formulas.md), [effect-execution-model.md](effect-execution-model.md), [../../engine/ue3-package-format.md](../../engine/ue3-package-format.md)

## Ground rules applied

- The legacy Python under `deprecated/python/` is a fan developer's best guess and is **not consulted and not cited as evidence** in this document. The only mention of it is the "Fan candidates" stub at the end.
- Every claim carries a reference and one **evidence tag**:
  - `ORIGINAL-RE` - decompiled or disassembled client code (SGW.exe, Ghidra address given).
  - `ORIGINAL-DATA` - shipped game data (2009-06-30 client install; cooked `.pak` entries carry a 2008-12-11 cook date per [cooked-data-pak-format.md](../../engine/cooked-data-pak-format.md)).
  - `UNKNOWN` - not recoverable from the client. Nothing is guessed to fill a gap.
- Confidence: **HIGH** = read directly from the artifact, unambiguous; **MEDIUM** = read directly but the meaning is an interpretation; **LOW** = inference.

## Verdict table

| # | Question | Formula / constant in client? | Best client evidence |
|---|---|---|---|
| 1 | Accuracy vs Defense to hit/miss | **No** | Unit convention only: 1 stat point = 0.01 QR (alias.xml) |
| 2 | Low/Med/High cover, Cover Def vs Cover Pen, Crouch Def | **No bonus values** | Cover Height/Quality enums (no numbers); stat units; designer-text magnitudes |
| 3 | Weapon scaling by TechComp / Tier / Quality / Applied Science | **No** | Item attributes exist (Tier 1-5, TechComp, Quality 1000-5000, Applied Science 0-4); no damage numbers |
| 4 | Armor / Resistance / Mitigation / Penetration | **No** | Stat semantics in comments; "30% Mitigation" item text; no combination rule |
| 5 | Focus effect on accuracy and Health damage | **No** | Designer notation `-XF / -YH` (10:1) and "bonus vs low Focus" text; nothing on accuracy |
| 6 | Buff/debuff stacking order and caps | **No** | Two "Stacks with ..." effect descriptions; no order, no cap |
| 7 | AoE vs cover / line of sight | **No** | AoE radius/cone constants; LOS error string; AoE cover-debuff ability text |

## Method: what was searched (so every negative is reproducible)

Client root: `C:\Users\Steve\source\projects\sgw\Stargate Worlds-QA` (abbreviated `<client>` below).

| # | Artifact | How it was searched | Result |
|---|---|---|---|
| S1 | `Working\SGWGame\Content\FRScript\{SGWGame,Engine,Core,Editor,GFxUI,IpDrv,UnrealEd,GFxUIEditor}.u` | `tools/upk_parser.py` for export tables; scratch script to read every `TextBuffer` export (UnrealScript `ScriptText`/`CppText` **is embedded in the cooked packages**: SGWGame 91, Engine 1,754, Core 15, Editor 40, GFxUI 5, UnrealEd 244) and every `Default__*` export's tagged properties | Class/enum/struct names and stock-UE3 defaults only; no combat math (see Q1-Q7) |
| S2 | `<client>\Common\res\entities\defs\alias.xml`, `enumerations.xml` (byte-identical to repo `entities/defs/alias.xml`, `entities/defs/enumerations.xml`; both dated 2009-06-30) | Full read of stat template, cover, damage, result, mitigation, quality and effect-flag enums | The richest source: unit conventions and enum sets |
| S3 | `<client>\Common\xml\SGWShared\CookedData\*.xsd` | Read `Ability.xsd`, `Effect.xsd`, `Item.xsd`, `SharedDefs.xsd` | Schemas carry no damage/stat-magnitude fields |
| S4 | `<client>\Working\SGWGame\SourceCache.en-us\*.pak` (ZIP of XML): `CookedDataAbilities` (1,886), `CookedDataEffects` (3,216), `CookedDataItems` (6,059), `CookedDisciplines` (78), `CookedSciences` (4), `CookedParadigm` (5), `TextStrings`, `ErrorStrings` | Parsed every entry; dumped to TSV; keyword grep per question | Designer-authored description text with magnitudes |
| S5 | `<client>\Working\SGWGame\Content\UI\**\*.lua` (93 files) and `*.int` | `grep -i accuracy\|penetrat\|defense\|armor\|mitigat\|techcomp\|qrmod\|cover\|stabiliz\|crouch\|focus` | UI shows raw stat values; no derived-stat math |
| S6 | `<client>\Working\binaries\SGW.exe` in Ghidra (project `SGW`) | See "Ghidra spot-check" | No formulas; stat names only |
| S7 | `<client>\Working\SGWGame\Config\*.ini`, `Content\XML\*.xml`, `Working\binaries\*.dll` list | grep for combat terms; module list | Nothing combat-related |

> [!NOTE]
> **Tooling detail for the Ghidra pass.** The `mcp__ghidra__*` analysis tools were not surfaced to this agent's tool list (only the bridge/debugger proxies were), so the spot-check used the same read-only `GET` endpoints the bridge fronts on `http://127.0.0.1:8100/` (`list_strings`, `get_xrefs_to`, `get_function_by_address`, `decompile_function`, `inspect_memory_content`, `read_memory`). No write, rename, retype or comment call was made.

## Evidence ledger (referenced by ID from the question tables)

| ID | Tag | Reference | What it is |
|---|---|---|---|
| E1 | ORIGINAL-DATA | `entities/defs/alias.xml:193-269` (client copy `Common\res\entities\defs\alias.xml`, same lines) | `StatList` template: one designer XML comment per stat. Defines units (see below). |
| E2 | ORIGINAL-DATA | `entities/defs/enumerations.xml` `ECoverQuality` 207-215, `ECoverHeight` 216-224, `Postures` 225-232, `EResultCode` 252-262, `EStatResultCode` 263-271, `EDamageType` 272-281, `EWeaponRange` 331-342, `EMitigationType` 404-425, `EStats` 468-554, `EEffectFlag` 1094-1123, `EItemQuality` 1693-1701, `ETargetCollectionParams` 11-32, `ETargetCollectionMethod` 307-318 | Enum name/value sets |
| E3 | ORIGINAL-DATA | `SourceCache.en-us\CookedDataAbilities.pak` entry `_<AbilityId>`; `CookedDataEffects.pak` entry `_<EffectId>` | `AbilityDesc` / `EffectDesc` text written by designers |
| E4 | ORIGINAL-DATA | `CookedDataItems.pak` entry `_<ItemId>` | Item `Tier`, `TechComp`, `QualityID`, `AppliedScienceID`, `Description`, `MeleeRanges`/`RangeRanges` |
| E5 | ORIGINAL-DATA | `Engine.u` exports `SGWCoverNodeComponent.ScriptText` [19465], `SGWSpecCoverNode.ScriptText` [19497], `CoverLink.ScriptText` [18330]; `Default__SGWCoverNodeComponent` [14324], `Default__SGWSpecCoverNode` [14493], `Default__CoverLink` [4945] | UnrealScript source and class defaults |
| E6 | ORIGINAL-RE | SGW.exe `0x00acbb10-0x00ad6b65`, `0x00aa0ba0`, `0x00aebf80`, `0x00904d80`, `0x00c81b60`, string table `0x01956d80-0x01957140` | Ghidra spot-check anchors |
| E7 | ORIGINAL-DATA | `Content\UI\Core\Character\Character.lua:8-12,458-472`, `Core\SCT\SCT.lua:188-210`, `Core\ChatWindow\ChatEvents.lua:68,146-147`, `Core\SelfStatus\SelfStatus.lua:19` | Client UI stat handling |
| E8 | ORIGINAL-DATA | `SourceCache.en-us\ErrorStrings.pak` `_39`, `_19`, `_4`; `TextStrings.pak` `_10009` | LOS / flank condition feedback text; Battlefield Warrior description |

### The unit convention (E1) - the one thing the client really proves

`alias.xml` comments define how a stat *point* converts to QR (Quality Rating). Quoted verbatim (line numbers in `alias.xml`):

| Stat | Line | Verbatim designer comment |
|---|--:|---|
| `accuracy` | 204 | modifies outgoing ranged and melee QR by +0.01 per point |
| `defense` | 205 | modifies incoming ranged and melee QR by -0.01 per point |
| `qrMod` | 206 | modifies both attacking and defending QR by 1 per point |
| `coverQRModifier` | 216 | increases both attack and defend QR while behind cover by 1 per point |
| `tracking` | 230 | modifies the QR shift frpm defensive movement by -0.01 per point *(sic)* |
| `stabilization` | 231 | modifies the QR shift from attacker movement by -0.01 per point |
| `awareness` | 232 | modifies outgoing ranged and melee QR by +0.01 per point |
| `coverAccuracy` | 234 | increases the accuracy of attacks against a target inside cover by +0.01 QR |
| `coverDefense` | 235 | increases the defense of a player in cover by -0.01 QR |
| `crouchingAccuracy` | 236 | increases the player's accuracy of attacks while crouching by +0.01 QR |
| `crouchingDefense` | 237 | increases the player's defense from attacks while crouching by -0.01 QR |
| `coordination` | 193 | +0.05 to ranged attack QR per point, +0.1% resistance to interrupts per point |
| `engagement` | 194 | +0.05 to melee attack QR per point, +1% to kinetic resists per point |
| `perception` | 197 | -0.05 to defense QR per point, +0.5% to stealth/disguise/reveal checks per point |

Confidence: **HIGH** that these are the designers' stated conversions; **UNKNOWN** how QR maps to Miss/Glancing/Hit/Critical/Double-Critical (`EResultCode`, E2) - no threshold table exists anywhere in the client.

The cooked descriptions are consistent with this convention (E3, cross-check): ability 1629 "Demand Accuracy" says "+200 Accuracy" (= +2.00 QR); ability 1725 "Oppression" says "-1 Offensive QR"; effect 4995 (ability 1884) says "+1 QR Cover Penetration" while ability 1450 "Cover Penetration" says "+100 Cover Penetration" (100 points x 0.01 = 1 QR). **MEDIUM**: the description text is designer prose, not a specification, and many entries are template placeholders (for example dozens of unrelated abilities read "Heals 35% of the player's Focus pool").

## Question 1 - Accuracy vs Defense to hit/miss

| Column | Finding |
|---|---|
| **What the client proves** | (a) Accuracy and Defense are additive QR shifts of +0.01 / -0.01 per point (E1 lines 204-205); `qrMod` shifts attacker and defender QR by 1 per point (E1:206). (b) The outcome vocabulary is Hit / Miss / Critical / DoubleCritical / Glancing (`EResultCode`, E2:252-262) and the client renders them (`Event_Effect_Hit_*`, Kismet enum `ComponentKismetData.ESeqEvent` in `Engine.u` [18318]). (c) QR is resolved **per effect**: `EF_DontUseQR` (bit 16) is set on 754 of 3,216 cooked effects, and of 418 effects whose description matches a Focus/Health damage pattern (loose regex on `F`/`H` notation, so approximate) 409 do *not* set it; `EF_CalculateQRFromTarget` (bit 4194304) is set on 39 effects (E2:1094-1123, E3 flag column). (d) The client sends and receives only the result: `CombatQueue_HandleOnEffectResults` `0x00eb1630` and `getUnitStat` `0x00aebf80` (see [combat-damage-analysis.md](combat-damage-analysis.md)). |
| **What is absent** | Any threshold/curve mapping net QR to hit/miss/glance/crit/double-crit; base hit chance; QR scale ceiling; whether Accuracy is compared to Defense as a difference or as a ratio; range and flank inputs. No such number or comparison exists in SGW.exe strings (`HitChance`, `ToHit`, `QualityRating`, `QRRoll` return no strings), in any `.u` script (only the enum names), or in cooked XML. |
| **Evidence refs** | E1:204-206; E2:252-262, 1094-1123; E3 abilities 1629, 1630, 1725, 1728, 1729, 1736; E6 `0x00eb1630`, `0x00aebf80` |
| **Confidence** | Unit convention HIGH; per-effect QR flag HIGH (names) / MEDIUM (semantics); hit/miss mapping UNKNOWN |
| **Next step** | Recover the server-side effect resolver (see "What would close each gap"). A ranged-fire packet capture from an *original* server (not the 2026 Cimmeria captures in `Working\binaries\sessions`) would give outcome frequencies at known Accuracy/Defense. |

## Question 2 - Low/Medium/High cover, Cover Defense vs Cover Penetration, Crouching Defense

| Column | Finding |
|---|---|
| **What the client proves** | (a) Cover nodes carry a **Height** and a **Quality** enum (E2:207-224, E5): `ECoverHeight` = Low 0, Mid 1, High 2, LOS 3 (Max 4 in UnrealScript); `ECoverQuality` = Good 0, Better 1, Best 2, None 3 (Max 4). Names only - no numeric values attached in the enum. (b) The `SGWCoverNodeComponent` class defaults are `CoverQuality = 3 (QUALITY_None)`, `CoverWidth = 1.0`, `CoverHeight = 0 (Low, enum default)`, `Scale3D = (1.5, 1, 1)`; `SGWSpecCoverNode` defaults are `CoverHeight = 4`, `CoverQuality = 4` (the "Max" sentinel, both marked `deprecated` in script). None of these is a combat bonus. (c) The stock-UE3 `CoverLink` geometric defaults (`StandHeight 130`, `MidHeight 70`, `MaxFireLinkDist 2048`, `AlignDist 34`, `SlipDist 152`, `TurnDist 512`, `LeanTraceDist 64`, `COVERLINK_ExposureDot 0.4`, `EdgeCheckDot 0.25`, `EdgeExposureDot 0.85`) are cover *geometry*, inherited from UE3, not SGW combat design. (d) Cover Defense and Crouching Defense are stats measured in QR: -0.01 QR per point (E1:235, 237); Cover Accuracy / Crouching Accuracy +0.01 QR per point (E1:234, 236); `coverQRModifier` +1 attack and defend QR per point behind cover (E1:216). (e) **"Cover Penetration" is not a separate stat**: `EStats` has `coverAccuracy` (66) and `coverDefense` (67) but no penetration-of-cover stat (E2:468-554), and the cooked text uses "Cover Penetration" and "Cover Accuracy" interchangeably in the same units (abilities 1450, 2201, 2175; effect 4995) - **MEDIUM/LOW inference** that the designers' "Cover Penetration" is `coverAccuracy`. (f) Designer-text magnitudes (E3): ability 1454 "Hunker Down" "+100 Cover Defense" (15 s); 1451 "Cover Stance" "+200 Cover Defense"; 1452/2123 "Duck and Cover" "+100 Crouching Defense"; 1450/2130 "Cover Penetration" "+100 Cover Penetration"; 2201 "Sharp Shooter" "Cover Accuracy +100"; 2175/849 "Sight In" "+200 Cover ACC" (15 s); 1487/2098 "Penetrating Barrage" "Ignores 2 QR of Cover". (g) SGW.exe: cover-node prefab spawner `0x00904d80` selects one of four float constants by `CoverHeight` byte: Low `0.71`, Mid `1.067`, High `1.524`, LOS `2.524` (float32 at `0x018f41d4`, `d0`, `cc`, `c8`; exact `0x3f35c28f`, `0x3f889375`, `0x3fc3126f`, `0x40218938`). |
| **What is absent** | The QR (or point) value a Low, Mid or High cover node grants to the defender, and the quality (Good/Better/Best) modifier. No table keyed on `ECoverHeight`/`ECoverQuality` holds a combat number. The client HUD does not compute a cover bonus either: `Character.lua` and `SelfStatus` display raw stats only (E7). Nothing states how Cover Defense combines with Crouching Defense (additive, max-of, or mutually exclusive). "Ignores 2 QR of Cover" tells us cover is expressed in QR but not what a node's total is. |
| **Evidence refs** | E1:216, 234-237; E2:207-224, 468-554; E3 abilities 1450, 1451, 1452, 1454, 1487, 2098, 2123, 2130, 2175, 2201, effects 1746, 1747, 2003, 4293, 4338, 4706, 4995; E5 `Engine.u` [19465], [19497], [14324], [14493], [4945]; E6 `0x00904d80`, `0x018f41c8-0x018f41d4` |
| **Confidence** | Enum values, class defaults, designer-text magnitudes: HIGH (read directly). The four floats: HIGH as values; **MEDIUM/LOW** as meaning - they are written into a field of the spawned "SGW_Cover.CoverNode" mesh actor (`piVar5[0x99]`, after `StaticLoadObject L"SGW_Cover.CoverNode"`), which points to a per-height **visual/geometry** dimension (0.71/1.067/1.524 m are 28/42/60 inches). They are not a combat modifier. Cover Penetration = `coverAccuracy`: LOW (inference). |
| **Next step** | Server cover-QR lookup (original `SGWCombatant`/ability-resolution script) or a designer spreadsheet. Optional client-side cross-check: the per-node Height/Quality distribution is in `Working\SGWGame\Cache\covernodes_nikols.pak` / `covernodes_sdeiter.pak` (1,331 / 1,185 chunk entries) - it shows which classes exist in maps, not their bonus. |

## Question 3 - Weapon scaling by TechComp / Tier / Quality / Applied Science

| Column | Finding |
|---|---|
| **What the client proves** | (a) The client's item schema carries `Tier`, `TechComp`, `QualityID`, `AppliedScienceID` (`Item.xsd`, `COOKED_ITEM`) and weapon `MeleeRanges` / `RangeRanges` (min/max). (b) Observed value sets across all 6,059 cooked items (E4): **Tier** 1-5 (2,788 / 1,052 / 1,044 / 597 / 578); **QualityID** 1000, 2000, 3000, 4000, 5000 (18 / 5,907 / 68 / 62 / 4) matching `EItemQuality` Poor 1000 ... Fantastic 5000 (E2:1693-1701); **AppliedScienceID** 0-4 (2,143 / 1,119 / 669 / 967 / 1,161) where `CookedSciences.pak` names 1 Biomedical Engineering, 2 Materials Engineering, 3 Power Systems Engineering, 4 Electronic Engineering; **TechComp** 0-100 with Tier bands: Tier 1 ~1-20 (max 100), Tier 2 20-33, Tier 3 33-48, Tier 4 43-50, Tier 5 50-60. (c) Discipline `TechCompetency` takes 1, 2, 5, 10, 15 ... 50 (`CookedDisciplines.pak`). (d) The UI only prints it: `Tech Comp: <n>` in item tooltips (`Inventory.lua:786`, `Vendor.lua:991`, `Character.lua:197`); Lua accessor `getTechCompForSlot` (string `0x01953610`). (e) Weapon ranges are data-driven, e.g. item 21 "SGHC 6 SMG" range 2-30, melee 0-2; item 2797 "Serpent Staff" range 3-30. (f) `EAmmoType` and ability text show ammo qualitatively changes Damage/Penetration (ability 1445 "EMP Ammunition": "Penetration: Decreased, Damage: Increased"). |
| **What is absent** | Base weapon damage, damage-per-tier, per-quality multipliers, any dependence on Applied Science, and the meaning of TechComp beyond a gating/tooltip number. **Cooked items carry no damage, accuracy or stat fields at all** (`COOKED_ITEM` attribute set is closed: E4). Effect and ability XML carry pulse count/duration/delay/flags/description only (`Effect.xsd`, `Ability.xsd`): the magnitudes live in the server. |
| **Evidence refs** | `Item.xsd` (client copy `Common\xml\SGWShared\CookedData\Item.xsd`); E2:1693-1701; E4 items 21, 2797, 2699; `CookedSciences.pak` `_1`-`_4`; `CookedDisciplines.pak`; `Inventory.lua:786`; SGW.exe strings `0x01953610` (`getTechCompForSlot`), `0x0195f064`, `0x0195f3e4` (`techCompentancy`, sic), `0x01b22fcc` |
| **Confidence** | Value sets and correlations: HIGH. Any formula: UNKNOWN. The Tier-to-TechComp band is an observed correlation (**MEDIUM**), not a stated rule. |
| **Next step** | Original item template / stat tables (server DB dump or `SGWItem` script). The item `.pak` already gives the identifier side; only the magnitude side is missing. |

## Question 4 - Armor / Resistance / Mitigation / Penetration and damage-type interactions

| Column | Finding |
|---|---|
| **What the client proves** | (a) Stat semantics from designer comments (E1, verbatim): `physicalAF`/`energyAF`/`hazmatAF`/`psionicAF` "damage armor factor" (207-210); `kineticRes`/`mentalRes`/`healthRes` "increases resistance to all harmful kinetic/mental/health effects by 1% additive" (211-213); `damage` "increases the damage done by all attacks and effects by 1%" (224); `penetration` "decreases the target's final calculated mitigation by 1%" (225); `physicalDensity`..`psionicDensity` "gives 1 additional AF vs <type> damage" (226-229); `negation` "decreases the chance that harmful effects will be resisted fully by 1% subtractive" (240); `mitigation` "armor mitigation percent (0-100%)" (243); absorb pools per type x {plain, Item-charged, Energy-charged} (255-269). (b) Damage-type sets: `EDamageType` Untyped 13, Physical 14, Energy 15, Hazmat 16, Psionic 18 (E2:272-281); a finer `EMitigationType` with 15 values (Physical Impact/Concussive/Slashing/Piercing, Energy Plasma/Radiation/Electrical/Particle, Environmental Biological/Chemical/Thermal/Cold) (E2:404-425). (c) Designer-text magnitudes (E3): ability 1235 "Shield: Universal" "+10% Mitigation" / effect 3148 "+10% Physical Mitigation"; armor items 2937, 2950, 2966, 2984 "30% Mitigation" (each a single armor piece: vest, boots, gloves, legs); ability 3192 "Phasic" "100% Energy Mitigation" (20 s). (d) The client displays PhysicalAF/EnergyAF/HazmatAF/PsionicAF, Kinetic/Mental/HealthRes, Damage, Penetration raw on the Character sheet (`Character.lua:8-12`, `458-472`) and applies the server's final `Delta` (`0x00eb1630`). |
| **What is absent** | The AF-to-reduction conversion (e.g. `AF / (AF + k)`), the order of AF, mitigation%, resistance%, absorb pools, `damage` %, and `penetration` %; whether `mitigation` and AF coexist or one derives the other (a 30% item description suggests `mitigation` is a percentage stat, but how AF feeds it is unstated); how `EMitigationType` sub-types map onto the five `EDamageType` values; `SRC_*` (Absorb/Immune/Mortal) trigger conditions. No AF/armor arithmetic exists in SGW.exe (searches: `Mitigation`, `Negation` return no strings; the stat name table at `0x01956d80` has `PhysicalAF..PsionicAF`, `KineticRes/MentalRes/HealthRes`, `Damage`, `Penetration` as bare labels). |
| **Evidence refs** | E1:207-229, 240, 243, 255-269; E2:272-281, 404-425, 263-271; E3 ability 1235/3148, 3192, items 2937/2950/2966/2984; E6 string table `0x01956d80-0x01957140` (`Penetration` `0x01956e00`, `Damage`, `QrMod`, `Defense` `0x01956fe0`, `Accuracy` `0x01956ff0`) |
| **Confidence** | Stat semantics HIGH (verbatim); combination order UNKNOWN. |
| **Next step** | Server damage pipeline source/config; a controlled measurement is the only alternative (known AF/penetration in, observed `Delta` out) but it measures the *emulator*, not the original, unless run against original binaries. |

## Question 5 - Focus effect on accuracy and on Health damage

| Column | Finding |
|---|---|
| **What the client proves** | (a) **Damage is written as a Focus/Health pair.** In `CookedDataAbilities.pak`, 299 descriptions use the `-<n>F / -<m>H` notation; **288 have Focus:Health = 10:1 exactly and the other 10 are 5:1** (e.g. 1072 `-100F / -10H`, 1242 `-300F / -30H`, 2042 `-1000F / -100H`, 1075 "Damage: 50 Health / 500 Focus"). Damage-over-time uses the same split (1671 "DOT: Focus -50F / -5H"). The `fortitude` (+10 health/pt) and `morale` (+100 focus/pt) comments (E1:195, 196) carry the same 10:1 ratio. (b) **Bonus damage vs low Focus is a designer-stated mechanic:** ability 1359 "Red Mist" "-500F / -50H" and "-1500F / -150H vs Low Focus"; 1978 "Executioner's Fire" "Bonus Damage when opponent Focus is below 50%"; 1247 and 1357 "Execution" "Bonus Damage when opponent Focus is low". (c) Flank/rear damage on the same F/H scale: ability 641 "Surprise Attack" Front -150F / -15H, Flank -200F / -20H, Rear -400F / -40H (effect 1559 "Flank Position Damage" -300F / -30H); ability 1613 effect 4143 "Bonus Damage: Flank" "F-100 H-10". (d) The client treats Health and Focus as independent optional entries in one result list: `SCTMod.verboseEventHit` prints `health.value` and, if present, `[focus.value]` (`SCT.lua:188-210`); `ChatEvents.lua:68` triggers combat chat on `statList[Stat.Health] or statList[Stat.Focus]`; `ChatEvents.lua:146-147` prints "Warning! You are out of focus points." when Focus <= 0; `SelfStatus.lua:19` `FOCUS_WARNING = .30` (UI warning threshold only). (e) Focus also appears as a resource stat: "Heals 35% of the player's Focus pool" (many abilities), "Focus Pool +1000" (2056), Max Focus +15% (1723). |
| **What is absent** | Any statement that Focus (current or max) changes **accuracy/QR** - none of the 60+ stat comments (E1) or ability texts link Focus to hit chance. Also absent: what happens to Health damage when Focus reaches 0 (whether Health damage is gated, multiplied, or independent), the exact "low Focus" multiplier curve (the only data points are 3x for "Low Focus" in 1359 and a 50% threshold in 1978), and whether the 10:1 ratio is a rule or a designer authoring convention. |
| **Evidence refs** | E3 abilities 1072, 1075, 1222, 1233, 1242, 1247, 1357, 1359, 1671, 1978, 2042, 641, 1613, effects 1559, 4143; E1:195-196; E7 `SCT.lua`, `ChatEvents.lua`, `SelfStatus.lua` |
| **Confidence** | F/H pairing and the 10:1 ratio in text: HIGH. Meaning ("Focus is a shield pool depleted before Health"): **not stated anywhere - UNKNOWN**. Focus-to-accuracy: not found. |
| **Next step** | Server `adjustStat`/effect-result code (the wire path carries `Delta` per `StatID`, so a captured original `onEffectResults` with both Health and Focus entries would reveal the routing rule). |

## Question 6 - Buff/debuff stacking order and caps

| Column | Finding |
|---|---|
| **What the client proves** | (a) Additive language for stat stacking: resistances "by 1% additive" (E1:211-213), stealth "1% additive", negation "1% subtractive" (E1:240). (b) Two explicit stacking notes in cooked effects (E3): effect 3663 (ability 2507) "Interruption Resistance +10% - Stacks with Remove Distractions"; effect 3664 (ability 2508) "Interruption Resistance +20% - Stacks with Reduce Distractions", both flagged `EF_AlwaysPersist` (0x80000). (c) Immunity is implemented as a **moniker marker** (effect 4575 "Adds Immunity_Calm moniker to target", effect 4668/4989 "Check for IMMUNITY_Calm moniker") rather than a DR formula. (d) Effect lifetime flags exist in `EEffectFlag` (E2:1094-1123): ClearOnDeath 668 effects, ClearOnDamage 98, ClearOnRez 3, AlwaysPersist 33, OfflineTime 188, RemoveOnBandolierSlotChange, RemoveOnStealthZeroed, RemoveOnDisguiseZeroed. (e) Client side, active effects are keyed by `SecondaryId` and re-timed (`EffectSet_HandleOnTimerUpdate` `0x00e09160`, see [effect-execution-model.md](effect-execution-model.md)). |
| **What is absent** | Stacking order (base then additive then multiplicative), same-stat-different-source rules, refresh-vs-stack semantics, per-stat caps, and any diminishing-returns table. `alias.xml` shows `Max` fields per stat instance but no cap constants; the earlier finding that the client holds no DR/stack limits is reconfirmed (searches: `Stack`, `Diminish`, `Cap`, `Immun`, `Highest` in ability/effect text return only the entries above). |
| **Evidence refs** | E1:211-213, 240; E2:1094-1123; E3 effects 3663, 3664, 4575, 4668, 4989; E6 `0x00e09160` |
| **Confidence** | HIGH for the two notes and the flag set; ordering/caps UNKNOWN. |
| **Next step** | Server effect-application code (`EffectScript` refcount/priority logic) or original design doc for "stat modifier layers". |

## Question 7 - AoE vs cover / line of sight

| Column | Finding |
|---|---|
| **What the client proves** | (a) AoE geometry constants (E2:11-32): `AE_RADIUS` Melee 2.5, Short 5, Medium 10, Long 15, Extreme 20; `AE_CONE` Melee 5, Short 20, Medium 35, Long 45, Extreme 70; `AE_ANGLES` Beam 5, Narrow 20, Medium 35, Wide 45, Extreme 70; `AE_CONE_STARTSIZE` 0.33; the enumeration is marked `BW_TO_UE3_DIST_CONVERT` (BigWorld metres to UE3 units). Target-collection methods: Single, AERadius, AECone, Group, Aura, RandomSingle (E2:307-318). (b) LOS is a **precondition with player feedback**: `ErrorStrings.pak` `_39` = "You do not have Line of Sight to your target" (moniker `CONDITION_FEEDBACK_LOS`); positional feedback strings `PositionCheckFlank` `_19` and `PositionCheckNotFlank` `_4`. The client has a debug LOS actor (`SGWLOSActor.setLOS(start, end, clear)` in `SGWGame.u` [695], drawing only). (c) Designer text ties AoE to cover: ability 2979/effect 4338 "Cover Debuff - AOE: Medium - -400 Cover Defense" (4 s); ability 3166/effect 4706 "PhaseMatter: Cover Defense debuff -100, AoE Pulse: 30 pulses"; 808/effect 853 "Cover Denial: LMG AOE Medium Radius -300F/-30H"; 1487/2098 cone "Ignores 2 QR of Cover". (d) Cover exposure is a cone/slot concept in UE3 script (`CoverLink.ExposedFireLinks`, `FireLink` structs in [18330]) - navigation/AI geometry, not a damage rule. |
| **What is absent** | Whether AoE targets are LOS-tested, whether AoE damage is reduced by cover QR or ignores it, cover-vs-cone interaction, and falloff by distance. No such rule exists client-side (SGW.exe `Falloff` strings are lighting/nav, unrelated). |
| **Evidence refs** | E2:11-32, 307-318; `ErrorStrings.pak` `_39`, `_19`, `_4`; E3 abilities 808, 1487, 2098, 2979, 3166, effects 853, 4338, 4706; `Engine.u` [18330]; `SGWGame.u` [695] |
| **Confidence** | Constants HIGH; the "AoE ignores or respects cover" question UNKNOWN. |
| **Next step** | Server target-collection code (`TCM_*` handlers) and the LOS check; original cell-app "line of sight" helper. |

## Ghidra spot-check (step 4) - what was run, so negatives are reproducible

Program: SGW.exe in project `SGW`. All read-only.

| Probe | Result |
|---|---|
| `list_strings` with filters `CoverDefense`, `CoverAccuracy`, `Crouching`, `Stabiliz`, `Awareness`, `Mitigation`, `Negation`, `HitChance`, `ToHit`, `QualityRating`, `QRRoll`, `DamageMod`, `Flank` | **No strings.** `Tracking` matches only unrelated strings (`Event_Editor_CameraFixedTracking`, PhysX). So the client's own stat-name table has no `coverDefense`/`crouching*`/`tracking`/`stabilization`/`awareness`/`negation`/`mitigation`; those exist only in the server-shared `alias.xml`/`enumerations.xml` (`EStats` ids 62-64, 66-69, 72, 80). |
| Stat-name table `0x01956d80-0x01957140` (UTF-16) | Names only: `...Level`, `PVPFlag`, `TrainingPoints`, `AppliedSciencePoints`, `Property`, `Penetration` (`0x01956e00`), `Damage`, `DeploymentAmmo`, `AmmoSlot5..1`, `CoverQRModifier` (`0x01956eac`), `RangeModifier`, `RevealRating`, `DisguiseRating`, `StealthRating`, `HealthRes`, `MentalRes`, `KineticRes`, `PsionicAF`, `HazmatAF`, `EnergyAF`, `PhysicalAF`, `QrMod`, `Defense` (`0x01956fe0`), `Accuracy` (`0x01956ff0`); `Glance` `0x01957138`. Referenced only from the giant Lua-binding registration function `0x00acbb10-0x00ad6b65` (Ghidra name `CEGUI_ButtonBase_3`, a misnomer): xrefs `0x00ad296f`, `0x00ad2a62` (`Defense`), `0x00ad282d` (`Glance`). **These are enum label tables for Lua, with no arithmetic.** |
| `getUnitStat` `0x00aebf80` | Sole caller is the Lua binding `0x00aa0ba0` (error string `#ferror in function 'getUnitStat'.`): a read accessor, no math. |
| `0x00904d80` | Decompiled: the cover-node prefab mesh spawner (asserts `SGWCoverNodeComponent.cpp` lines 0xbb/0xbc, `StaticLoadObject "SGW_Cover.CoverNode"`); writes one of `0x018f41c8..d4` floats per `CoverHeight` byte. Node record stride 0x18 (+0 x, +4 y, +8 z, +0xC width, +0x10 orientation, +0x14 height byte, +0x15 quality byte), matching the editor CSV header at `0x01a3f2d0` (`CoverNodeHeight,CoverNodeWidth,CoverNodeQuality,OrientationRadians`). |
| `0x00c81b60` | Reads a float property `"Multiplier"` (default `g_flOnePointZero` = 1.0) and forwards it in a NetOut event: a GM/dev slash-command relay (the specific command is not yet identified). Forwarded to the server, not applied locally. |
| `0x00eb1630` `CombatQueue_HandleOnEffectResults` | Re-confirmed as the prior finding: consumes `Delta`, no recalculation. |

Not re-run (already covered): the `UIHitType` / `UIDamageType` Lua enum contents and the `0x01e6ce00` result-code table in [combat-damage-analysis.md](combat-damage-analysis.md).

### Ghidra task list for the owner (if a deeper client-side sweep is wanted)

1. **Confirm no client-side QR/predicted hit anywhere.** From `CombatQueue_HandleOnEffectResults` (`0x00eb1630`) and `GameEntityManager` `onEffectResults` (`0x00e00e60` for `onStatUpdate`), list every function that reads a `Stat` object's `current`/`max` (struct at `GameBeing+0x160` std::map, 7 floats per entry, accessor `0x00aebf80`). Expected: display/tooltip and stat-bar code only. Any function that multiplies two stat values would be a lead.
2. **Float-constant sweep.** `search_byte_patterns` for IEEE-754 candidates of the alias.xml scale factors (`0.01` = `0x3c23d70a`, `0.05` = `0x3d4ccccd`, `0.005` = `0x3ba3d70a`, `0.1` = `0x3dcccccd`); expected hits are unrelated UI/animation code, and any hit in `SGWTextCommandMgr`/`CombatQueue`/`SelfStatus` deserves a look.
3. **Identify the `Multiplier` slash command** (caller chain of `0x00c81b60`, strings near `0x019adbc4`) to complete the dev-command inventory; it is very likely a `SetDamageMultiplier`-style GM tool, which would confirm a server-side damage multiplier exists.
4. **Cover-height caller.** Trace `getUnitCoverHeight` (string `0x01955658`, error string `0x0193fbb8`) to its native implementation and check whether it reads the same four floats; `cover-system.md` currently attributes those floats to it (see "Discrepancies").
5. **x64dbg (live client) option.** With a non-freezing logging breakpoint on `0x00eb1630` (per the project's x64dbg rule), record `Delta` values for known ability/target combinations from the *emulator*; this measures Cimmeria, not the original, so use only to validate a candidate formula, never as its source.

## Not evidence (excluded on purpose)

| Item | Why excluded |
|---|---|
| `Working\binaries\sessions\*.pcap` (9 files, 2026 dates) | Captured against the Cimmeria emulator, not an original server. |
| `Working\binaries\SGWDebugLog.log` (last written 2026-09-17) | Runtime log of the emulator sessions. Contains no combat-math strings. |
| `SourceCache.en-us\CookedBehaviorEvents.pak`, `CookedDataKismetSeqEvent.pak`, `CookedDataKismetSetEvent.pak`, `CookedInteractionSet.pak` | 2026-dated (Discord stub / merged), irrelevant to combat; the rows used above come from the 2008-12-11 QA cook. |
| Ability/effect description placeholders ("Heals 35% of the player's Focus pool" on unrelated abilities, "Effect ability template", "No ability description available ...") | Template noise. A description is only used above when the text and the ability name are coherent and it uses the alias.xml units. |

## Discrepancies noticed in existing docs (for the doc owner to reconcile)

1. [cover-system.md](cover-system.md) attributes the four cover-height floats (`DAT_018f41d4/d0/cc/c8`) to `getUnitCoverHeight` at `0x00904d80` and describes them as an enum-to-float map. The decompile shows `0x00904d80` is the **cover-node mesh spawner** (E6); the values are `0.71/1.067/1.524/2.524` and look geometric. `getUnitCoverHeight`'s own implementation was not located here (Ghidra task 4).
2. [cover-system.md](cover-system.md) says `CoverQRModifier` at `0x01956eac` is a client-side "cover quality HUD" property. That address is one entry in the Lua stat-name table (E6), next to `AmmoSlot1` and `RangeModifier`; no HUD consumer was found.
3. [stat-scaling-formulas.md](stat-scaling-formulas.md) header says "formulas from designer comments in alias.xml, validated against binary" and titles a table "Derived Stat Formulas (Server Implementation)". The rows are paraphrases of the alias.xml comments (E1, ORIGINAL-DATA); the binary contains no such math, so "validated against binary" cannot mean the arithmetic. The base terms ("base + ...") are not defined by any client artifact.
4. [combat-damage-analysis.md](combat-damage-analysis.md) lists `UIStatResultType` as Immune 1, Absorb 2, Mortal 3, while `enumerations.xml` `EStatResultCode` (E2:263-271) has Absorb 1, Immune 2, Mortal 3. They may be different enumerations (UI vs wire); nobody has verified the wire side against the Lua side. The prior finding's other claim (client has no formula) is **reconfirmed**.

## What would close each gap

| # | Gap | What would close it |
|---|---|---|
| 1 | QR to hit/miss/glance/crit mapping; Accuracy vs Defense combination | Original server resolver source (the BigWorld cell `SGWAbilityManager`/effect-resolution scripts) or a designer combat spec; alternatively an original-server packet capture with known stats. |
| 2 | Low/Mid/High cover values; quality modifier; Cover Def + Crouch Def combination | Same server resolver plus the cover height/quality to QR table (likely in `SGWCombatant` or a config); designer cover doc. |
| 3 | Weapon damage by TechComp/Tier/Quality/Applied Science | Original item/weapon stat tables (server DB or `SGWItem` data), or a designer weapons spreadsheet. |
| 4 | AF/Resistance/Mitigation/Penetration order and curve | Server damage-pipeline code and per-type absorb ordering; `EMitigationType` to `EDamageType` mapping table. |
| 5 | Focus role (shield pool? gating? accuracy?) | `adjustStat` server implementation (the `mitigation: FLOAT` parameter) and an original `onEffectResults` capture showing Health and Focus deltas together. |
| 6 | Stacking order, caps, DR | Server effect-application code and per-effect group/priority data (not in cooked client XML). |
| 7 | AoE vs cover/LOS | Server target-collection (`TCM_*`) and LOS routines; designer AoE notes. |

## Fan candidates (not consulted, not evidence)

The project owner ruled that the legacy Python (for example `deprecated/python/cell/AbilityManager.py` `DamageCalc`, `deprecated/python/common/Config.py` `QR_*` constants) is a previous fan developer's best guess. It was **not read** for this document and nothing above depends on it. If a fan candidate is ever tested, it should be validated against E1/E3 unit conventions above and labelled a candidate.
