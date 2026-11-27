# NPC combat reference: roster follow-up spec (NC-34)

Packet: NC-34. Campaign: `docs/analysis/npc-combat-reference/`. Read with
[work-packets-w3-w5.md](work-packets-w3-w5.md) § NC-34 and the rules in
[work-packets.md](work-packets.md). This note proposes one follow-up
campaign for the six roster NPCs that the reference campaign left out. It
changes no seed, no code and no test. Assignment waits for the W4 UAT
(brief § Definition of done) and for a fresh "may I use the lab now?" before each run (D-NC7, decided).

## Why these six need a campaign

- All six are creature (`MOB_*`) or Jaffa-body templates with **no ability
  set** (`ability_set_id` NULL) and **no `WP-*` component**. The
  [placeholder inventory](placeholder-inventory.md) gives them the verdict
  `holds fire` (rows 71-75, 105-110, 132-137 and the lineup rows).
- Under `Legacy` (today) they fall back to 592 Pistol Shot through
  `NPC_DEFAULT_ABILITY`. The hostile AMBRat (template 74) fires a pistol
  today, as the [hub-pass UAT README](../hub-pass-uat/README.md) records.
- Under `RequireWeaponEvidence` (D-NC1, NC-30) they would idle and log a
  warning. Each one needs an ability set before that flip, or it stays idle.
- ScavDog (template 76) is the only creature with a set (set 6, `Mob
  Strike`, RECONSTRUCTED under D-NC5). It is the pattern for the organic
  roster.

## Roster

| Roster NPC | Templates (world / lineup) | Body family and components | Matching reference pipeline | Candidate abilities and the evidence each still needs | Packet shape |
|---|---|---|---|---|---|
| Carnosaur | 71 / 1559 (name_id 26846) | creature `MOB_Carnosaur.BS_MOB_Carnosaur`; `MOB_Carnosaur.BC_MOB_Carnosaur` | organic: ScavDog (NC-13) | 983 `Bite` (unassigned, EARLIEST LEGACY DATA, effect 1092 `-125F -15H`; NC-05 found no client evidence for it on ScavDog, registry O5, and moniker 1470900795 is UNRESOLVED, O2) or 1433 `Mob Strike` (set 6, RECONSTRUCTED, effect 1717). Needs: a client record or sequence in `MOB_Carnosaur` that names the attack (ORIGINAL CLIENT VERIFIED), and the owner's choice of damage text (RECONSTRUCTED from the ability text, per D-NC2) | one seed packet: new set with one ability, effect NVPs and the C5 comment, `ability_set_id` on templates 71 and 1559 |
| Horden | 72 / 1558 | creature `MOB_Horden.BS_MOB_Horden`; `MOB_Horden.MOB_Horden00` | organic: ScavDog | as Carnosaur, with the `MOB_Horden` package | one seed packet, as Carnosaur, on templates 72 and 1558 |
| Lenny | 73 / 1556 (name_id 7485); 1557 is a lineup-only baby (`BS_MOB_LennyBaby`, a lineup-only template row with no world template) | creature `MOB_Lenny.BS_MOB_Lenny`; `MOB_Lenny.MOB_Lenny00` | organic: ScavDog | as Carnosaur. `MOB_Lenny.upk` is in the ancient-assets inventory (file presence only; its sequences are UNRESOLVED). The baby has a lineup template row but no world template, so setting its `ability_set_id` is an owner call | one seed packet on 73 and 1556; the baby (1557) is an owner call |
| AMBRat | 74 / 1554 (name_id 7940); hostile test mob | creature `MOB_AMBRat.BS_MOB_Rat`; `MOB_AMBRat.MOB_AMBRat_00` | organic: ScavDog | as Carnosaur. It is the hostile mob the hub-pass UAT uses, so its evidence should come first | one seed packet on 74 and 1554, placed first in the campaign |
| Rhinolion | 75 / 1560 | creature `MOB_Rhinolion.BS_MOB_Rhinolion00`; `MOB_Rhinolion.MOB_Rhinolion00` | organic: ScavDog | as Carnosaur | one seed packet on 75 and 1560 |
| Unas | world 105-110 (male) and 132-137 (female); lineup 1476-1481 and 1517-1522 | Jaffa body `BS_JaffaMale` or `BS_JaffaFemale`, with the `AR_J_Unas.*` armour parts. No `WP-*` component. Client Unas kit: `AR_J_*` (Unas) in the pets content inventory | Jaffa-body reference (templates 159 and 160, registry section; damage payload from NC-12), but Unas carry no weapon, so the staff step does not apply. No ScavDog-style creature match | No candidate ability in the seeds. The pets content inventory records that no Unas pet ability exists. Needs: a melee or ranged attack the Unas use in the client (ORIGINAL CLIENT VERIFIED), then an ability row | an owner decision first (see below); then one seed packet if the owner wants Unas to fight |

## Decisions the owner must make before assignment

1. **Unas.** Give them a set (needs client evidence first), or accept that
   they idle under D-NC1 with a warning. The idle choice needs no packet.
2. **Shared set or one set per creature.** Giving the organic roster ScavDog's
   set 6 (`Mob Strike`) is the quickest path. It would also give six NPCs an
   attack the client has not shown for them, so this spec does not recommend
   it. The default shape is one set per creature.
3. **Lenny's baby (1557).** It has a lineup template row (`ability_set_id` NULL) and no world template, so it can stay as it is (idle) or take Lenny's set.

## Packet shape (applies to each organic seed packet)

- Seed only, with no code change: a new `ability_sets` row
  (`description` carrying `RECONSTRUCTED`), one `ability_set_abilities` row,
  `ability_set_id` on the template and on its lineup row, and the effect NVP
  rows if a new effect is needed. If the chosen ability is 983 Bite, the packet also edits effect 1092 in place: set its trailing `script_name` from NULL to `'MeleePhysicalDamage'` and add HealthDamage 15 and FocusDamage 125 NVP rows (ids from the architect's C3 block, outside 470-479), each RECONSTRUCTED from the effect text `-125F -15H` (D-NC2 pattern, NC-12). Ability 983 reaches event set 300 only through the PR #822 Tier 3 link, which is RECONSTRUCTED. Ability 1433 needs none of this: effect 1717 is done. Each changed row gets the C5 comment. The
  set id comes from the C3 block, assigned by the architect.
- A reference test in `reference/` in the ScavDog shape (NC-13): the
  template picks its own set, the set is non-empty, and the server path
  never reaches 592 (`ability_select.rs:59-69`).
- An evidence registry section per roster NPC in the C6 shape, with the
  client rows (animation package, attack sequence, audio) left as
  UNRESOLVED until the owner's client is read.
- Lane: the standard cell-combat check order. Live-DB only if the packet
  adds a live reference test.

## Not in scope here

- No seed change, no code change and no test in NC-34.
- No lab session (D-NC7 is approved in principle, but each run is asked for
  first, and NC-34 never drives the lab).
- No docs outside this file. The campaign README packet table gets its row
  in NC-51.
