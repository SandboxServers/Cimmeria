---
name: project-lore-vo-handoff
description: "2026-10-03 Castle lore VO handoff v1: seed claims verified, bank not included, needs a dialog_castle .fev + client patch to play"
metadata:
  type: project
---

Reviewed 2026-10-03, read-only: `SGW_LORE_VO_DEV_HANDOFF_V1.zip` (SHA-256 `c6aa36e220b2445c097b6b49fe276dd1e037e601026b80b6566ecda48b7ae0e2`). It holds six text files: a README, an evidence matrix, a sample manifest, implementation notes, a dev handoff, and `Stage_Lore_Castle_63682.ps1`. **It contains no audio.** The staging script searches the author's own archaeology drive for `lore_castle.fsb`, so we cannot reproduce it.

The handoff claims `lore_castle.fsb` (from builds 43485-62429, with a later distinct hash in 63682) holds ten samples: Coppelman, Marsh, Muelbach, Romney and Zuritzka, each with a `_narr` variant. It says the bank is absent from QA4046 and recommends backporting the 63682 copy as a `RECONSTRUCTION_DECISION`.

Seed claims, verified in this repo:

- EventSet 282 is named `Romney/Zuritzka dialog VO`. Its sequences 283 and 284 have `SoundBankName` `dialog_castle/lore/Romney`, and 289 and 290 have `dialog_castle/lore/zuritzka` (lowercase z). All four run `KIS-DialogVO.KIS_PlayScreenVO`.
- Dialogs 2659 and 2668 use EventSet 282, as the handoff says.
- Moniker 7554 is `DN_Ob_Sc_GoauldDeco_Cellblock_LoreObject` / "Hieroglyph Panel". `dialog_set_maps` row 3018 (set 630, `dialog_id` NULL, flags 1024) is "Analyze the Panel", with tooltip moniker 21841 `DN_Ds_ob_Cellblock_LorePanel_Translate_TT3018`. No spawn or template in the seed uses dialog set 630.

Gaps the handoff does not cover:

- The `SoundBankName` values are FMOD Ex **event paths** (project `dialog_castle`, group `lore`, event `Romney`). An `.fsb` alone cannot resolve them. Playback also needs `dialog_castle.fev`, and an entry for it under `[FMODAudio.Projects]` in `Engine/Config/GameplayEngine.ini`. The stock QA client lists only `ambID_cast` and `prp_cast` for Castle there, and has no dialog VO bank at all (`docs/client/audio-voice-inventory.md` §13). Ask the author whether the recovered builds contain `dialog_castle.fev`.
- The only way to deliver the bank is a client patch (precedent: `data/client-patches/006-gate-sound-bank`). That needs a maintainer decision, which covers redistributing audio from builds other than QA4046.
- The server has nothing to trigger it yet. The content engine's `Action::PlaySound` (`crates/content-engine/src/actions.rs`) is declared but has no wire path. Playing EventSet 282 through `playSequence` is the existing route, and needs no new opcode.

Treat it as a lead, not an import. The archive and `docs/` keep authority, and placement, trigger and collection UI stay `RECONSTRUCTION_DECISION`, as the handoff itself says.
