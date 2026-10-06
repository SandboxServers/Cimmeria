---
name: dakara-e1-evidence
description: Dakara_E1 (worlds 61/62) evidence base from the 2026-10-06 audit - which missions belong to the zone, what the dialogs say about the start, the one recovered location clue, and what no source holds.
metadata:
  type: project
---

# Dakara_E1 zone evidence (audited 2026-10-06, main @ ddd549797)

Full audit: `docs/analysis/dakara-e1-rebuild/audit.md`. Campaign prefix `DK-`.

- **Twelve missions play on world 61**, all labelled "General", so a label
  search misses them: 1570 SG-18 from step 4906 on, and 1645-1655. Order comes
  from the name strings `DN_ms_A00_DakaraE1_sgc_01SG18` .. `_12Shutdown` in
  `texts.sql`. Dialog sets: 1656 (1570) and 1832-1842 (1645-1655).
- **The arc is level 3-5 and assumes an SGC start.** Jaffa-addressed dialogs
  5811 ("Welcome back, my friend"), 6110 (Rak'nor) and 5855 ("liaison to the
  Jaffa on Dakara") have the Free Jaffa arriving from the SGC after the attack.
  A level-1 Dakara start is a project decision, not recovered data.
- **Only recovered location clue:** dialog 6110, Rak'nor: the command tent is
  "just to the east of the Stargate, next to the healing tent". The client names
  one respawner, `DN_Respawner_DakaraE1_MedTent`. No coordinate for any actor.
- **Nothing recovered anywhere** (seed, client map, external archives): NPC
  positions, hostile templates, encounters, offer rules, rewards. The client's
  cooked missions carry only `TaskType="1"` tasks with no parameters.
- **Map gameplay Kismet is three things:** the gate (event set 10013), two
  Ha'tak show/explode sequences (event sets 1194, 1195; unreferenced), and two
  outgoing ring transporters under the Ha'taks (mission 1652 devices, not a
  travel network).
- **World 62 is one tent interior**, instanced; the client names four tent
  flaps (to/from the command tent and Moh'katan's tent).
- **The SGC is a one-way trip today:** worlds 58 and 86 have no DHD and no
  Harriman dial chain. Omega Site (gate 5) has a DHD.
- Templates exist for Bra'tac (59) and Moh'katan (54, shared with Harset);
  speakers 2956 and 2959 (empty names) are the gate greeter and gate commander.

**Why:** the "final RE" handoff archive's Dakara_E1 mission list has only 8 of
the 12 (it appears to join missions to name strings by display text, and four
of the Dakara strings have empty text), and the earlier handoff pack says the
starter begins at 1645. Both are wrong against the client.

**How to apply:** start any Dakara chain work from the seed and the client, not
from a handoff's mission list. See [[multi-chain-dispatch-semantics]] for the
Human/Jaffa sibling split and [[dialog-chain-authoring-rules]] for the radio
dialogs (NPC speaker, no NPC present).
