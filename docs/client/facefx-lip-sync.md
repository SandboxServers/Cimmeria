# FaceFX Lip Sync System

Analysis of FaceFX middleware integration in the Stargate Worlds client, how lip sync data is authored, and the current state of the content pipeline.

## What is FaceFX

FaceFX is a facial animation middleware by OC3 Entertainment, widely used in UE3-era games (Mass Effect, Oblivion, Fallout 3, Gears of War). It automatically generates facial animation from audio recordings by analyzing speech phonemes and driving skeletal face rigs in real time.

## Authoring Pipeline

### 1. Audio Recording

Voice-over lines are recorded as individual `.wav` files — one per dialog line, bark, or cinematic beat.

### 2. Phoneme Extraction

FaceFX Studio (the desktop authoring tool) analyzes each audio file and automatically extracts **phonemes** — the individual mouth shapes corresponding to each sound in the speech. The tool uses speech recognition to produce a time-aligned phoneme track. Artists can hand-correct the auto-detection when it misidentifies sounds.

### 3. Phoneme-to-Viseme Mapping

Each phoneme maps to a **viseme** (face pose) on the character's skeletal mesh. FaceFX ships with standard English phoneme-to-viseme mappings. Character artists define the bone or morph target positions for each viseme:
- Jaw open/closed
- Lips pursed/spread/rounded
- Tongue position
- Teeth visibility

### 4. Animation Curve Generation

FaceFX generates smooth animation curves that blend between face poses over time, synchronized to the audio waveform. Artists can layer additional curves on top:
- Brow raises, squints, blinks
- Head tilts and nods
- Emotional expressions (anger, sadness, surprise)
- Custom **registers** — arbitrary float parameters the game code can read at runtime (e.g., eye dart intensity, breathing rate)

### 5. Export

The result is a **`.fxa` FaceFX Animation Set** file containing:
- Phoneme timing tracks per animation
- Blended bone/morph animation curves
- References to the source audio files
- Named animation groups (typically one group per character, one animation per dialog line)

### 6. UE3 Integration

In Unreal Engine 3, the `.fxa` is cooked into a `.upk` package and referenced by a `UFaceFXAsset` object. At runtime:
1. The asset is mounted onto a `SkeletalMeshComponent` via `MountFaceFXAnimSet`
2. A Kismet node (`SeqAct_PlayFaceFXAnim`) or code call triggers playback
3. FaceFX drives the facial bones while the associated audio plays through `GetFaceFXAudioComponent`
4. The game can read/write registers via `GetFaceFXRegister` / `SetFaceFXRegister` for dynamic facial behavior

For Matinee cinematics, the `InterpTrackFaceFX` track type synchronizes FaceFX playback with camera cuts, character movement, and other cinematic tracks.

---

## Client Binary Evidence

The SGW client has the **complete FaceFX runtime** compiled in. Functions found via Ghidra:

### Playback Control
| Function | Address | Purpose |
|---|---|---|
| `PlayFaceFXAnim` | (UnrealScript exec) | Start a FaceFX animation on an actor |
| `StopFaceFXAnim` | (UnrealScript exec) | Stop current FaceFX animation |
| `IsPlayingFaceFXAnim` | (UnrealScript exec) | Query if animation is active |
| `PlayActorFaceFXAnim` | `0x0183152c` (string ref) | Actor-level playback |
| `StopActorFaceFXAnim` | `0x0183196c` (string ref) | Actor-level stop |

### Asset Management
| Function | Address | Purpose |
|---|---|---|
| `MountFaceFXAnimSet` | (UnrealScript exec) | Attach animation set to mesh |
| `UnmountFaceFXAnimSet` | (UnrealScript exec) | Detach animation set |
| `GetActorFaceFXAsset` | `0x018310ec` (string ref) | Get actor's FaceFX asset |
| `GetFaceFXAudioComponent` | `0x01831114` (string ref) | Get audio component for VO playback |

### Register System
| Function | Purpose |
|---|---|
| `SetFaceFXRegister` | Write a named float register |
| `SetFaceFXRegisterEx` | Extended register write |
| `GetFaceFXRegister` | Read a named float register |
| `DeclareFaceFXRegister` | Declare a new custom register |

### Kismet / Matinee
| Node | Purpose |
|---|---|
| `SeqAct_PlayFaceFXAnim` | Kismet action: play FaceFX animation |
| `InterpTrackFaceFX` | Matinee track for cinematic facial animation |
| `InterpTrackInstFaceFX` | Matinee track instance data |

### Source Files (from debug strings)
- `.\Src\UnFaceFXAnimSet.cpp` — Animation set loading/management
- `.\Src\UnFaceFXAsset.cpp` — Asset management, reports: `"%d FG Nodes, %d Ref. Bones, %d Anim Groups, %d Anims, FaceFX Actor Name: %s"`

---

## Kismet VO Nodes

The client has Kismet sequence actions specifically designed to trigger dialog voice-over with lip sync:

| Kismet Node | Purpose |
|---|---|
| `KIS_DialogVO` | Play voice-over during NPC dialog screens |
| `KIS_BarkVO` | Play ambient NPC barks (one-liners near NPCs) |
| `KIS_PlayScreenVO` | Play voice-over during cinematic/cutscene screens |
| `KIS_VolumeDuckDown` | Duck other audio channels during VO playback |
| `KIS_VolumeDuckUp` | Restore ducked audio channels after VO |

These nodes would combine `SeqAct_PlayFaceFXAnim` (facial animation) with `PlaySoundEvent` (FMOD audio) to synchronize lip movement with voice playback.

---

## Audio Volume Categories

The options menu exposes voice-specific volume controls:

| Category | Purpose |
|---|---|
| `OptionVoice` | In-game voice volume (dialog VO, barks) |
| `MovieVoice` | Cinematic/cutscene voice volume |

These map to FMOD mixer categories, allowing players to adjust voice volume independently of music and SFX.

---

## Current State: Infrastructure Without Content

The client has full FaceFX support but **no content was ever connected**:

| Asset Type | Status | Notes |
|---|---|---|
| FaceFX Runtime | Compiled into binary | All functions present and functional |
| `.fxa` Animation Sets | **Not present** | No FaceFX data files ship with the client |
| Voice-over audio | **Not present** | No dialog `.wav`/`.fsb` files (see [audio-voice-inventory.md](audio-voice-inventory.md)) |
| Character face rigs | Unknown | Face bones likely exist on character skeletal meshes but unverified |
| Kismet VO nodes | Present as stubs | `KIS_DialogVO`, `KIS_BarkVO` exist but have no audio/animation to play |
| Dialog system | Text-only | `dialog_screens` DB table has text + speaker but no audio reference column |

### What Would Be Needed for Full VO

To add voice acting to the game, the full pipeline would be:

1. **Record dialog** — Voice actors record lines matching `dialog_screens.text` entries
2. **Process in FaceFX Studio** — Import audio, auto-generate phoneme tracks, adjust curves
3. **Export `.fxa` files** — One per character or NPC archetype
4. **Cook into `.upk` packages** — UE3 content cooking pipeline
5. **Add audio column to DB** — `dialog_screens` needs an audio reference (FMOD event or SoundCue path)
6. **Wire Kismet nodes** — Connect `KIS_DialogVO` sequences in each map to the audio + FaceFX assets
7. **Add FMOD event banks** — Create `.fev`/`.fsb` pairs for each character's dialog lines

The infrastructure is ready — the game was clearly designed to have voice acting from the start. This QA build simply predates the content being recorded and integrated.
