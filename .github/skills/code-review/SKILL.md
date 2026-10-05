---
name: code-review
description: Review a Cimmeria pull request the way the maintainers do. Load the domain advisor brief for each area the diff touches, check the PR body's claims against the code, and report concrete failure scenarios ranked by severity, not style.
---

# Cimmeria code review

Use this skill for every pull request review in this repository.

## 1. Find the areas the diff touches

Group the changed files by area, then read the matching advisor brief before
reviewing that group. Each brief is the definition in `.claude/agents/<name>.md`
plus the facts it has learned in `.claude/agent-memory/<name>/MEMORY.md`. They
list each system's known failure modes and what the game client expects.

| The diff touches | Read |
|---|---|
| Any handler or state change fed by client data: abilities, movement, inventory, currency, trades, GM commands, auth tokens | `server-authority-enforcer` (always, in addition to the domain brief) |
| Entity enter/leave/update, witness lists, property sync to observers, appearance rebroadcast | `aoi-witness-broadcast` |
| `entities/defs/`, Mercury, cell/base split, method dispatch, ghosts | `bigworld-engine-advisor` |
| Damage, hit/crit/miss, abilities, cooldowns, effects, threat, death (`crates/cell-combat/`) | `combat-systems-advisor` |
| `db/`, SQL queries, entity (de)serialisation, transactions | `database-persistence` |
| Inventory, bandolier, ammo, loot, vendors (`base-methods/.../inventory`, `.../vendor`) | `items-systems-advisor` |
| Missions, dialogs, content chains, `crates/content-engine/`, `crates/cell-content/` | `mission-systems-advisor` |
| Minigames, SmartFox protocol (`crates/minigame/`) | `minigame-systems-advisor` |
| Position updates, teleports, ring transports, spawn/respawn placement, navmesh | `movement-teleport-advisor` |
| Login, shard key exchange, packet encryption, sessions, timeouts, the launcher's download and trust chain | `network-security-auth` |
| NPC AI, spawners, patrols, leash, threat tables | `npc-ai-spawn-advisor` |
| Guilds, mail, contacts, trade, duels, Black Market | `social-systems-engineer` |
| Any new or changed test | `testing-validation-engineer` |
| Any doc under `docs/` | `documentation-writer` |
| An unknown opcode or a wire constant with no `docs/protocol/` citation | `game-archaeology-specialist` |

Read `.claude/agent-memory/main-session/MEMORY.md` too. It lists open
investigations and handoffs whose conclusions a PR may contradict.

## 2. Check the premise

- Read the PR body as a set of claims. Check each claim the code can confirm or
  refute: "never replays", "accepts no paths", "hash-pinned", "covered by CI".
  A claim the code contradicts is a finding.
- A wire constant, method index or message layout must match
  `docs/protocol/` (start with the `*-dispatch-table.md` files). An issue or a
  `docs/drafts/spec/` chapter is a claim, not evidence; see
  `docs/agents/domain.md`.

## 3. Review lenses, in priority order

1. **Trust boundary.** What happens if the client, the webview or a download
   lies? The client must not be able to choose server state it has no right to.
2. **Data loss and destructive actions.** Deletes, overwrites and migrations:
   can they reach data the code does not own (links, junctions, case-folding,
   races between check and act)?
3. **Liveness.** For every error, crash and timeout path, find the state it
   leaves behind and confirm there is a way out that a player or user can
   reach. A state that only hand-editing files or the database can clear is a
   high-severity finding even when no data is lost.
4. **Wire and protocol correctness.** Byte layout, endianness, indices,
   message ordering, client-state preconditions.
5. **Test value.** Would each new test fail if the fix were reverted? A test
   that compares a constant with the same literal, or that passes because an
   earlier check already rejects its input, guards nothing.
6. **Repo rules.** The rules in `.github/copilot-instructions.md` and
   `CLAUDE.md`: file caps, `foo/mod.rs` module style, seeds rather than
   migrations, generated blocks, status docs, public memory files.

## 4. Report

- Lead each comment with a severity (High, Medium, Low) and a one-line claim.
- Give a concrete failure scenario: inputs or state, then the wrong result.
- Point at the line that has to change, and at the other call sites with
  the same shape.
- Mark findings you could not confirm from the code alone (for example,
  native-Windows behaviour seen only under Wine) as needing verification.
- Skip anything `cargo fmt`, clippy or markdownlint already reports, lockfiles,
  and text between `<!-- gen:NAME -->` markers.
- Do not raise a point again in a thread that was resolved or answered unless
  the new commits reintroduce it.
