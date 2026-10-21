---
name: game-implementation-status
description: Which of the 10 SGW minigames are implemented server-side in Cimmeria, which Python references actually have logic, and which client SWF packages ship
metadata:
  type: project
---

# Minigame implementation status (verified 2026-10-20, MG-72)

## Rust server side: five ports live, five shells

`GameKind` in `crates/minigame/src/minigame/games/kind.rs` is the registry (10 kinds). Validation comes from each module's `PORTED` constant.

- **Server-validated ports** (all `PORTED`): Livewire (always server), CrystalGame, Alignment, GoauldCrystals, ConverseBasicHumanoid. Each in its own dir under `games/` (`livewire`, `crystal_game`, `alignment`, `goauld_crystals`, `converse_humanoid`).
- **Client-declared shells**: Hack, Activate, Analyze, Bypass, Converse via `placeholder::PlaceholderGame`. A client `victory` counts only after 5 s (D-MG13); a 60 s countdown (240 ticks) then `failure`.
- `ability_bits()`: Alignment and GoauldCrystals `0x7FFF`, ConverseBasicHumanoid `0x78000` (Trump bits 15-18), Livewire 0 (cheats deferred, MG-LW1 delta 11), shells 0.
- `processreset`: refused in CrystalGame, Alignment, GoauldCrystals; Converse refuses until a deal ends; **Livewire still accepts it** (open parity item).
- Alignment's live RNG is OS entropy (authority F2); only the REPL and goldens seed from `session.seed`. The other ports seed from `session.seed`.
- Item cheats are not ported anywhere (M8); ability cheats work in Alignment and GoauldCrystals. A client `victory` in GoauldCrystals is WARN `unknown_command`.

## Launch flow and cell methods (MG-21, 22, 24, 62, 63a/b, 64, 74)

- Cell 24 `startMinigame` and 30 `minigameStartCancel` are implemented offer-based (`minigame_launch::accept_offer`, `cancel_offer`); 25 cancels the caller's own session. Flag `CIMMERIA_MINIGAME_START_DIALOG` (default off, 60 s offer) turns on the pre-game dialog; its default stays off until the lab UAT passes.
- Still stubs: debug 20-23, spectate 26/27, helper register 28/29, call accept/decline/abort 31-33, contact request 34.
- Level, intelligence and ability mask come from the player at launch (MG-22); tech competency stays 1 (D-MG18, M9).
- `on_failure_chains` fire on result 2 and 4; 0 and 3 fire nothing (D-MG5). MG-24: an ability effect `StartMinigame` launches for the caster. MG-62: Converse abilities 1327 and 2089. MG-63a/b: the movement hold (`movement_lock::check(new_pos, anchor)`) and the Prison Boot movement-only hold. MG-64: load-time `MINIGAME_TYPE_NAMES` check. MG-74: six Debug Area terminals (`docs/content/debug-area.md#minigame-terminals`).
- Sessions: `PENDING_SESSION_TTL` 180 s, sweep every 60 s, one connection per ticket (`no_free_ticket` at accept, `ticket_already_claimed` at login).
- Docs: `docs/gameplay/minigame-system.md`, `docs/reverse-engineering/findings/minigame-wire-formats.md`, CAT-K statuses, campaign README `docs/analysis/minigames/`.

## Historical note: the Python reference (2026-09-17)

Before the ports landed, only Livewire was real and Alignment and GoauldCrystals were commented-out TODOs that auto-won as placeholders. That is no longer true; keep this only to explain the Python findings below.

## Correction to a common misreading of the Python reference

`deprecated/python/base/minigame/` has 10 files, but they are **NOT** all
`Placeholder` subclasses:

- Real `Atrea.Minigame` subclasses **with game logic**: `Livewire.py`,
  `Alignment.py`, `GoauldCrystals.py`.
- `Placeholder` subclasses (no logic): `Activate.py`, `Analyze.py`,
  `Bypass.py`, `Converse.py`, `Hack.py`.

So Alignment + GoauldCrystals have a Python reference to port from; Converse /
Hack / Bypass / Activate / Analyze genuinely never had server logic — their
rules lived in the client SWF only.

`Placeholder.py` docstring: *"Placeholder handler for minigames that are not
implemented **on the client**"* — and its `message()` accepts only `victory`.

## Client SWFs DO ship for all of them

`docs/client/ui-layout-inventory.md:286,325` — `Flash/*.upk` contains 11
packages: Activate, Alignment, Analyze, Bypass, Converse,
ConverseBasicHumanoid, CrystalGame, DHD, GoauldCrystals, Hack, Livewire.

This contradicted `docs/reverse-engineering/findings/minigame-architecture.md`
("no SWF files exist for them"; corrected in MG-72). The packages exist; the open
question was whether the placeholder ones contain a real game or just a shell with a win
button. **Resolved 2026-09-29:** the Hack/Activate/Analyze/Bypass/Converse
GFX movies each have a `win_btn` that sends `victory` (see
[[livewire-client-swf-facts]]).

Audio evidence that the placeholder games were at least partly built:
`docs/client/audio-voice-inventory.md:515` — `activateMG.fev` has 22 FMOD events
(choose/ready/rotate/timeout/match for activate/repair/**search** variants).
So Activate was the "search a container" game family.

See also [[converse-minigame-evidence]], [[chain-wiring-and-gaps]].

## SWF extension inventory (MG-R6, 2026-10-19, static analysis of all 10 GFX movies)

- Five movies embed a real server-side extension: Livewire, Alignment, CrystalGame, GoauldCrystals, ConverseBasicHumanoid. Activate/Analyze/Bypass/Converse/Hack are ~55 KB shells with a `win_btn` and no rules.
- joinOK vars read by the client: Livewire and Alignment read abilityBitfield, CA0-4, difficulty, instcc, pclevel, seed, techcomp; ConverseBasicHumanoid reads abilityBitfield, intelligence, pclevel, techcomp; the rest read none.
- No client code uses `seed` (only the extension, via java.util.Random). No PRNG port is needed for client compatibility.
- Alignment's client sends `doscore` and `dovictory`: client-reported, must be ignored by a server-authoritative port.
- Full detail: `docs/analysis/minigames/spec-protocol-common.md`.
