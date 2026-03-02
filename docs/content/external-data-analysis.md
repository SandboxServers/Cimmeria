# External Game Data Analysis

> **Source**: Analysis of 11 spreadsheets and text files assembled by the external dev team
> **Date**: 2026-03-02
> **Files analysed**:
> - `Missions.xls`, `Mission_Steps.xls`, `Mission_Objectives.xls`
> - `SGW_ Milky Way Missions.xlsx` + `SGW_ Milky Way Missions_Dev_Ver(1).xlsx`
> - `SGW_ Progression.xlsx`, `SGW Timeline.xlsx`
> - `Planets & Locations.xlsx`, `Address Database.xlsx`
> - `Archetypes.xlsx`, `Spawn List(1).xls`
> - `Dialogues.txt`

These documents are the result of years of community research, independent from the server emulator's own database. They document story intent, world progression, NPC placement, dialogue scripts, and mission design. Cross-referencing them against the server data (see [content-inventory.md](content-inventory.md)) fills in significant gaps, particularly for zones that are SHELL or have no Python scripts.

---

## Key Finding: Release 1 Target Zones Are Well-Documented

The three zones identified as Release 1 priority — **Castle, Harset, Tollana** — are the single best-documented slice in all external data. They represent the complete early-game loop for the Praxis factions (Goa'uld + Loyalist Jaffa). The data needed to make them functional exists; the work is wiring it to the server.

---

## 1. Story & World Structure

### Source: `SGW Timeline.xlsx`

The timeline documents a dual-faction narrative (**Stargate Union** vs **Praxis**) across 3 Episodes and 8 Chapters. The setting is Season 8.5: after S8, before S9. The original Ba'al is alive. The Asgard have splintered into two groups.

### Episode 1, Chapter 1 — FULLY WRITTEN

**Castle (Praxis/Loyalist Jaffa starting zone):**
Players wake from stasis in a Goa'uld prison-fortress repurposed by the NID. They fight out alongside Col. Marsh and Moh'katan. A dying prisoner named Lethander delivers a warning about an incoming alien threat, and the gate address for Harset is burned into the minds of all present. A flashback reveals the Goa'uld player's history with Anat and Ra in ancient Egypt. The faction (Praxis) then moves to Harset as its base of operations.

**Omega Site (SGU/Asgard/Free Jaffa starting zone):**
Ba'al attacks the SGC as a ruse to extract database intel; the SGC relocates to Omega Site, an Asgard-built facility in Cronus's old fortress. Asgard consciousness-transfer to drone bodies is completed. Free Jaffa consolidate on Dakara. All three factions form a joint leadership at Omega Site (Asgard station to the north, Jaffa section to the north, human section south, eastern vault boarded off).

**Harset (hub world — both factions):**
Former Ra-controlled planet. Layout:
- East: OP-Core sector
- West: Jaffa sector
- North: Goa'uld sector + black market (under the north pyramid)
- South: Goa'uld sector + jail (south pyramid)
- Central: Command center (Praxis leadership: Ba'al, Anat, Moh'katan, Marsh)

### Chapters 2–8 — STUBBED

Bullet-point outlines exist for Chapter 2 (Beta Site / Tollana) and location names for Chapters 3–8, but the actual story text is blank. All content below is inferred from mission data and dialogue, not the Timeline.

---

## 2. Faction Progression Map

### Source: `SGW_ Progression.xlsx`

Five faction paths through the full game world. Each row is one zone stop in sequence:

| Faction | Starting Zone | Hub | Content Path (in order) |
|---|---|---|---|
| Asgard | Pertho → Asgard High Council | Omega Site | Beta Site → Men'fa → Ihpet → Hebridan → Lucia → Pen-Lai → Dakara → Agnos → Tartarus → Vitrus |
| Free Jaffa | Dakara | Omega Site | Beta Site → Men'fa → Ihpet → Hebridan → Pen-Lai → Agnos → Tartarus → Vitrus |
| SGU Human | Earth (SGC) | Omega Site | Beta Site → Men'fa → Ihpet → Agnos → Tartarus → Vitrus |
| Goa'uld | Earth (Egypt Past) | **Harset** | Tollana → Men'fa → Naitac → Harset → Lucia → Hebridan → Harset → Agnos → Tartarus → Vitrus |
| Loyalist Jaffa | **The Castle** | **Harset** | Tollana → Pen-Lai → Men'fa → Naitac → Ihpet → Dakara → Agnos → Tartarus → Vitrus |

The Harset/Tollana/Castle triad is the entirety of early-game content for two of five factions. Completing it delivers a complete playable arc.

---

## 3. Mission Data

### Sources: `Missions.xls`, `Mission_Steps.xls`, `Mission_Objectives.xls`, `SGW_ Milky Way Missions_Dev_Ver(1).xlsx`

The dev-version missions spreadsheet has 41 location tabs. Of those, ~10 contain actual mission data. The rest are empty header rows. Summary:

| Zone | Mission Rows | Notes |
|---|---|---|
| Castle (CellBlock) | 30 | Complete chain from wake-up to escape |
| Castle (main) | 30 | Complete chain from breach to gate |
| Harset | 100+ | Predominantly Praxis-faction missions |
| Men'fa Dark | 11 | Two missions, levels 10–12 |
| Tollana | 16 | Three named mission arcs |
| Pen-Lai | 14 | Two missions |
| Beta Site E2 | 20 | Three missions (level 33–38) |
| Asgard High Council | 9 | One mission ("The High Council", finds Thor) |
| Agnos | 100 | Extensive, levels 36–49 |
| All other zones | 1 each | Header row only |

### Castle Mission Chain (complete)

```
Arm Yourself (L1)
  └ Speak to Prisoner 329
  └ Hack cell door
Find Ambernol (L1)
  └ Defeat drone
  └ Retrieve Ambernol vial from desk
  └ Use Ambernol to cure stasis sickness
Hack the Rings (L1)
Preparation (L1)
  └ Speak to Col. Marsh
  └ Open stasis pods via terminal
Escape the Cellblock
  └ Navigate lockdown

[Main Castle]
Reinforce Copplemann → free Copplemann from security boot → escort to safe zone
Rescue Dr. Zuritska → reach Interrogation Block → locate and free Zuritska
Payback → find and eliminate NID Interrogator Romney
Hack Communications → escort Zuritska to Comms Room → download security specs → deliver crystal
Power Behind the Throne → reach Throne Room → hack Goa'uld comm grid via panel behind throne
Secure the Stargate → discover why gate won't dial
  ├ Option A: interrogate guard
  └ Option B: use Access Panel on Throne
  Retrieve DHD Control Crystal
  ├ Option A: get it from NID Officers at Checkpoint Bravo
  └ Option B: kill Warden Muelbach in the bunker above Checkpoint Bravo
  Repair the DHD
```

### Harset Mission Sample (Praxis faction, selected)

| Mission | Level | Summary |
|---|---|---|
| Meet Your Queen | 1 | Convince Anat's Royal Guard to grant audience; speak to Anat |
| Plant Spy | 6 | Extract symbiote from Anat's tank; infest Storage Officer Dawson; gather surveillance footage |
| Infiltrators | 1 | Examine three shield towers for sabotage; use Ba'al's tracker; report to Ba'al |
| Invasion Plans | 1 | Scan Command Center Lab, Market, Jaffa Zone, Storage Room |
| Harset Supplies | 10 | Consult Machra; eliminate competitors Piran and Jowan on Tollana; supply chain established |

### Tollana Mission Sample

| Mission | Level | Summary |
|---|---|---|
| Recruit Morrigan | 1 | Go to Goa'uld Resort; convince Morrigan to join Praxis; save her from assassin Rithbe; deliver letter to Ba'al |
| Symbiotes | ~13 | Deliver staff weapons to Lethander; extract symbiote from Anat's tank; return to Lethander; collect Svarog's staves |
| Deliver Symbiote | 13 | Give symbiotes to Copplemann for study |

### Mission Names from Raw XLS (not in tracking sheet)

These names exist in the binary mission database but have not been cross-referenced to zones:

> *"The Fall of Dakor", "Hide and Seek", "Gaap's Goings On", "Asmodai's Interests", "Enemy at the Gates", "Withdrawal Orders", "Moh'katan's Scouts", "Left Behind", "Priorities", "Getting Even", "What's Behind Door Number Two?", "Walter's Rescue", "Disarming the Self-Destruct", "Baksheu Who?", "The Short, Grey Cavalry", "The Scientific Method", "They're Stealing Our Stuff!", "Rumors of My Demise...", "My Three Sons", "Follow the Bodies", "Engineering Reversed", "Treasure Map of the Stars", "The Last Son of Ras'nor", "Take Two of These...", "Burtonol Bribe", "Dirt on Jordin", "To Hebridan", "Heist Plans", "Investigate the Reins of the Sun Chariot"*

---

## 4. Dialogue

### Source: `Dialogues.txt`

| Metric | Value |
|---|---|
| Total lines | 25,947 |
| Dialog entries | 5,405 (IDs #11–#6,427) |
| Unique NPC speaker IDs | 601 |
| Multi-line conversations (3+ lines) | 1,819 |

The writing quality is high — full character voice, branching responses, stage directions. The Vala → Netan → Opheltes quest chain on Tollana/Pen-Lai is a complete, polished script. The Agnos chapter (Hammond, Jackson, Carter, Thor, Ba'al discussing the Ancient Library) has rich multi-party conversations.

### Identified Speaker IDs

| ID | NPC | Confidence | Basis |
|---|---|---|---|
| 156 | Vala Mal Doran | HIGH | Named in dialogue; distinctive voice ("You know you love me") |
| 209 | Netan | HIGH | "I am Netan" — dialog #181 |
| 389 | Kefflin (Netan's lieutenant) | MEDIUM | Context: works for Netan, leads searches |
| 399 | Opheltes (slave trader Goa'uld) | HIGH | Confirmed dialog #210 |
| 431 | Dr. Daniel Jackson | HIGH | Addressed as "Dr. Jackson" by Hammond, dialog #1138 |
| 496 | Lethander | HIGH | "I am Lethander" in dialogue |
| 706 | Podhertz (drug chemist) | MEDIUM | Context: "I'm an independent contractor" |
| 777 | Thor | HIGH | Asgard voice, "We have no record of this world" — dialog #1138 |
| 941 | Colonel Marsh | HIGH | Addressed as "Colonel" by faction council, OP-Core references |
| 942 | Ba'al | HIGH | Harset council scenes, "those are his Hataks in the sky over Harset" |
| 943 | Nerus | HIGH | "I have specific dietary requirements" — food obsession confirmed |
| 944 | Anat | MEDIUM | Harset queen role, missions referencing "her Royal Guard" |
| 945 | Moh'katan | HIGH | Jaffa context, Harset references, "They are honorless" Jaffa speech |
| 968 | Capt. Copplemann | HIGH | "Copplemann here" — radio dialog |
| 1008 | Unknown (high line count) | UNKNOWN | 96 lines — likely a major Harset NPC |
| 2410 | Gen. Hammond | HIGH | Mission briefing role; refers to "Dr. Jackson" by name, Omega Site context |
| 2568 | Col. Carter | HIGH | Technical briefing role alongside Hammond ("We've sent a MALP through") |

**Critical gap:** Dialogue IDs are not mapped to mission steps in these files. The table that says "mission step X plays dialog Y" would be in the original game scripts or DB. The 5,405 dialogs exist but the trigger wiring is not present in external data.

---

## 5. Locations & Coordinates

### Sources: `Planets & Locations.xlsx`, `Address Database.xlsx`

### Functional Gate IDs (usable with `.giveaddress <id> 0`)

| ID | Planet | Gate In | Gate Out | Notes |
|---|---|---|---|---|
| 2 | The Castle | YES | NO | One-way in — appropriate for prison |
| 3 | Harset | YES | YES | Hub world |
| 4 | Tollana | YES | YES | First content world (Praxis) |
| 5 | Omega Site | YES | YES | Hub world (SGU) |
| 6 | Beta Site | YES | YES | First content world (SGU) |
| 7 | Men'fa (Praxis) | YES | YES | |
| 8 | Ihpet (Praxis) | YES | YES | |
| 10 | Lucia | YES | YES | |
| 14 | Pen-Lai | NO | NO | DB-registered but no gate travel |
| 15 | Agnos | YES | NO | One-way in — LARO-controlled world |
| 20 | Ihpet (SGU) | YES | YES | |
| 22 | Men'fa (SGU) | YES | YES | |
| 25 | Dakara | YES | YES | |
| 26 | Pertho | NO | NO | Asgard drone homeworld |
| 28 | Asgard High Council | NO | NO | Phased space station — no gate travel |

### Milky Way Map Coordinates

Agnos is the most completely documented world — XYZ coordinates for every named sub-area (city stargate, LARO hub, atmosphere generators ×2, research facility, repair node, robotics monitoring facility, tower of ascendancy, etc.). Beta Site has complete coordinates for all sub-areas. Most other worlds have place-name lists without coordinates.

---

## 6. Archetypes

### Source: `Archetypes.xlsx`

7 species × 3 specializations = **21 archetypes**. Names and client-side IDs are complete. No ability trees, stats, or progression data in this document.

| Species | Spec 1 | Spec 2 | Spec 3 |
|---|---|---|---|
| Archaeologist | Anthropology | Archaeology | Sociology |
| Asgard | Attack Programs | Defensive Programs | Scientific Programs |
| Commando | Demolitions | Munitions | Stealth |
| Goa'uld | Ash'rak | Battle Lord | Servant Lord |
| Jaffa | Alien Lore | Goa'uld Specialisation | SGU Specialisation |
| Scientist | Medical Science | Robotics | Support |
| Soldier | Automatic Weaponry | Command | Heavy Weaponry |

**Release 1 scope (Human + Loyalist Jaffa only):** expose `Commando`, `Soldier`, `Scientist`, `Archaeologist` (3 specs each) and `Jaffa` with both `Goa'uld Specialisation` and `SGU Specialisation`.

---

## 7. Spawn Data

### Source: `Spawn List(1).xls`

Named entity spawn IDs exist for:

**Castle (complete):**
`Prisoner_329`, `ArmYourself_NIDGuard`, `ArmYourself_AmbernolVial`, `ArmYourself_PrisonerRetrievalUnit`, `Cellblock_FakeRingSwitch`, `Cellblock_WoodenCrate`, `Preparation_ColMarsh`, `Preparation_RingSwitch`, `Castle_ColMarsh`, `Castle_Mohkatan`, `Castle_PrJaffaGuard1/2/3`, `Castle_PrJaffaLieuternant`, `Castle_DHD`, 17+ named NID guards by position

**SGC W1 (complete):**
`SGCW1_GenHammond`, `SGC_W1_Tealc`, `SGC_W1_SamCarter`, `SGC_W1_Airman` (variants), `SGC_W1_GroomJaffa1/2/3`, `SGC_W1_JaffaBomb`, `SGC_W1_NaqBomb`, `SGC_W1_GateTerminal1`

**Transport infrastructure (DHDs + ring sets):**
Harset, Tollana, Beta Site, Lucia, Ihpet Crater (both versions), Men'fa Dark (15 ring endpoints), Dakara E1, Omega Site — all named with coordinate data

---

## Cross-Reference: External Data vs Server Data

Where this external data agrees with or extends what the server DB and Python scripts show:

| Finding | External Data | Server Data | Status |
|---|---|---|---|
| Castle is the Praxis starting zone | Confirmed in Timeline + mission chain | Castle_CellBlock has 511-line Python script, 15 scripted missions | **CORROBORATED** |
| Harset is the Praxis hub | Confirmed in Timeline + 94 dialog references | Harset has 71-line script, 1 of 26 missions scripted | **CORROBORATED — scripts need work** |
| Tollana is first Praxis content world | Confirmed in Progression doc + 16 mission rows | Tollana is SHELL: 3 DB missions, no scripts | **EXTERNAL DATA IS AHEAD** |
| Thor speaker ID 777 | Identified in dialog analysis | NPC ID 777 in entities DB | **TO VERIFY** |
| 5,405 dialogs in Dialogues.txt | Count in external file | Server DB has 5,411 dialog entries (6 discrepancy) | **CORROBORATED** |
| 602 unique NPC speaker IDs | From Dialogues.txt | 602 speakers in DB | **EXACT MATCH** |
| Archetypes: 7 species × 3 specs | Archetypes.xlsx | DB has 2 implemented (Soldier, Commando), 5 stub | **EXTERNAL DATA SHOWS INTENDED STATE** |

---

## Data Gaps (Not in External Files)

1. **Mission-to-dialog junction** — which mission step triggers which dialog ID. Lives in original Python scripts or DB triggers — not in these spreadsheets.
2. **Item/loot tables** — missions reference naquadah rewards and XP but no item database exists in this dataset.
3. **Level/XP curve** — missions have minimum level requirements (1–49+) but no XP-per-level table.
4. **Minigame specifications** — mission objectives frequently say `TODO: Minigame`. No minigame design data exists.
5. **Enemy stats** — no HP, damage, faction, or AI data in any external file.
6. **Map coordinates for most worlds** — only Agnos and Beta Site have complete XYZ coordinate sets.
7. **Most world missions** — 31 of 41 tracked location tabs are empty. Full mission data for Omega Site, Dakara, Pen-Lai, Beta Site, Hebridan, Lucia, Ihpet, and all late-game worlds is missing from tracking documents (raw names/step text exist in binary XLS but are unstructured).

---

## Release 1 Readiness Assessment

### Castle — ~85% data-complete
- Full mission chain with step/objective IDs
- All key NPCs named in spawn list (Marsh, Moh'katan, Copplemann, Zuritska, Prisoner 329, Lethander)
- Dialogue exists for all major characters
- Map layout described in Timeline
- **Gap:** NPC-to-speaker-ID mapping needs full verification

### Harset — ~70% data-complete
- Large mission set documented (100+ rows)
- Named NPCs: Ba'al, Anat, Nerus, Machra, Moh'katan, Lethander
- Hub layout fully described in Timeline
- DHD and ring transport spawns present
- **Gap:** SGU-side missions untracked; interior NPC coordinates mostly absent

### Tollana — ~60% data-complete
- 16 mission rows with step data
- Key NPCs: Morrigan, Netan, Vala, Lethander, Travell, Councilors Rinak and Geran
- Dialogue for Netan/Vala/Morrigan/Opheltes chain is complete and polished (30+ dialog entries)
- DHD and ring transport spawns present
- **Gap:** Full map coordinates missing; most supporting NPC spawn positions absent

### Overall: The data exists. The engineering is the bottleneck.

The Castle → Harset → Tollana loop has enough content data to implement. The blockers are not missing story or mission data — they are:
1. Wiring mission steps to dialog IDs (requires script authoring or DB updates)
2. Placing NPC spawns with world coordinates (requires map work)
3. Enemy stat data for anything that needs to be killed/looted (requires design work or estimation)
4. Mission reward population (XP/naquadah — formulaic, not blocked by data)
