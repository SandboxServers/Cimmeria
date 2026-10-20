# MG-72 writer report (docs sweep)

Branch `minigames/mg-72-docs`, reset to `minigames/integration` first. One local commit, nothing pushed.

## Files changed

| File | Change |
|---|---|
| `docs/gameplay/minigame-system.md` | Status and companions; failure chains and `CIMMERIA_MINIGAME_START_DIALOG`; "Testing by hand"; `startMinigame`, `minigameStartCancel`, tech competency rows; "Games at a glance" table; ticket table; result codes 3 and 4; Remaining Work rewritten; Related Docs expanded; blank lines before three rules lists |
| `docs/reverse-engineering/findings/minigame-wire-formats.md` | Metadata and companions; launch flow and result-code table; SmartFox messages per game; implementation notes (URL shape, shell timer) |
| `docs/reverse-engineering/findings/minigame-architecture.md` | Type table rows 5-10 and the 5 games / 5 shells summary corrected; links to the specs |
| `docs/security-audit/2026-05-31-server-authority/findings/CAT-K-minigame.md` | 2026-10-20 status table; a Status line per finding (01 mitigated, 02/05/06/09 open latent, 03/04/07 fixed, 08 partial); Livewire `processreset` open-item note; crate paths updated |
| `docs/analysis/minigames/spec-alignment.md` | D-MG12 corrected to OS entropy for live play; `list_functions.py` locator; section 11 item 7 resolved |
| `docs/analysis/minigames/spec-livewire-parity.md` | Row 16 marked fixed (MG-LW1) |
| `docs/analysis/minigames/packets/wave2-launch.md` | MG-63a snippet uses the merged `check(new_pos, anchor)`, `hold_anchor`, `movement_held` |
| `docs/analysis/minigames/packets/wave3-5-ports.md` | `list_functions.py` invocation corrected (movie first, `-I`) |
| `crates/README.md` | `minigame` crate row: `GameKind` registry, five ports, shells, REPL harness |
| `.claude/agent-memory/minigame-systems-advisor/game-implementation-status.md` | Rewritten head for the shipped state; old content kept as history |
| `README.md`, `docs/readme.md`, `docs/agents/doc-update-map.md`, `docs/testing/inventory/README.md` | Generated blocks only (`regen.py --skip-crate-graph`) |

No status docs touched (`docs/gap-analysis*`, `docs/project-status.md`: MG-99). No new docs, so no index additions.

## Doc-update-map rows touched

Game systems (`docs/gameplay/`), wire and protocol RE findings, security-audit statuses, `crates/README.md`, agent memory, campaign specs.

## Tested vs untested

- Checked: claims verified by reading the merged code and specs in the worktree. `regen.py --check` reports blocks current. `lint-md` reports no errors on lines I added.
- Not run: no cargo build or test, no client, no lab. SmartFox command tables come from the specs, not from a live session.
- Lint residue is pre-existing and untouched: MD032 and MD040 in `CAT-K-minigame.md` bodies, `minigame-architecture.md` lines 30 and 139, MD001 in `minigame-wire-formats.md:18`, MD040 at `minigame-system.md:394`.

## Documentation debt

- Livewire still accepts `processreset`; the other ports refuse it. Parity decision is open (noted in CAT-K and the system doc).
- Item cheats are not ported (Alignment and GoauldCrystals ignore them; Livewire ignores both kinds). The tutorial is unwinnable without cheats.
- `CIMMERIA_MINIGAME_START_DIALOG` defaults off pending lab UAT of the dialog flow (D-MG11), including the mission 582 offer path.
- Social cell methods 20-23, 26-29, 31-34 are still stubs. Result 4 is never emitted by the host.
- CAT-K finding-body line numbers are stale; paths are updated and symbols named.
- Campaign README packet-table statuses are stale (MG-99 owns).
- `minigame-system.md` is about 440 lines, under the 500 soft cap.
- Unverified: per-game refusal-throttle details for ConverseBasicHumanoid, and the wording that `onEndMinigame` follows every result.
