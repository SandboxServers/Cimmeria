# Contributing to Cimmeria

Cimmeria is a server emulator for the cancelled MMO *Stargate Worlds*. It's a hobby project run by a small group of contributors who care about the game and about doing the engineering right. If you'd like to help — thank you — this document covers everything you need to land your first PR.

## What contributing looks like

There is no expectation of pace or volume. People dip in and out as they have time. A first PR that fixes one mission's broken step, adds one missing wire-format test, or polishes one doc page is a *great* first PR. Big sweeping changes are not preferred — small, reviewable, well-tested work is.

The maintainers prioritise:

1. **Correctness over speed.** A change that works on the happy path but breaks under reconnect is a regression, not a feature.
2. **Tests that reproduce the bug shape.** A regression guard that passes when the fix is reverted is theatre, not protection. [`TESTING.md`](TESTING.md) explains the picker and the test-types catalog.
3. **Evidence over confidence.** When you claim "the client expects X," cite the Ghidra address, the `.def` field, or the BigWorld reference source. The tier system in [`docs/guides/evidence-standards.md`](docs/guides/evidence-standards.md) is how we keep speculation out of the docs.
4. **Documentation that matches the code.** A PR that changes user-visible behaviour without updating the corresponding doc will be sent back. See "Required documentation" in [`CLAUDE.md`](CLAUDE.md) for the doc-update map.

## Before you start coding

Read these in order:

1. [`README.md`](README.md) — what the project is and where it stands.
2. [`docs/guides/getting-started.md`](docs/guides/getting-started.md) — get the server running on your machine. You cannot meaningfully contribute without this working.
3. [`CLAUDE.md`](CLAUDE.md) — repo invariants, build memory rules (WSL), the **pre-PR checklist**. CI runs the checklist exactly; if you skip it locally you will round-trip.
4. [`TESTING.md`](TESTING.md) — test types and when to use which. The single biggest source of PR rework is "you wrote a unit test, this needed a live-DB regression guard."
5. The relevant doc for the area you're touching — `docs/protocol/` for wire formats, `docs/gameplay/` for game systems, `docs/architecture/` for cross-cutting concerns, `docs/content/` for content engine work, `docs/operations/` for runtime/deployment.

## Picking something to work on

We track work in GitHub issues at <https://github.com/SandboxServers/Cimmeria/issues>. Useful starting points:

- **Issues labelled `good-first-issue`** — scoped to one file or one system, well-described, won't need RE work to land.
- **Issues labelled `documentation`** — most of these are surface-level fixes that help you learn the codebase without changing runtime behaviour.
- **Issues labelled `verify`** — small RE-flavoured tasks. Confirm a specific wire-format claim, check a single handler, document the evidence.
- **Open content-chain bugs** — broken missions, miswired dialogs, vendor flows that lose state. These usually don't need RE knowledge.

If you'd like to propose something not yet in an issue, please open one first and let a maintainer comment before you start coding. We don't want you to spend a weekend on a change we're going to ask you to redo.

### Approachable areas (no RE required)

These areas mostly need careful Rust work, content authoring, or test additions — not Ghidra:

- **Content-chain authoring** — triggers/conditions/actions in [`crates/content-engine/`](crates/content-engine/). [`docs/content/extending-the-engine.md`](docs/content/extending-the-engine.md) is the entry point. Adding a new mission step or a new trigger type lands without touching wire formats.
- **Mission fixes** — the database has 1,040 mission rows and many have known bugs. See [`docs/content/mission-chains.md`](docs/content/mission-chains.md) and [`docs/content/zone-audit.md`](docs/content/zone-audit.md).
- **Game-system tests** — most subsystems (combat, inventory, vendors, social) have known gaps in test coverage. See [`docs/testing/inventory/`](docs/testing/inventory/) for the per-crate test census.
- **Documentation** — about 90 in-scope docs are missing the metadata blocks the documentation-writer agent expects. [Issue #344](https://github.com/SandboxServers/Cimmeria/issues/344) tracks this sweep.
- **Tauri admin app** — [`tools/`](tools/) and the launcher under [`crates/launcher/`](crates/launcher/). Frontend changes need a REPL-style logic UAT per [`AGENTS.md`](AGENTS.md).

### RE-required areas (Ghidra knowledge needed)

These touch wire formats or client expectations directly. The wrong byte = silent disconnect.

- **Protocol implementations** — anything under [`crates/mercury/`](crates/mercury/) and [`crates/services/src/mercury/`](crates/services/src/mercury/).
- **Wire-format additions** — new message handlers, new property dispatches. See [`docs/protocol/`](docs/protocol/) and [`docs/drafts/spec/`](docs/drafts/spec/) (the bible chapters in progress).
- **New entity types or methods** — `.def` files in [`entities/defs/`](entities/defs/) drive everything; touching them propagates outwards.

For RE work, start with [`docs/guides/re-toolchain-setup.md`](docs/guides/re-toolchain-setup.md) and [`docs/guides/reverse-engineering-with-claude.md`](docs/guides/reverse-engineering-with-claude.md). The `game-archaeology-specialist` agent (configured in `.claude/agents/`) is your friend.

## The development loop

```text
# 1. Branch off main
git checkout -b your-handle/short-description

# 2. Iterate fast with cargo check (1.5s, <2 GB RAM)
cargo check -p cimmeria-services

# 3. Add or update a test that reproduces what you're fixing
#    See TESTING.md for the picker — wrong test type is the #1 PR rework cause

# 4. Run the pre-PR checklist locally (CI runs this exactly)
cargo fmt --all -- --check
cargo clippy --workspace --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher --all-targets -- -D warnings
cargo nextest run --profile=ci --workspace \
  --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher

# 5. If you changed anything touching the database, run live-DB tests:
DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw \
  cargo nextest run --profile=ci-live-db -p cimmeria-services --lib

# 6. Update the doc-update-map entries CLAUDE.md identifies for your change

# 7. Commit and push
git add . && git commit -m "fix(area): short description (closes #NNN)"
git push -u origin your-handle/short-description

# 8. Open the PR
gh pr create
```

See [`CLAUDE.md`](CLAUDE.md) for the full pre-PR checklist, the WSL memory rules (the full link can OOM at ~47 GB without care), and the doc-update map.

## Code style

- **Rust** — `cargo fmt` is the source of truth. The project enforces `-D warnings` on clippy; project-level thresholds for `too_many_arguments` and `type_complexity` live in [`clippy.toml`](clippy.toml). Don't sprinkle `#[allow]` annotations per call site — fix the root cause.
- **Comments** — default to writing none. When the *why* is non-obvious (a hidden constraint, a subtle invariant, a workaround for a specific bug), then a one-liner. The well-named identifier should explain the *what*.
- **File organisation** — files should "do what it says on the tin" — predictable names, split along natural seams. Soft cap 500 lines, hard cap 700 lines, but seams matter more than counts. See [`CLAUDE.md`](CLAUDE.md) → "File organization."
- **Markdown** — `tools/lint-md.sh` (or `tools/lint-md.ps1` on Windows) checks the docs. Currently warn-only in CI but reviewers will nudge.

## Commit and PR conventions

We use Conventional Commits-style prefixes (`feat`, `fix`, `docs`, `chore`, `refactor`, `test`) followed by an optional `(area)` scope. Examples from recent history:

- `fix(cell): generate_threat content action refreshes appearance on first-add (#418)`
- `fix(respawn): full inventory re-init bundle, not just onUpdateItem (#409)`
- `feat(observability): full auth + base + world-entry pipeline instrumentation (#414)`

PR titles follow the same pattern. Keep titles under ~70 chars; put the details in the body.

PR body checklist (the maintainers will look for these):

- **What changed and why** — one paragraph.
- **Test plan** — what you ran, what passed, anything you couldn't test (be honest).
- **Doc updates** — which doc-update-map rows you touched.
- **Related issue** — `closes #NNN` if appropriate.

## Review and merge

- Expect 1–3 review rounds for a non-trivial PR. The reviewers care about correctness and test quality; they will ask "but what if the client lies?" and "does this regression guard actually fail when the fix is reverted?" Both are fair questions.
- We squash-merge to keep `main` linear and the commit messages searchable.
- If a review goes quiet for a week, ping the PR — that's a signal, not an insult.

## Reverse-engineering specifics

If you're producing new findings against the SGW binary:

- Cite the Ghidra address for every claim. The findings under [`docs/reverse-engineering/findings/`](docs/reverse-engineering/findings/) are the template.
- Tag confidence: **HIGH** (decompiled + corroborated), **MEDIUM** (single source), **LOW** (inferred). See [`docs/guides/evidence-standards.md`](docs/guides/evidence-standards.md).
- If your finding overlaps with an in-progress bible chapter under [`docs/drafts/spec/`](docs/drafts/spec/), coordinate before writing — the bible is the canonical reference and we don't want parallel versions of the same chapter.

## Security

If you discover something that looks like a security issue, **do not open a public issue**. See [`SECURITY.md`](SECURITY.md) for the private reporting path.

## Conduct

We expect contributors to be helpful, precise, and kind. The full code of conduct is in [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md). Short version: review the diff, not the person; assume good faith; this is a space for everyone willing to do the work.

## Saying thanks

We're a small group; nice comments on a PR go a long way. If you appreciate someone's work — review, design feedback, test contribution, doc cleanup — say so. The maintainers do this work for fun, and your kindness is part of what keeps it fun.
