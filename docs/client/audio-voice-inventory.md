# Audio & Voice File Inventory

Analysis of all audio and voice content in the Stargate Worlds QA client build.

## Audio System: FMOD

The game uses **FMOD** for all audio (not Unreal Engine 3's native audio system). Audio files come in two formats:
- **`.fev`** — FMOD Event files (metadata: event names, parameters, routing, categories)
- **`.fsb`** — FMOD Sound Banks (compressed audio samples)

FMOD projects are referenced from the client binary via `FMODAudio.Projects`. The custom `SGWAudioDevice` and `USGWAudioComponent` classes (implemented in `SGWAudio.cpp` / `SGWAudioComponent.cpp`) bridge UE3's audio system to FMOD.

---

## Audio Inventory

**Location:** `SGWGame/Content/audio/`
**Total size:** 734 MB across 845 files in 30 categories

### Size Breakdown by Category

| Category | Size | Files (fev/fsb) | Content |
|---|---|---|---|
| **genamb** | 115M | 6/25 | Ambient environments (wind, rain, thunder, fluid, generic) |
| **weapJaf** | 97M | 75/150 | Jaffa weapons (staff, zat, ribbon device — fire/ice/elec/plasma/sonic/radiation, 5 tiers each) |
| **weapTau** | 82M | 111/213 | Tau'ri weapons (AR, SMG, LMG, pistol, rifle, shotgun, flamethrower, grenades, melee — 5 tiers) |
| **globalMR** | 75M | 1/17 | Global music and reverb |
| **genfx** | 70M | 2/14 | Generic sound effects |
| **weapGoa** | 68M | 40/75 | Goa'uld weapons (drone attack, ashrak blade, ribbon elemental variants, cannon — 5 tiers) |
| **abilities** | 51M | 2/11 | Ability activation and impact effects |
| **tollana** | 24M | 3/4 | Tollana zone ambient audio |
| **agnos** | 19M | 3/3 | Agnos zone ambient audio |
| **castle** | 19M | 2/5 | Castle zone ambient audio |
| **genprp** | 15M | 1/5 | Generic prop/interaction sounds (includes Stargate chevron) |
| **ui** | 14M | 5/8 | UI click, hover, notification, menu sounds |
| **weapAsg** | 12M | 5/5 | Asgard weapons |
| **goauld** | 9.9M | 1/2 | Goa'uld environment/theme audio |
| **lucia** | 9.8M | 2/2 | Lucia zone ambient audio |
| **beta** | 8.2M | 1/1 | Beta Site zone ambient audio |
| **sgc** | 7.0M | 1/1 | SGC zone ambient audio |
| **hebridan** | 6.0M | 2/2 | Hebridan zone ambient audio |
| **avatars** | 5.8M | 1/4 | **Player combat vocalizations** (see below) |
| **footsteps** | 5.4M | 2/2 | Footstep sounds (2 surface types) |
| **hthuman** | 5.1M | 2/2 | Human environment/theme audio |
| **statusMR** | 3.9M | 2/2 | Status effect music/reverb |
| **mobs** | 3.9M | 2/2 | Mob sounds (scavenger creature, drone flyer) |
| **genmus** | 3.3M | 1/3 | Generic music stings (level up, mission) |
| **emilitary** | 3.3M | 1/1 | Earth military environment sounds |
| **test_files** | 2.7M | 1/2 | Test audio |
| **asgard** | 2.2M | 2/2 | Asgard environment/theme audio |
| **ancient** | 1.1M | 2/2 | Ancient environment/theme audio |
| **weapMisc** | 968K | 1/1 | Miscellaneous weapon sounds |

### Additional Audio Packages

| File | Size | Location | Content |
|---|---|---|---|
| `KismetAudioAssets.upk` | 3.9M | `CookedPC/Packages/` | Kismet-triggered audio (sequences, matinees) |
| `GoauldCrystalsSounds.upk` | 18M | `Content/Minigames/Sounds/` | Goa'uld Crystals minigame audio |
| `KIS-DialogVO.upk` | 60K | `CookedPC/Packages/` | Dialog VO Kismet metadata (stub — no actual audio) |
| `KD_VOX-Human.upk` | 13K | `CookedPC/Packages/Character/` | Human VOX package (stub — no actual audio) |

---

## Player Vocalizations

The only "voice" content is **combat vocalizations** — short grunts, death cries, and hit reactions.

**Location:** `SGWGame/Content/audio/avatars/`

| File | Size | Content |
|---|---|---|
| `animSFX_avatar.fev` | 42K | FMOD event definitions |
| `animSFX_avatar_cmmn.fsb` | 3.3M | Common animation SFX (weapon draw/holster, jump, loot, grenade, stun, knockdown) |
| `animSFX_avatar_hm1.fsb` | 1.3M | Human Male voice set 1 |
| `animSFX_avatar_hm2.fsb` | 719K | Human Male voice set 2 |
| `animSFX_avatar_hm3.fsb` | 539K | Human Male voice set 3 |

### FMOD Event Tree

```
/events/
    grengrab, grenload, grenthrow       — Grenade animations
    hitkd_A, hitkd_R                    — Knockdown hit (attack/receive)
    hxm_death                           — Death animation
    jumpDown, jumpUp                    — Jump sounds
    loot_A, loot_R                      — Loot animation
    stun                                — Stun effect

/foleyaccents/
    1hx_weapdrop, 2hx_weapdrop          — Weapon drop sounds
    hkx_weapdrop1, sxx_weapdrop         — Holster/sheathe
    PDAidle1, PDAidle2                  — PDA idle sounds

/switches/
    1HtoRX, 2HtoRX, HKtoRX, MEtoRX     — Weapon swap to relaxed
    RXto1H, RXto2H, RXtoHK, RXtoRA     — Relaxed to weapon
    PDAarm, PDAaway, RAtoRX             — PDA and ranged

/vocalizations/
    HM1_death, HM1_focus, HM1_hit, HM1_hitc    — Voice set 1
    HM2_death, HM2_hit, HM2_hitc               — Voice set 2
    HM3_death, HM3_hit, HM3_hitc               — Voice set 3
```

**Notable gaps:**
- No female voice sets (HF1, HF2, etc.)
- No Jaffa, Asgard, or Goa'uld voice sets
- Only `death`, `focus`, `hit`, `hitc` (hit critical?) variants — no idle chatter, taunts, or emotes

### Mob Vocalizations

| Mob | Sounds |
|---|---|
| Scavenger | `attack` (5 variants), `breathe`, `death`, `hit`, `shake` |
| Drone Flyer | `deathA` (3 variants), `deathB` (3 variants), `hover`, `idle`, `hitch` |

---

## Spoken Dialog VO: Not Present

**There are no recorded dialog voice-over files in this build.** Evidence:

### 1. Empty VO Packages

`KIS-DialogVO.upk` (60KB) and `KD_VOX-Human.upk` (13KB) are stub packages. They contain Kismet node metadata but zero audio samples.

### 2. No Dialog Audio in FMOD Banks

All 845 `.fev`/`.fsb` files were scanned — none contain speech, dialog, or NPC voice samples.

### 3. No FaceFX Lip Sync Data

The engine has full FaceFX support (decompiled functions: `PlayFaceFXAnim`, `StopFaceFXAnim`, `GetFaceFXAudioComponent`, `PlayActorFaceFXAnim`, `SetFaceFXRegister`, `MountFaceFXAnimSet`). However, no `.fxa` FaceFX animation files ship with the client.

### 4. Dialog System is Text-Only

The database schema for dialogs:
```sql
CREATE TABLE dialog_screens (
    dialog_id   integer NOT NULL,
    screen_id   integer NOT NULL,
    text        text NOT NULL,       -- Dialog text content
    speaker_id  integer,             -- Who speaks it
    index       integer NOT NULL     -- Screen ordering
);
```

No audio reference column exists. The `Dialog.lua` UI renders text only — no audio playback calls.

### 5. CookedDataDialogs.pak Contains Text Only

`CookedDataDialogs.pak` (2.1MB) contains serialized dialog text data delivered to the client via the cooked data system. It does not contain audio.

---

## VO Infrastructure (Present but Unused)

The client binary has complete infrastructure for voice acting, just no content connected:

### Kismet VO Nodes

| Node | Purpose |
|---|---|
| `KIS_DialogVO` | Play voice-over during NPC dialog screens |
| `KIS_BarkVO` | Play ambient NPC barks (one-liners near NPCs) |
| `KIS_PlayScreenVO` | Play voice-over during cinematic screens |
| `KIS_VolumeDuckDown` / `KIS_VolumeDuckUp` | Duck other audio during VO playback |

### Audio Categories (from binary)

| Category | Purpose |
|---|---|
| `OptionVoice` | Voice volume slider in Options menu |
| `MovieVoice` | Cinematic/cutscene voice volume |
| `FMOD_Cat_Sound Effects` | SFX category |

### Voice Chat (BigWorld VOIP)

The client has a `ServerConnection::voiceData` handler — this is BigWorld's built-in real-time voice chat system, not pre-recorded dialog. It receives voice data packets from the server for player-to-player communication. The server-side handler exists at `ServerConnection_voiceData` (`0x00dd68b0`).

---

## Weapon Audio Detail

The weapon audio system is the most complete, with 5 upgrade tiers per weapon and element/damage type variants for alien weapons:

### Tau'ri Weapons (Earth)
- **Assault Rifles** (AR): 5 tiers × 4 variants (A/B/C/D) = 20 banks
- **SMGs**: 5 tiers × 4 variants = 20 banks
- **LMGs**: 5 tiers × 3 variants = 15 banks
- **Pistols**: 5 tiers × 2 variants = 10 banks
- **Rifles**: 5 tiers × 2 variants = 10 banks
- **Shotguns**: 5 tiers × 1 variant = 5 banks
- **Flamethrowers**: 5 tiers × 1 variant = 5 banks
- **Grenade Launchers**: 5 tiers × 1 variant = 5 banks
- **Explosives**: C4, M67, Mine
- **Dart Guns**: Pistol + Rifle × 5 tiers
- **Melee**: Human blade × 5 tiers
- **Misc**: Alert dart, alert XBR, turret

### Jaffa/Goa'uld Weapons
- **Staff Weapon**: 7 damage types × 5 tiers = 35 banks (fire/ice/elec/plasma/sonic/radiation/melee)
- **Zat'nik'tel**: 7 damage types × 5 tiers = 35 banks
- **Ribbon Device**: 7 damage types × 5 tiers = 35 banks
- **Drone Attack**: 5 tiers
- **Ashrak Blade**: 5 tiers
- **Plasma Cannon**: 5 tiers

### Asgard Weapons
- 5 weapon banks (likely beam weapons, 5 tiers)

---

## Summary

| Audio Type | Status | Size |
|---|---|---|
| Ambient/Environment | Complete (13 zones) | ~210M |
| Weapons (Tau'ri) | Complete (5 tiers, all types) | 82M |
| Weapons (Jaffa) | Complete (7 elements, 5 tiers) | 97M |
| Weapons (Goa'uld) | Complete (5 types, 5 tiers) | 68M |
| Weapons (Asgard) | Present (5 tiers) | 12M |
| Global Music/Reverb | Present | 75M |
| Generic SFX | Present | 70M |
| Abilities | Present | 51M |
| UI | Present | 14M |
| Player Combat VO | Partial (3 male sets only) | 2.5M |
| Mob Sounds | Partial (2 mobs) | 3.9M |
| Footsteps | Present (2 surfaces) | 5.4M |
| Minigame Audio | Present (Goa'uld Crystals) | 18M |
| **NPC Dialog VO** | **Not present** | 0 |
| **Cinematic VO** | **Not present** | 0 |
| **NPC Bark VO** | **Not present** | 0 |
| **FaceFX Lip Sync** | **Not present** | 0 |
| **Female Player VO** | **Not present** | 0 |
| **Alien Player VO** | **Not present** | 0 |
