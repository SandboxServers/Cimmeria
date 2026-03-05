# Complete Audio Inventory

Comprehensive accounting of every audio file in the Stargate Worlds QA client build — what it is, where it is, whether it's referenced in code, and where the gaps are.

**Client build:** `SGWGame/Content/audio/` + UPK packages
**Total FMOD audio:** 734 MB across 280 `.fev` + 566 `.fsb` = 846 files in 29 categories (+ 1 test)
**Total UPK audio:** ~21 MB across 2 packages with actual audio
**Grand total:** ~755 MB of audio content

---

## Audio Engine Architecture

### FMOD Ex (Not FMOD Studio)

The game uses **FMOD Ex** (2009 era) for all runtime audio. UE3's native `ALAudio` is completely replaced.

| Component | Role |
|---|---|
| `SGWAudioDevice` | Custom UE3 audio device wrapping FMOD Ex |
| `USGWAudioComponent` | Per-actor audio component bridging UE3 → FMOD |
| `FMODAudio.Projects` | Config entry listing all `.fev` projects to load |
| `.fev` files | Event definitions (names, parameters, routing, categories) |
| `.fsb` files | Compressed audio sample banks (FMOD's proprietary format) |

### Naming Conventions (from FMOD event paths)

| Prefix | Meaning | Example |
|---|---|---|
| `m_` | Mono oneshot | `m_col_gen_water_splash1` |
| `ml_` | Mono loop | `ml_prp_gen_stmpip_flow` |
| `s_` | Stereo oneshot | `s_ambOD_agn_calib` |
| `sl_` | Stereo loop | `sl_ambOD_agn_shore_rollLight` |
| `Qm_` | Quiet mono oneshot | `Qm_col_gen_yucca_rustle1` |
| `ambID_` | Indoor ambient | `ambID_fx_tech_hum1` |
| `ambOD_` | Outdoor ambient | `ambOD_fx_bird1_chirp1` |
| `prp_` | Prop/interactive object | `prp_sgc_blastdoor_close` |
| `pfx_` | Particle effect sound | `pfx_gen_torch_burn1` |
| `col_` | Collision sound | `col_gen_water_splash1` |
| `strg_` | Stargate | `strg_gate_chevOpen` |

### How Audio is Triggered

| Trigger Source | Mechanism | Example |
|---|---|---|
| Client animation | AnimNotify → FMOD event | Weapon fire, footsteps, vocalizations |
| Client UI (Lua) | `playSound('path')` | `playSound('ui/cue/submitG')` in Trade.lua |
| Server → Client RPC | `playSequence` with sequence ID | Dialog sequences, mission events |
| Kismet sequences | `SeqAct_PlaySound` nodes | Matinee cutscenes, zone events |
| Server NVP | `SoundBankName` on event sequences | Dialog VO paths (bank names, no audio) |
| Python minigame | FMOD event path strings | `hackMG/play/low` in Livewire.py |
| Weather/critter system | KIS-LogicModules ambient | Bird chirps, weather sounds |
| FMOD adaptive params | Intensity/HealthLow/FocusLow | Combat music transitions |

---

## 1. Zone Ambient Audio

Zone-specific ambient, prop, and particle-effect sounds. Each zone directory contains `.fev` event definitions + `.fsb` sample banks.

### Zones With Dedicated Audio

| Zone | Directory | Size | Files | Events | Confidence |
|---|---|---|---|---|---|
| Tollana | `tollana/` | 24M | 3 fev, 4 fsb | Outdoor ambient (`ambOD_toll_ratscav`, `ambOD_toll_bombard`), particle FX (falling debris, fire, shields), props (comm beam, holo on/off, ion cannon fire, floating lights) | **High** — event names match Tollana assets |
| Agnos | `agnos/` | 19M | 3 fev, 3 fsb | Outdoor ambient (shore waves, calibration), particle FX (water, fire, elemental), props (generators, plasma containers, waterfalls) | **High** |
| Castle | `castle/` | 19M | 2 fev, 5 fsb | Indoor ambient (machine starts, cell doors, alarms, chamber hum, window strike), props (CPU hum, stasis beeps, pool churn, screens, doors) | **High** |
| Lucia | `lucia/` | 9.8M | 2 fev, 2 fsb | Outdoor ambient (dog bark, trash fall, cicadas), particle FX (grill, gas lights, sewer bubbles) | **High** |
| Beta Site | `beta/` | 8.2M | 1 fev, 1 fsb | Outdoor ambient (battle pulse, bomb, alarm, glider flyby, gun, staff, drone) — combat-heavy zone | **High** |
| SGC | `sgc/` | 7.0M | 1 fev, 1 fsb | Props (bell, phone, tape drive, floppy, incoming ring alarm, code9 alarm, telemetry beeps, blast/bulkhead doors, elevator, freight lift) | **High** |
| Hebridan | `hebridan/` | 6.0M | 2 fev, 2 fsb | Particle FX (holographic advertisements — "fast break", "cruise", "floor bang"), props (HMV idle) — commercial/urban zone | **High** |
| Goa'uld (generic) | `goauld/` | 9.9M | 1 fev, 2 fsb | Props (AC units, awning, generators, water pumps, hanging items, atmosphere, wire arcs, monitor beeps, water tower) | **High** — used by Ha'tak, Harset, etc. |
| Human (generic) | `hthuman/` | 5.1M | 2 fev, 2 fsb | Particle FX (naquadah bomb smoke), props (atmosphere, floating board hum, monocar hum) | **High** |
| Asgard | `asgard/` | 2.2M | 2 fev, 2 fsb | Indoor ambient (CPU hum, machine accents), props (beam in/out, console beeps) | **High** |
| Ancient | `ancient/` | 1.1M | 2 fev, 2 fsb | Indoor ambient (power off/on/hum), props (lights off/on) — minimal | **High** |
| Earth Military | `emilitary/` | 3.3M | 1 fev, 1 fsb | Props (no events extracted — likely machinery, vehicles) | **Medium** — `.fev` returned no event strings |

### Zones WITHOUT Dedicated Audio (Gap)

These worlds exist in the server database but have **no dedicated audio directory**. They rely on generic ambient (`genamb/`) and prop (`genprp/`) banks, or have no ambient sound at all.

| World | Type | Notes |
|---|---|---|
| Dakara (5 variants) | Major quest hub | No `dakara/` audio directory |
| Harset (4 variants) | Major Goa'uld hub | Uses `goauld/` generic; no `harset/` specific |
| Omega Site (4 variants) | Military base | No `omega/` audio directory |
| Kheb | Quest zone | No `kheb/` audio directory |
| Naitac | Quest zone | No `naitac/` audio directory |
| Sokars Moon | Quest zone | No `sokar/` audio directory |
| Ihpet Crater (4 variants) | Quest zone | No `ihpet/` audio directory |
| Pertho (3 variants) | Quest zone | No `pertho/` audio directory |
| Meridian | Quest zone | No `meridian/` audio directory |
| Pen-Lai | Quest zone | No `penlai/` audio directory |
| Egypt (2 variants) | Story zone | No `egypt/` audio directory |
| Yotunheim | Quest zone | No `yotunheim/` audio directory |
| Vitrus / Anima Vitrus | Dungeon | No `vitrus/` audio directory |
| Sewer Falls | Sub-zone | No dedicated audio |
| Temple | Sub-zone | No dedicated audio |

**Coverage:** 12 of ~30 unique zone themes have dedicated ambient audio. The remaining ~18 would use generic ambients or be silent.

---

## 2. Generic Ambient Audio

Reusable ambient sounds not tied to a specific zone. Used by zones without dedicated audio and as supplements.

### `genamb/` — 115M, 6 fev + 25 fsb

| Bank | Events | Purpose | Referenced |
|---|---|---|---|
| `ambID_fx` | 15 events: tech hum, vent hum, drips, room hum (Med/Lrg/Sml), hangar hum, reactor hum, ruins creak | Indoor ambient loops — any interior space | **Yes** — placed via Kismet volumes |
| `ambOD_fx` | 19+ events: crickets (4 variants), bird chirps (5 species), jungle day/night (3 each), leaves blowing (3 types), critter ambient | Outdoor ambient loops — flora/fauna | **Yes** — Kismet/LogicModules weather system |
| `amb_fluid` | 4 events: stream flow (S/M/L), thick pool movement | Water feature ambients | **Yes** — placed at water bodies |
| `amb_rain` | 7 events: outdoor rain drops, indoor rain on metal/hard/soft surfaces, rain on car | Rain weather system | **Yes** — weather/critter ambient system |
| `amb_thunder` | 2 events: strange nearby thunder, boomy distant thunder | Storm weather | **Yes** — weather system |
| `amb_wind` | 18 events: desert dusty, forest day/night, whistling, eerie, gentle, indoor large/med space, cold low hum, debris blow | Wind ambients (indoor + outdoor) | **Yes** — weather system |

**Confidence: High** — event names are descriptive, and the weather/critter system in KIS-LogicModules references these categories.

---

## 3. Music System

### `globalMR/` — 75M, 1 fev + 17 fsb (Adaptive Music)

The music system uses FMOD's parameter-driven adaptive music. Four FMOD parameters control real-time transitions:

| Parameter | Range | Purpose |
|---|---|---|
| `Intensity` | 0–1 | Combat intensity (0=exploration, 1=heavy combat) |
| `HealthLow` | 0–1 | Player health danger level |
| `FocusLow` | 0–1 | Player focus/mana danger level |
| `RandSeed` | 0–1 | Randomization for variation |

#### Music Banks by Zone

| Bank File | Size | Zone | Contains |
|---|---|---|---|
| `globalMR_cast1.fsb` | 8.8M | Castle | Zone music layer 1 |
| `globalMR_cast2.fsb` | 6.6M | Castle | Zone music layer 2 |
| `globalMR_cast3.fsb` | 2.0M | Castle | Zone music layer 3 |
| `globalMR_toll1.fsb` | 8.9M | Tollana | Zone music layer 1 |
| `globalMR_toll2.fsb` | 8.9M | Tollana | Zone music layer 2 |
| `globalMR_toll3.fsb` | 8.9M | Tollana | Zone music layer 3 |
| `globalMR_toll4.fsb` | 8.9M | Tollana | Zone music layer 4 |
| `globalMR_sgc1.fsb` | 1.8M | SGC | Zone music |
| `globalMR_beta1.fsb` | 1.6M | Beta Site | Zone music |
| `globalMR_logMain.fsb` | 3.0M | Login screen | Main theme |
| `globalMR_logPrax.fsb` | 3.2M | Login screen | Praxis variant |
| `globalMR_logSGU.fsb` | 3.2M | Login screen | SGU variant |

#### Encounter/Emotional Music Stings

| Bank File | Size | Purpose |
|---|---|---|
| `globalMR_emsB.fsb` | 1.8M | "Blue" emotional layer (melancholy/exploration) |
| `globalMR_emsG.fsb` | 1.8M | "Green" emotional layer (success/positive) |
| `globalMR_emsR.fsb` | 1.6M | "Red" emotional layer (danger/combat) |
| `globalMR_emsSt.fsb` | 1.9M | "Stinger" layer (dramatic hits/transitions) |
| `globalMR_emsY.fsb` | 1.8M | "Yellow" emotional layer (caution/tension) |

#### Music Events (from .fev)

| Event | Purpose |
|---|---|
| `beta1_ent` | Beta Site zone entry music |
| `sgc_milit1`, `sgc_milit2` | SGC military themes |
| `sgc_reveal` | SGC reveal/arrival sting |
| `sgc_welc1` | SGC welcome theme |

**Referenced:** Yes — client binary reads zone properties and activates globalMR events based on world ID. FMOD parameters driven by combat state, health, and focus levels.

**Confidence: High** — bank names directly identify zones; emotional layer naming (B/G/R/St/Y) matches standard adaptive music design.

#### Zones WITHOUT Music (Gap)

| Zone | Has Ambient | Has Music |
|---|---|---|
| Tollana | Yes (24M) | **Yes** (36M, 4 layers) |
| Castle | Yes (19M) | **Yes** (17M, 3 layers) |
| SGC | Yes (7M) | **Yes** (1.8M) |
| Beta Site | Yes (8.2M) | **Yes** (1.6M) |
| Agnos | Yes (19M) | **No** |
| Lucia | Yes (9.8M) | **No** |
| Hebridan | Yes (6M) | **No** |
| Dakara | No | **No** |
| Harset | No | **No** |
| Omega Site | No | **No** |
| All other zones | Varies | **No** |

**Only 4 zones have music. Login screen has 3 theme variants (9.5M total).**

### `genmus/` — 3.3M, 1 fev + 3 fsb (Music Stings)

| Bank | Size | Purpose | Events |
|---|---|---|---|
| `genStings_creepy.fsb` | 1.0M | Creepy/horror music sting | (event names not extractable from .fev) |
| `genStings_nmgoauld.fsb` | 0.5M | Goa'uld "near miss" sting | Context-sensitive combat music |
| `genStings_opcore.fsb` | 1.7M | Operation core / mission-critical sting | Used for high-stakes moments |

**Referenced:** Yes — triggered by combat and mission state changes.
**Confidence: Medium** — bank naming is clear; exact trigger conditions inferred from context.

### `statusMR/` — 3.9M, 2 fev + 2 fsb (Status Stings)

| Event | Bank | Size | Purpose |
|---|---|---|---|
| `sts_lugen_xx_xx` | `levelup_gen.fsb` | 0.9M | Level-up fanfare |
| `sts_msn_MissionStart` | `mission_gen.fsb` | 2.9M | Mission accepted |
| `sts_msn_MissionComplete` | — | — | Mission complete fanfare |
| `sts_msn_Step` | — | — | Mission step complete |
| `sts_msn_ObjComplete` | — | — | Objective complete |
| `sts_msn_ObjUnlocked` | — | — | New objective unlocked |

**Referenced:** Yes — server sends mission state changes; client plays corresponding stings.
**Confidence: High** — event names are self-documenting.

---

## 4. Weapon Audio

The most complete audio system in the game. Every weapon has per-tier audio (5 tiers = 5 quality levels), and alien weapons additionally have per-element variants.

### Tau'ri Weapons — `weapTau/` — 82M, 111 fev + 213 fsb

Each weapon `.fev` contains events: `sing` (fire), `empty` (dry fire), `reload`, `sustain` (auto-fire loop). Some add `chamber`, `attack`/`release` (flamethrower), or `hit`/`hitCrit` (melee).

| Weapon | Subdirectory | Tiers × Variants | Files | Events per weapon |
|---|---|---|---|---|
| Assault Rifle | `ar/` | 5 × 4 (A/B/C/D) | 20 fev, 40 fsb | sing, empty, reload, sustain |
| SMG | `smg/` | 5 × 4 | 20 fev, 40 fsb | sing, empty, reload, sustain |
| LMG | `lmg/` | 5 × 3 (A/B/C) | 15 fev, 30 fsb | sing/attack+release, empty, reload, sustain |
| Pistol | `pistol/` | 5 × 2 (A/B) | 10 fev, 20 fsb | sing, empty, reload |
| Dart Pistol | `pistol/` | 5 × 1 | 5 fev, 10 fsb | sing, empty, reload |
| Rifle | `rifle/` | 5 × 2 (A/B) | 10 fev, 20 fsb | sing, chamber, empty, reload |
| Dart Rifle | `rifle/` | 5 × 1 | 5 fev, 10 fsb | sing, empty, reload |
| Shotgun | `shotgun/` | 5 × 1 | 5 fev, 10 fsb | sing, chamber, empty, reload |
| Flamethrower | `flame/` | 5 × 1 | 5 fev, 10 fsb | attack, release, empty, reload, sustain |
| Grenade Launcher | `exp/` | 5 × 1 | 5 fev, 10 fsb | impact, sing, empty, reload |
| C4 | `exp/` | 1 | 1 fev, 1 fsb | blow |
| M67 Grenade | `exp/` | 1 | 1 fev, 1 fsb | blow, hand pull |
| Mine | `exp/` | 1 | 1 fev, 1 fsb | blow |
| Human Blade | `melee/` | 5 × 1 | 5 fev, 5 fsb | hit, hitCrit, sing |
| Alert Dart | `alertDart/` | ? | fev+fsb | Alert sound |
| Alert XBR | `alertXBR/` | ? | fev+fsb | Alert sound |
| Turret | `turret/` | 1 | 1 fev, 1 fsb | lamp beeps (3), death explode, death |

**Referenced:** Yes — weapon item data drives animation notifies which trigger FMOD events by weapon type + tier.
**Confidence: High** — naming convention is `{WeaponType}_{Tier}{Variant}`.

### Jaffa Weapons — `weapJaf/` — 97M, 75 fev + 150 fsb

7 elemental damage types × 5 tiers for each weapon. Each `.fev` has: `sing`, `empty`, `reload`, `sustain`.

| Weapon | Elements | Tiers | Files | Total Banks |
|---|---|---|---|---|
| Staff Weapon | fire, ice, elec, plasma, sonic, radiation, melee | 5 each | 35 fev, 70 fsb | 35 |
| Zat'nik'tel | fire, ice, elec, plasma, sonic, radiation, melee | 5 each | 35 fev, 70 fsb | 35 |
| Plasma Cannon | plasma only | 5 | 5 fev, 10 fsb | 5 |

**Referenced:** Yes — same weapon/animation system as Tau'ri.
**Confidence: High** — naming: `{weapon}_{element}_{tier}a`.

### Goa'uld Weapons — `weapGoa/` — 68M, 40 fev + 75 fsb

| Weapon | Elements | Tiers | Files |
|---|---|---|---|
| Ribbon Device | fire, ice, elec, plasma, sonic, radiation, melee | 5 each | 35 fev, 70 fsb |
| Ashrak Blade | N/A (melee) | 5 | 5 fev, 5 fsb |

**Referenced:** Yes — same system.
**Confidence: High** — naming: `ribbon_{element}_{tier}a`, `ashrakblade_{tier}a`.

### Asgard Weapons — `weapAsg/` — 12M, 5 fev + 5 fsb

| Weapon | Tiers | Files |
|---|---|---|
| Drone Attack | 5 | 5 fev, 5 fsb |

**Referenced:** Yes — same system.
**Confidence: High** — naming: `drone_attack_{tier}a`.

### Misc Weapons — `weapMisc/` — 968K, 1 fev + 1 fsb

| File | Content |
|---|---|
| `melee_gen.fev` / `.fsb` | Generic melee hit sounds (shared fallback) |

**Referenced:** Likely — fallback for unspecified melee.
**Confidence: Medium**.

### Weapon Audio Gap Analysis

| Missing | Notes |
|---|---|
| Jaffa Zat visual/electric effect | Zat has elemental audio tiers but no iconic "zat discharge" SFX separate from weapon fire |
| Goa'uld Drone Attack | Present in Asgard (`weapAsg/`) but no Goa'uld drone variant |
| Goa'uld Cannon | Present in Jaffa (`weapJaf/cannon/`) but no Goa'uld-specific cannon |
| Ori weapons | No Ori weapon category exists at all |

---

## 5. Abilities Audio

### `abilities/` — 51M, 2 fev + 11 fsb

Two FMOD projects covering all non-weapon ability effects:

#### Human Abilities (`abil_hum.fev`)

| Category | Events | Description |
|---|---|---|
| `/dot/` (Damage Over Time) | 22 events | 11 damage types × 2 variants (oneshot `m_` + loop `ml_`): acid, disease, electric, energy, fire, gravitic, ice, poison, psionic, radiation, wound |
| `/effects/` | 20+ events | Debuffs/buffs: decAccuracy, decAttrib, decCover, decDefense, decEnergy, decFocus, decHealth, decMitigate + more |
| `/explosions/` | Multiple | Ability explosion impacts |
| `/objects/` | Multiple | Summoned object sounds |
| `/state/` | Multiple | State change sounds (entering/exiting) |
| `/statecast/` | Multiple | Casting state sounds |
| `/unique/` | Multiple | Class-specific unique ability sounds |

#### Strae Abilities (`abil_strae.fev`)

| Events | Description |
|---|---|
| 9 beam events: `beam_alleg`, `beam_comm`, `beam_conf`, `beam_creat`, `beam_dest`, `beam_force`, `beam_know`, `beam_life`, `beam_matt` | Asgard Strae class beam abilities by school of knowledge (Allegiance, Communication, Conflict, Creation, Destruction, Force, Knowledge, Life, Matter) |

**Referenced:** Yes — ability system triggers FMOD events on cast/impact.
**Confidence: High** — damage type names match `EDamageType` enum; Strae school names match Asgard discipline design.

---

## 6. Player Vocalizations

### `avatars/` — 5.8M, 1 fev + 4 fsb

| File | Size | Content |
|---|---|---|
| `animSFX_avatar.fev` | 42K | Event definitions for all avatar sounds |
| `animSFX_avatar_cmmn.fsb` | 3.3M | Common animation SFX (shared across all characters) |
| `animSFX_avatar_hm1.fsb` | 1.3M | Human Male voice set 1 |
| `animSFX_avatar_hm2.fsb` | 719K | Human Male voice set 2 |
| `animSFX_avatar_hm3.fsb` | 539K | Human Male voice set 3 |

#### Event Tree

```
/events/          — Animation-driven mechanical sounds
    grengrab        Grenade grab
    grenthrow       Grenade throw
    hitkd_A         Knockdown (attacker)
    hitkd_R         Knockdown (receiver)
    hxm_death       Death animation
    jumpDown        Land
    jumpUp          Jump
    loot_A          Loot (attacker)
    loot_R          Loot (receiver)
    stun            Stun effect

/foleyaccents/    — Weapon handling foley
    1hx_weapdrop    1H weapon drop
    2hx_weapdrop    2H weapon drop
    sxx_weapdrop    Sidearm weapon drop
    PDAidle1/2      PDA idle fidget

/switches/        — Weapon stance transitions
    1HtoRX          1-hand to relaxed
    2HtoRX          2-hand to relaxed
    MEtoRX          Melee to relaxed
    RXto1H          Relaxed to 1-hand
    RXto2H          Relaxed to 2-hand
    RXtoHK          Relaxed to holstered
    RXtoRA          Relaxed to ranged
    PDAarm/PDAaway  PDA equip/unequip
    RAtoRX          Ranged to relaxed

/vocalizations/   — Voiced combat reactions (actual voice samples)
    HM1_death       Human Male set 1 — death cry
    HM1_focus       Human Male set 1 — focus/ability grunt
    HM1_hit         Human Male set 1 — hit reaction
    HM1_hitc        Human Male set 1 — critical hit reaction
    HM2_death       Human Male set 2 — death cry
    HM2_hit         Human Male set 2 — hit reaction
    HM2_hitc        Human Male set 2 — critical hit
    HM3_death       Human Male set 3 — death cry
    HM3_hit         Human Male set 3 — hit reaction
    HM3_hitc        Human Male set 3 — critical hit
```

**Referenced:** Yes — animation notifies on character mesh trigger these events.
**Confidence: High** — event names are self-documenting; naming convention (HM1/HM2/HM3) is clear.

#### Player Vocalization Gaps

| Missing | Expected Pattern | Notes |
|---|---|---|
| Human Female voices | `HF1_*`, `HF2_*`, `HF3_*` | No female voice sets exist. Female characters are silent. |
| Jaffa voices | `JM1_*`, `JF1_*` | Jaffa is a playable race — no voice sets at all |
| Asgard voices | `AM1_*` | Asgard are playable — no voice sets |
| HM1 focus variant for sets 2/3 | `HM2_focus`, `HM3_focus` | Only HM1 has a `focus` event |
| Effort/exertion | `HM*_effort` | No climbing, sprinting, or heavy action grunts |
| Emotes | `HM*_laugh`, `HM*_cheer` | No social/emote vocalizations |

---

## 7. Mob/Creature Audio

### `mobs/` — 3.9M, 2 fev + 2 fsb

| Mob | File | Events | Description |
|---|---|---|---|
| Scavenger | `mob_scav.fev` | `attack` (5 variants), `breathe`, `death`, `hit`, `shake` | 4-legged scavenger creature |
| Drone Flyer | `mob_caDroneFlyer.fev` | `deathA` (3), `deathB` (3), `hover`, `idle`, `hitch` | Flying combat drone |

**Referenced:** Yes — mob animation notifies trigger these.
**Confidence: High**.

#### Mob Audio Gaps

The game has many more mob types than the 2 with audio:

| Missing Mob Type | Notes |
|---|---|
| Jaffa soldiers | Primary enemy faction — use generic humanoid hit/death? |
| Goa'uld NPCs | No voiced combat reactions |
| Ashrak assassins | No stealth/attack sounds |
| Unas | Large creature — no sounds |
| Replicators | Iconic SG-1 enemy — no sounds |
| Kull Warriors | Super soldiers — no sounds |
| Generic wildlife | Only scavenger + drone have audio |

**Only 2 of potentially dozens of mob types have dedicated audio.**

---

## 8. Generic Sound Effects

### `genfx/` — 70M, 2 fev + 14 fsb

Two FMOD projects covering all non-specific environmental effects:

#### Collision Effects (`col_gen.fev`)

| Events | Description |
|---|---|
| `col_gen_water_splash1` + underwater loop | Water collision |
| 10 vegetation types: yucca, softbush, palmsoft, palmhard, longleaves, leafybush, dryreeds, dryleaves, boxwood, bigleaves | Foliage rustle (each has normal `m_` + quiet `Qm_` variants) |

#### Particle/General Effects (`pfx_gen.fev`)

| Events | Description |
|---|---|
| `pfx_gen_fly_hover1`, `pfx_gen_fly_zip1` | Insect sounds |
| `pfx_gen_icon_interact`, `pfx_gen_icon_minigame` | Interaction/minigame prompt icons |
| `pfx_gen_wind_burst1` | Wind burst |
| `pfx_gen_torch_burn1` | Torch fire loop |
| `pfx_gen_flies_buzz` | Fly swarm loop |
| `pfx_gen_fountain_splash1` | Fountain |
| `pfx_gen_loot_glow` | Loot pickup glow |
| `pfx_gen_waterfall_base/top (S/M/L)` | 6 waterfall variants (small/medium/large × base/top) |
| `pfx_toll_*` (8 events) | Tollana-specific: gas jets, ground fire, roof fire, sparks, burn debris, shield hum, static deco |

**Referenced:** Yes — particle systems and collision notifies.
**Confidence: High** — event paths directly describe the sound.

---

## 9. Generic Props & Stargate Audio

### `genprp/` — 15M, 1 fev + 5 fsb

| Category | Events | Description |
|---|---|---|
| **Stargate DHD** | `strg_dhd_center`, `strg_dhd_dial1`–`dial7` | DHD center button + 7 chevron dials |
| **Stargate Gate** | `strg_gate_chevShut`, `chevOpen`, `enter`, `pool`, `gateWH_open`, `gateWH_shut`, `spin_begin`, `spin_stop`, `spin_sustain` | Full gate activation sequence |
| **Steam pipes** | `prp_gen_stmpip_flow` | Industrial prop loop |
| **Industry** | 6 events: large hum, low hum, power hum (2), big whir, hiss whir (2), medium whir | Factory/industrial machinery |
| **Fences** | `prp_gen_fence_hitsoft1`, `hithard1` | Physical fence collision |
| **Electric fence** | `prp_gen_Efence_hum`, `Efence_hitsoft1` | Electrified fence hum + touch |

**Referenced:** Yes — Stargate interaction scripts, prop placement.
**Confidence: High** — Stargate audio is core gameplay.

---

## 10. UI Audio

### `ui/` — 14M, 5 fev + 8 fsb

#### Core UI (`ui.fev`)

| Event | Description |
|---|---|
| `ui_cue_mousent` | Mouse enter hover |
| `ui_cue_navdwn/lft/rgt/up` | Navigation sounds (4 directions) |
| `ui_cue_scenecls/sceneopn` | Scene close/open transitions |
| `ui_cue_submitB/submitG` | Submit bad (error) / Submit good (success) |
| `ui_cue_clicked` | Click confirmation |
| `ui_cue_focused` | Focus gained |
| `ui_cue_indexdec/indexinc` | Index scroll up/down |

**Referenced:** Yes — Lua `playSound('ui/cue/submitG')` in Trade.lua, `playSound('ui/cue/submitB')` in Trade.lua.

#### Minigame UI Audio

| File | Events | Description |
|---|---|---|
| `activateMG.fev` | 22 events: activate/repair/search variants of choose, ready, rotate, timeout, match + generic achieve, burst, cash, pop, start, sweep, xp | Activation minigame (lock picking / repair / search) |
| `crystalMG.fev` | 23 events: telem correct/incorrect/no, time alter/change/stop/exchange, timer short, tools alter/apply, game failure/start/success, move parts, obstacles, playing field | Goa'uld Crystal puzzle minigame (FMOD audio — separate from UPK audio below) |
| `hackMG.fev` | 17 events: obstacle/play/goal/move (high/low/mid each), clock beep, door shut/open, time short, wire snip | Livewire hacking minigame |
| `genMG.fev` | 6 events: Fail1, GoodJob1/2, FoundIt1, SeriesDone1, StepDone1 | Generic minigame results |

**Referenced:** Yes — Python `Livewire.py` references `hackMG/play/low`, `hackMG/move/mid`, etc. directly. Trade.lua calls `playSound()`.
**Confidence: High** — minigame event paths match Python code exactly.

#### Lua `playSound()` Usage in Client

| File | Call | FMOD Path |
|---|---|---|
| `Trade.lua:309` | `playSound('ui/cue/submitG')` | Trade success |
| `Trade.lua:313` | `playSound('ui/cue/submitB')` | Trade failure |
| `Duel.lua:48,57` | `playSound(DuelMod.CountDownSound)` | Duel countdown (path defined elsewhere) |
| `Trade.lua:1` | `-- TODO: playSound('TradeSuccess')` | Commented out — never implemented |

---

## 11. Footstep Audio

### `footsteps/` — 5.4M, 2 fev + 2 fsb

Two weight classes, each with 8 surface types × 2 movement speeds:

| File | Events | Description |
|---|---|---|
| `foot_w1.fev` | 16 events: `w1shoe_run_*` + `w1shoe_walk_*` × 8 surfaces | Weight class 1 (light — human normal) |
| `foot_w2.fev` | 16 events: `w2shoe_run_*` + `w2shoe_walk_*` × 8 surfaces | Weight class 2 (heavy — armored/Jaffa) |

**Surface types:** dirt, gravel, hard (stone/concrete), metal, mud, sand, snow, wood

**Referenced:** Yes — animation notify on foot-down triggers by surface material.
**Confidence: High** — naming convention is clear; 8 surfaces × 2 gaits × 2 weights = 32 events.

#### Footstep Gaps

| Missing | Notes |
|---|---|
| Water wading | No splash footstep variant |
| Carpet/fabric | Indoor soft surface |
| Glass/crystal | Tollana/Ancient interiors |
| Crouch movement | Only walk and run — no crouch variant |

---

## 12. UPK Audio Packages

Most UPK packages in the client use FMOD bridge classes (referencing `.fev`/`.fsb` files). Only **2 packages** contain actual embedded audio samples:

### Packages WITH Embedded Audio

| Package | Size | Location | Format | Content |
|---|---|---|---|---|
| `GoauldCrystalsSounds.upk` | 18M | `Content/Minigames/Sounds/` | OGG Vorbis (UE3 native `SoundNodeWave`) | Goa'uld Crystals minigame — this is the **only** use of UE3 native audio in the entire game |
| `MiniGames.upk` | 2.7M | `Content/Minigames/Textures/` | Mixed (textures + some audio refs) | Minigame textures + audio references |

### Packages WITH FMOD Bridge References (No Embedded Audio)

68 UPK packages contain FMOD bridge class references (`SGWAudioComponent`, `FMOD*` classes) that point to `.fev`/`.fsb` files rather than embedding audio. These include all level packages, character packages, and effects packages.

### Stub/Metadata Packages (No Audio Content)

| Package | Size | Location | Content |
|---|---|---|---|
| `KIS-DialogVO.upk` | 60K | `CookedPC/Packages/` | Kismet VO node metadata — zero audio samples |
| `KD_VOX-Human.upk` | 13K | `CookedPC/Packages/Character/` | VOX package stub — zero audio samples |
| `KismetAudioAssets.upk` | 3.9M | `CookedPC/Packages/` | **Misnamed** — contains VFX/particle references, not audio |

---

## 13. Spoken Dialog VO: Not Present

**There are no recorded dialog voice-over files in this build.** The evidence is conclusive:

### Evidence

1. **No dialog audio in any FMOD bank** — All 846 FMOD files scanned; none contain speech or dialog samples.

2. **Empty VO packages** — `KIS-DialogVO.upk` (60KB) and `KD_VOX-Human.upk` (13KB) contain only Kismet node metadata, zero audio.

3. **No FaceFX lip sync data** — Client binary has full FaceFX runtime (`PlayFaceFXAnim`, `StopFaceFXAnim`, `MountFaceFXAnimSet`, etc.) but no `.fxa` files ship. See [facefx-lip-sync.md](facefx-lip-sync.md).

4. **Dialog schema is text-only** — No audio reference column in `dialog_screens` table. `Dialog.lua` renders text, no audio playback.

5. **CookedDataDialogs.pak is text-only** — 2.1MB of serialized dialog text data, no audio.

### But the Plumbing Exists: 533 SoundBankName References

The database contains **533 `SoundBankName` NVP entries** across **266 unique values** in `sequences_nvp`, pointing to dialog VO bank paths that were **planned but never recorded**:

#### VO Path Structure

```
{zone}/{npc_name}/{mission_prefix}/{DialogTitle}_{ScreenNum}_V{npc_name}
```

Example: `d_tollana/coppleman/MsM/ApartmentHunting_1_Vcaptcopplemann`

#### NPCs with SoundBankName Entries

| NPC | Zone Dialogs | Total Entries |
|---|---|---|
| Copplemann (Captain) | Tollana, Lucia, Agnos, Harset | ~120 |
| Marsh | Tollana, Agnos, Harset | ~80 |
| Lethander | Tollana, Lucia, Harset | ~60 |
| Baal | Tollana, Harset | ~50 |
| Anat | Agnos, Harset | ~40 |
| Mohkatan | Tollana, Harset | ~30 |
| Nerus | Agnos, Harset | ~25 |
| Vala | Lucia | ~15 |
| Castle quest/lore NPCs | Castle | 12 |

#### Zone Distribution

| Zone Prefix | SoundBankName Entries |
|---|---|
| `d_tollana` | 209 (39%) |
| `d_Harset` | 152 (29%) |
| `d_lucia` | 116 (22%) |
| `d_agnos` | 44 (8%) |
| `dialog_castle` | 12 (2%) |

**These represent the planned VO recording scope.** Only 5 of the game's 30+ zones were slated for dialog VO. Tollana was the most complete in terms of dialog planning.

### Kismet VO Nodes (Infrastructure, No Content)

| Node | Purpose |
|---|---|
| `KIS_DialogVO` | Play voice-over during NPC dialog screens |
| `KIS_BarkVO` | Play ambient NPC barks (one-liners near NPCs) |
| `KIS_PlayScreenVO` | Play voice-over during cinematic screens |
| `KIS_VolumeDuckDown/Up` | Duck other audio during VO playback |

### Audio Categories (Binary Strings)

| Category | Purpose |
|---|---|
| `OptionVoice` | Voice volume slider in Options menu |
| `MovieVoice` | Cinematic/cutscene voice volume |

### Voice Chat (BigWorld VOIP)

`ServerConnection::voiceData` (`0x00dd68b0`) — BigWorld real-time VOIP for player-to-player communication. Not pre-recorded dialog.

---

## 14. Event System Audio Pipeline

The server drives audio through the event/sequence system:

| Table | Records | Purpose |
|---|---|---|
| `event_sets` | 675 | Named event groups (missions, interactions, dialogs) |
| `sequences` | 1,973 | Individual playable sequences within event sets |
| `event_sets_sequences` | 1,958 | Links sequences to event sets |
| `sequences_nvp` | 533 (SoundBankName) | Audio bank name-value pairs for VO |

**Flow:** Server triggers `event_set_id` → client receives `playSequence` RPC → client looks up Kismet sequence → Kismet may contain `SeqAct_PlaySound` or `KIS_DialogVO` → plays audio.

For the 533 SoundBankName entries, the chain breaks at the audio file level — the Kismet nodes exist, the NVPs exist, but no actual `.fsb` banks were ever delivered for dialog VO.

---

## 15. Test Audio

### `test_files/` — 2.7M, 1 fev + 2 fsb

| Event | Description |
|---|---|
| `Go Deeper - Verse 05 - Full Mix` | Music test sample |
| `440tone` | 440Hz sine wave (standard audio test tone) |
| `Hand Claps` | Clap test sample |
| `ChannelTest` | Stereo/surround channel test |

**Referenced:** No — development testing only.
**Confidence: High** — clearly debug/test content.

---

## Summary: Complete Status Matrix

| Audio Category | Status | Size | Referenced | Files | Confidence |
|---|---|---|---|---|---|
| **Zone Ambient (12 zones)** | Present | ~132M | Yes (Kismet) | 26 fev, 30 fsb | High |
| **Generic Ambient** | Complete | 115M | Yes (weather/Kismet) | 6 fev, 25 fsb | High |
| **Weapons (Tau'ri)** | Complete | 82M | Yes (anim notify) | 111 fev, 213 fsb | High |
| **Weapons (Jaffa)** | Complete | 97M | Yes (anim notify) | 75 fev, 150 fsb | High |
| **Weapons (Goa'uld)** | Complete | 68M | Yes (anim notify) | 40 fev, 75 fsb | High |
| **Weapons (Asgard)** | Complete | 12M | Yes (anim notify) | 5 fev, 5 fsb | High |
| **Weapons (Misc)** | Present | 968K | Likely | 1 fev, 1 fsb | Medium |
| **Adaptive Music** | Partial (4 zones) | 75M | Yes (FMOD params) | 1 fev, 17 fsb | High |
| **Generic SFX** | Complete | 70M | Yes (collision/PFX) | 2 fev, 14 fsb | High |
| **Abilities** | Complete | 51M | Yes (ability system) | 2 fev, 11 fsb | High |
| **Stargate/Props** | Complete | 15M | Yes (interactions) | 1 fev, 5 fsb | High |
| **UI** | Complete | 14M | Yes (Lua playSound) | 5 fev, 8 fsb | High |
| **Footsteps** | Complete | 5.4M | Yes (anim notify) | 2 fev, 2 fsb | High |
| **Player VO (Male)** | Partial | 2.5M | Yes (anim notify) | — | High |
| **Mob Audio** | Partial (2 mobs) | 3.9M | Yes (anim notify) | 2 fev, 2 fsb | High |
| **Status Stings** | Complete | 3.9M | Yes (mission events) | 2 fev, 2 fsb | High |
| **Music Stings** | Present | 3.3M | Yes (combat state) | 1 fev, 3 fsb | Medium |
| **Minigame (Crystals)** | Complete | 18M | Yes (UPK native) | 1 UPK | High |
| **Minigame (FMOD)** | Complete | — | Yes (Python/Lua) | 3 fev, 3 fsb | High |
| **Test Audio** | Dev only | 2.7M | No | 1 fev, 2 fsb | High |

---

## Gap Analysis: What's Missing

### Critical Gaps (Core gameplay impact)

| Gap | Impact | Scope |
|---|---|---|
| **No dialog VO** | All NPC conversations are silent text. 533 planned VO entries never recorded. | 8 NPCs across 5 zones |
| **No female player voices** | Female characters have zero combat vocalizations | ~50% of player characters |
| **No alien player voices** | Jaffa and Asgard PCs are silent in combat | 2 of 3 playable races |
| **Most mobs are silent** | Only scavenger + drone flyer have sounds | Dozens of mob types missing |

### Moderate Gaps (Immersion impact)

| Gap | Impact | Scope |
|---|---|---|
| **18+ zones without ambient audio** | Major areas like Dakara, Harset, Omega Site are silent | ~60% of zones |
| **26+ zones without music** | Only Tollana, Castle, SGC, Beta Site have zone music | ~87% of zones |
| **No NPC bark VO** | `KIS_BarkVO` node exists but no bark audio | All NPCs |
| **No cinematic VO** | `KIS_PlayScreenVO` node exists but no cinematic audio | All cutscenes |
| **No FaceFX lip sync** | Full runtime present, zero `.fxa` content | All NPCs |

### Minor Gaps (Polish items)

| Gap | Impact |
|---|---|
| No water wading footsteps | Walking in water is silent |
| No crouch footstep variants | Crouch movement uses walk sounds |
| No player effort grunts (climbing, sprinting) | Exertion is silent |
| No emote/social vocalizations | `/cheer`, `/laugh` etc. are silent |
| No Ori weapon audio | No weapon category for Ori faction |
| `emilitary/` .fev has no extractable events | May be corrupt or use different encoding |
| `genmus/genStings.fev` has no extractable events | Same — stings bank may use binary-only metadata |

### What Was Planned vs Delivered

| System | Infrastructure | Content | Status |
|---|---|---|---|
| Dialog VO | Kismet nodes, SoundBankName NVPs, volume ducking, voice options slider | 0 audio files | **Planned, not produced** |
| FaceFX lip sync | Full runtime compiled into binary | 0 `.fxa` files | **Planned, not produced** |
| NPC barks | `KIS_BarkVO` Kismet node | 0 bark audio | **Planned, not produced** |
| Cinematic VO | `KIS_PlayScreenVO` Kismet node | 0 cinematic audio | **Planned, not produced** |
| Female voices | Voice set pattern supports `HF*` prefix | 0 female voice banks | **Planned, not produced** |
| Alien voices | Voice set pattern supports `JM*`/`AM*` prefix | 0 alien voice banks | **Planned, not produced** |
| Full zone music | Adaptive music system supports per-zone layers | 4 zones covered | **Partially produced** |
| Full zone ambient | Audio directory per zone | 12 zones covered | **Partially produced** |
| Full mob audio | Per-mob .fev/.fsb pattern | 2 mobs covered | **Partially produced** |

---

## Identification Methodology

Audio content was identified through:

1. **FMOD `.fev` metadata extraction** — `strings` on each `.fev` file to extract event paths (e.g., `/vocalizations/HM1_death`)
2. **Directory and file naming conventions** — The naming system is highly consistent (see Naming Conventions table above)
3. **Cross-reference with code** — Python minigame scripts, Lua UI scripts, and DB `sequences_nvp` table confirm usage paths
4. **Binary analysis** — Ghidra decompilation of client binary for FMOD bridge classes, Kismet node names, and audio category strings
5. **`.fsb` bank naming** — Bank names directly indicate content (e.g., `globalMR_toll1.fsb` = Tollana music layer 1)

**Not verified by ear** — no audio files were actually played/listened to. All categorization is from metadata, naming, and code references. Confidence levels reflect certainty of identification method.
