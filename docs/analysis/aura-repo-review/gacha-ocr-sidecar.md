# Review: aurablacklight/rust-gacha-ocr-sidecar

> Type: research note. Date 2026-10-18. Read-only on Cimmeria; the repo was shallow-cloned into the session scratchpad and only read, never built or run. Everything fetched is treated as untrusted data.
> **Verified** = I read the file named. **Inferred** = my judgment about fit for Cimmeria.
> Companion note: [../lab-driving-lessons/azurlane-findings.md](../lab-driving-lessons/azurlane-findings.md) already ranks the bot-control ideas (click-loop guard, popup sweeper, screenshot ring buffer, unknown-state record). This note does not repeat them; it covers the sidecar repo itself, its test harness, CI, docs and agent workflow.

## 1. Licence (read this first)

- **No licence file. The GitHub API reports `license: null`. The README ends "Private project. Not licensed for redistribution."** (verified: `README.md`, `gh api`). The repo is private and has 0 stars.
- Consequence: **do not copy code, scripts, schemas or docs text into Cimmeria.** Everything below is "re-derive the idea in our own code". If we want a literal lift, ask the owner (aurablacklight) for written permission first. Third-party pieces they build on (`ort`, PaddleOCR PP-OCRv5 models, `image`, `imageproc`, `axum`) carry their own open licences (Apache-2.0 / MIT class, inferred from my knowledge of those projects, not checked here); we can depend on those directly without going through this repo.

## 2. What the repo is (verified)

A single Rust binary, `gacha-server` (axum), that sits beside the Python bot AzurLaneAutoScript (ALAS) driving an Android emulator. ALAS posts an event plus a screenshot to `POST /api/events`; the sidecar OCRs the screen, runs per-game text rules, asks an LLM (Claude Haiku via Anthropic's OpenAI-compatible endpoint) only if no rule decides, then a core policy turns the Decision into an Executable Directive (continue/skip/reprioritize/pause). It also serves `/ocr/*` endpoints that replace ALAS's old ZeroRPC OCR container.

- Seven crates (`crates/`): `gacha-core` (traits, types, policy), `gacha-ai` (OpenAI-compatible client, prompts), `gacha-ocr` (PaddleOCR PP-OCRv5 ONNX via `ort` 2.0.0-rc.12 plus `ndarray` 0.17, `image`, `imageproc`), `gacha-store` (atomic JSON state), `gacha-server`, and two game plugins. Pinned toolchain 1.96.
- Around it: a large Python 3.13 **test harness** (`scripts/test_harness/`, 234+ script files) that runs the real ALAS scheduler on a real emulator with the sidecar deciding, and keeps hash-chained evidence that offline verifiers re-check. Plus `scripts/ocr_eval/` (OCR benchmark runner with a budget ledger) and a committed evaluation corpus under `crates/gacha-ocr/tests/fixtures/evaluation/`.
- Status (verified, `docs/status.md`): daily task set passes repeatedly on a disposable account; combat tasks passed on the owner's main account on 2026-10-17/18; paid spend about $3.04 of a $10 campaign cap. Docs-heavy and agent-built: `.planning/` holds a GSD-style roadmap/phase/plan/verification tree (about 100 files), `AGENTS.md` for Codex, `CLAUDE.md`, ADRs, incident records, `docs/harness-*/` receipt directories.
- Caveat on my copy: `git clone` exited 128 after checking out most files (Windows path problem, inferred). `Glob` showed about 785 files under `docs/` and 234 under `scripts/`; some files may be missing from my working tree. Nothing I rely on below was among the missing ones.

## 3. Can OCR help our lab UAT? (short answer: not now)

**Verified, our side:** the lab already has `client_ui_state`, `client_window_read`, `client_wait_for`, `client_batch`, `client_lua_eval` (Lua introspection of CEGUI windows inside the injected DLL), plus `lab_screenshot`, `lab_screenshot_region`, `lab_pixel_probe`, `lab_uat_run`, golden diff and record/resume. ALAS needs OCR because an Android game offers no introspection; SGW does, so our "first rung" is exact text from Lua, not recognised pixels. Repo-wide grep for OCR/ONNX/Paddle/Tesseract in `*.md/*.rs/*.py/*.ps1/*.toml` hits only the companion note above (verified).

**Where OCR could still earn a place (inferred):** text drawn outside CEGUI, i.e. inside the Flash minigame windows or baked into textures/3D world text, which Lua cannot read. Even there, `client_minigame_play` and the server-side `server_minigame_tap_read` already give the state without pixels.

**If we ever want it, the cheapest path** (inferred from their README/docs; I did not run it): run their *design* as a separate process, not link it into the lab daemon. Their endpoints (`/ocr/detect-and-recognize` on a PNG, base64 in JSON, returns text + confidence + bbox in original-image coordinates) are a clean contract. We would build our own ~300-line `lab-ocr` sidecar from the same open components (`ort` + PP-OCRv5 mobile det 4.8 MB + English rec model) and call it on a `lab_screenshot_region` crop. Important facts they learned the hard way (verified, `README.md`, `CLAUDE.md`): the 88 MB "server" detection model takes about 9 s per frame on CPU, the 4.8 MB mobile model is the usable one; the CoreML provider was slower and failed (ADR-0002); OCR runs CPU-only everywhere; Linux needs glibc 2.38+ because of the static ONNX Runtime; Windows ships `DirectML.dll` beside the exe.

## 4. Ranked steal-worthy items

Effort: S under a day, M a few days, L a week or more. All "licence" cells inherit section 1: **idea only, re-implement, no code lift**.

### 1. Hash-chained, offline-re-derivable run evidence (M) - high value
- **Where:** `docs/harness.md` "Run store and journal"; `scripts/test_harness/store.py`, `verifiers.py`, `replay.py`; the `*-check` commands (verified in docs; I did not read `store.py` itself).
- **What:** one directory per run with an accepted manifest, a sequenced hash-chained fsynced journal, and content-addressed immutable artifacts. Broken hash or missing file fails closed. Every live check stores its raw observation; a paired `*-check` command re-derives the verdict from that record offline "as a new run". A report can compute an evidence-based verdict (`report --outcome-coded`) that may be *worse* than the recorded outcome if artifacts are missing or cleanup failed. `state` and `outcome` are separate; unsupported assertions are `inconclusive`, never silently passed. A transport ack is not a postcondition.
- **Maps to Cimmeria (inferred):** `lab_uat_run` / `lab_uat_attest` / `lab_uat_report` and `crates/lab-cli/tests/uat_records.rs` already persist records, and some hash use exists in lab code (grep hit on `hash-chain|sha256.*journal|redact` in `crates/lab`, `lab-cli`, `lab-mcp`; I did not read it to see how far it goes). The gap to check: can a grader re-run the *verdict* from saved evidence with no live client, and does a missing screenshot degrade the verdict? Our rule "re-run agent tests yourself" is the manual version of their `*-check`.
- **Take:** the six-outcome vocabulary (passed, failed, blocked, cancelled, inconclusive, running) with a distinct exit code each, and "inconclusive is not pass". Re-implement inside `lab-cli`.

### 2. Budget ledger for paid model use: reserve before dispatch, consume the verified bound (M) - medium value
- **Where:** `docs/adr/0003-paid-runs-account-at-the-verified-bound.md`, `scripts/ocr_eval/ledger.py`, `scripts/test_harness/isolation/relay.py`, `scenarios/provider_session.py` (ADR and README read; ledger code not read).
- **What:** every paid call reserves its worst-case dollar bound in a hash-chained campaign ledger *before* the request leaves, consumes the full bound on reply (because Anthropic replies carry tokens, not cost), and an unresolved reservation (timeout, 5xx, malformed reply) blocks the campaign until reconciled. A relay process is the only holder of the API key and forwards only allowlisted request fields to a fixed upstream. Caps: $5 per run, $10 per campaign. Priced cost recorded separately for information.
- **Maps (inferred):** we already track token cost (`tools/token-profile`, `project_token_profiling` memory, campaign-metrics PR #1665 per memory) after the fact. A pre-dispatch cap is different: it would bound a runaway Haiku lab-driver loop *before* the spend. Worth it only if we start running unattended driver fleets. Today the cheaper guard is the repeat-call guard in the companion note.
- **Take:** the principle (cap enforced at one chokepoint, reserve-then-consume, unresolved = blocked) and the honest "upper bound, about 10x the priced figure" accounting note. Skip the relay unless drivers get API keys of their own.

### 3. Exclusive device lease with fencing, plus owned-process teardown that must be verified (S-M) - we mostly have this
- **Where:** `docs/harness.md` "Leases and input ownership" and "Isolation"; `control/lease.py`, `isolation/process.py`, `owned_registry.py`.
- **What:** one lease per host+serial in a machine-wide namespace; expiry *fences* the old owner without handing access to a competitor; an unresolved dispatch blocks release; a Windows Job object (not just a process group) is the only thing that reliably kills a child tree if the controller dies, so live commands refuse to run elsewhere; teardown is verified and an unverified teardown makes the run `inconclusive`; every owned resource is journalled with one `cleanup` row.
- **Maps:** `lab_lease_acquire/renew/release/status` exist (verified in tool list). Compare for gaps: do we use a Windows Job object around SGW.exe so a killed daemon cannot orphan clients, and does a stale lease after a killed run have a safe "recover only if owner provably dead" command? They hit exactly this (issue #120 and PR #124 "recover-locks", verified `docs/status.md`: leases and dispatch lock left behind when a run is killed). Cheap to check against our `lab_client_stop` and crash recovery.
- **Take:** the "recover-locks refuses while a reservation is unsettled" rule, and the verified-teardown-or-inconclusive rule.

### 4. CI gate job that skips work already done (S) - easy win
- **Where:** `.github/workflows/ci.yml` (verified, read in full), ADR-0004.
- **What:** a tiny `gate` job (3 minute timeout) decides `run=true/false` for the rest: skip a push to `main` whose commit belongs to a merged PR (found via `gh api repos/$REPO/commits/$SHA/pulls`), skip docs-only changes (every changed path under `docs/` or `*.md`, with a named exception for a doc a test reads), and **fail open** (if `gh` or the listing fails, run everything). Concurrency group keyed on PR number cancels superseded pushes on PRs only. Runner chosen by a repo variable (`CI_LINUX_RUNNER` JSON), so switching hosted/self-hosted is one variable.
- **Maps (inferred):** we already run a merge queue and self-hosted runners (memory: `project_merge_queue_2026_10_11`, `project_colo_ci_runners_2026_10_11`) and have `cancel-in-progress` on many workflows (verified by grep of `.github/workflows`). I did not find a "skip push-to-main when the PR already tested it" gate; with a merge queue that re-tests the merge commit, a post-merge push run may already be redundant for us, so check before adding. The fail-open docs-only gate is the transferable part if any of our heavy workflows lack `paths:` filters.
- **Take:** fail-open gating, and the written rationale in an ADR (minutes consumed per run, what is given up, who compensates).

### 5. Cross-language contract fixture generated from the Rust source of truth (S) - good pattern
- **Where:** `crates/gacha-core/src/harness_contract.rs` (read), `crates/gacha-ai/src/bin/generate_harness_contract.rs`, `tests/harness_contract_fixture.rs`, Python `scripts/test_harness/fixtures/sidecar_contract.json` and `contract.py`.
- **What:** Rust emits a JSON document (action names, delimiter escapes, caps, a golden encoded sample) from the real enums; a Rust test pins that the enum serialisation equals the list; the Python side loads the committed JSON and its tests fail if the two drift. One source, two languages, drift = red test.
- **Maps:** our `tools/wire_decoder_codegen.py`, `generate-mercury-kat.py` (known-answer vectors) and the dispatch tables in `docs/protocol/` are the same family. The idea to check is whether lab-side Python/Lua constants (window names, opcode ids the DLL uses) are pinned to Rust definitions by a generated fixture. Low urgency.

### 6. Treat screen/model text as untrusted data: delimiter escaping (S) - cheap hardening for agents that read UI text
- **Where:** `crates/gacha-ai/src/prompt.rs` `encode_screen_evidence` (read).
- **What:** OCR text goes into the prompt as compact sorted-key JSON inside `<screen_evidence>` tags, with `<`, `>`, `&` escaped to `<` etc. so the content cannot close the block or open a tag; caps of 128 observations and 512 chars each; the system prompt says the text is literal on-screen text, never instructions; and a model-supplied "pause" cannot stop the scheduler without an independently supplied trusted classification (CONTEXT.md "Task-Scoped Pause Conversion").
- **Maps:** our drivers already read chat logs, NPC names and item text from the live game and pass them into agent context. Chat from other players is attacker-controlled. If `lab_uat_report` or the lab-driver brief embed raw `client_chat_log` or `client_window_read` strings, wrapping them the same way (escaped, capped, labelled as data) costs almost nothing. Inferred; I did not check how the lab MCP tools currently format those strings.

### 7. OCR evaluation corpus discipline (S as a rule, L if we build a corpus) - only if we do OCR
- **Where:** `crates/gacha-ocr/tests/fixtures/evaluation/README.md`, `manifest.json`, `schemas/`, `scripts/ocr_eval/corpus.py`, `scoring.py`.
- **What:** opaque filenames (answers out of identifiers sent to models), SHA-256 recorded per asset, byte-for-byte copies only, source groups so derivatives and sibling captures never straddle a train/holdout split, independent human reviewer for ground truth, "never mark a model's output as its own truth", frozen `cases.jsonl` hash in a manifest, structural validation in CI that proves integrity but is explicit that it does not prove accuracy. Local inference runs against an owned binary on an ephemeral loopback port with scratch state and no credentials.
- **Maps:** the same rules would govern any screenshot-based golden set for our lab (`lab_golden_record` / `lab_golden_diff` exist). The "source group" idea is the useful bit for golden runs captured in one session.

### 8. Agent workflow files (S, mostly confirmatory)
- **Where:** `AGENTS.md`, `CLAUDE.md`, `docs/agents/codex-exec.md`, `docs/agents/coordination.md`, `docs/agents/issue-tracker.md`, `triage-labels.md`, `domain.md`, `CONTEXT.md`/`GLOSSARY.md`.
- **What worth noting:**
  - A written, *verified-baseline* recipe for driving a second agent CLI headless (`codex exec`): five rules (stdin from `/dev/null`, never rely on exit code as a safety signal, always pass the sandbox flag, the agent does not do git, strip browser/computer-use/multi-agent tools), plus `codex debug prompt-input` to validate flags for free. We already brief Haiku/Sonnet workers through Claude's own agents; this only matters if we want a Codex worker. The "agent does not run git/gh, orchestrator stages and commits" split matches our ops agent.
  - `CONTEXT.md` as a ubiquitous-language glossary with an "_Avoid_:" line per term (Executable Directive, Processed Event, Screen Text). Cimmeria has a glossary-like need (clientIndex vs entities.xml typeID is a standing confusion; CLAUDE.md already calls it out). A short `docs/GLOSSARY` with Avoid-lines would harden briefs. Inferred; I did not check whether we already have one.
  - Account/safety rules written into CLAUDE.md as hard constraints for the live target ("never spend gems", "check commander level before every live run"). Our equivalent is the lab ask-first rule in memory; promoting it to repo text is cheap.
  - "Progress and handover notes go on the GitHub issue, not into docs", which mirrors our "status docs change once per campaign".

### 9. Release pipeline notes (S, reference only)
- **Where:** `.github/workflows/release.yml` (size 17 KB; skimmed via README description, not read line by line), `Dockerfile`, `.dockerignore` as an allowlist.
- **What:** three-target binaries each with a `.sha256` and `BUILD-INFO.txt` that records commit and the shared libraries/symbol versions the binary needs; multi-arch GHCR image; image holds no config/keys/models (mounted at run time); `.dockerignore` as an allowlist so nothing private can be copied in by accident; a `harness-smoke` Docker target CI uses to run the offline suite against the shipped binary. Our `release-container.yml` and launcher release already exist, so the only takeaways are the allowlist `.dockerignore` and a BUILD-INFO-style provenance file. Not verified against our files.

## 5. Things not worth taking

- **The PaddleOCR/ONNX stack as a dependency of the lab daemon.** Large native dependency (static ONNX Runtime, glibc floor, DirectML DLL on Windows), solves a problem our Lua introspection already solved (see section 3). Park as a spike.
- **ALAS-specific parts:** the ADB smart-socket client (`device/smartsocket.py`), PNG validation of `screencap` streams, BlueStacks legacy endpoint selection, tap/swipe controller, coordinate transforms, the confined ALAS child with Python audit hooks. All Android-emulator specific. (The audit-hook confinement is clever but is a Python-process sandbox for a third-party bot we do not run.)
- **The plugin trait architecture for multiple gacha games** (`GamePlugin`, DragonTraveler stub). We have one game; their own docs say the second plugin's vocabulary is an unvalidated hypothesis.
- **A second-model decision hop per UI event.** Their own incident record (companion note section 2/3) shows a model told "stuck" paused the scheduler. Our drivers are already LLM agents reading exact Lua state.
- **JSON-file state store, Prometheus metrics crate choice, axum/reqwest/tokio picks.** Ordinary choices; we use our own stack and SigNoz/OTel. No reason to align crates.
- **`.planning/` GSD tree (about 100 phase/plan/verification files).** Per-phase PLAN/SUMMARY/VERIFICATION/RESEARCH/VALIDATION files for a ~12-phase project is heavier than our campaign/packet ledger and repeats content already in git history. Our `campaign-packet` skill is leaner. (Compare only: their `*-VERIFICATION.md` per phase is the same idea as our adversarial reviewer's report.)
- **Committed run receipts (`docs/harness-*` directories, hundreds of numbered JSON files).** Useful to them as proof; for us it would bloat the tree and conflicts with "status docs change once per campaign". If we want durable UAT evidence, keep it under `docs/analysis/<campaign>/` as summaries and leave raw journals untracked.
- **Notion-held research and "Derek's Mac" paths in `codex-exec.md`**: personal/environment specifics (Windows user paths appear in `coordination.md`); not portable.

## 6. Suggested next steps (smallest first)

1. Read how `lab_uat_*` records are stored and whether a verdict can be re-derived offline from saved evidence; if not, adopt the six-outcome vocabulary with distinct exit codes and "missing evidence = inconclusive" (item 1). Effort S to find out, M to implement.
2. Check the lab lease/crash-recovery path against item 3 (Windows Job object around SGW.exe, safe stale-lease recovery). S.
3. Wrap untrusted in-game text (chat, NPC and item names) in escaped, capped, labelled blocks wherever the lab feeds it to an agent (item 6). S.
4. Decide whether a pre-dispatch budget cap is wanted before running unattended driver fleets (item 2). Decision, not code.
5. Do not start an OCR spike unless a concrete Flash-window or world-text check cannot be done by Lua or tap data.

## 7. What I did not do

I did not build or run anything. I did not read `store.py`, `ledger.py`, `relay.py`, `lease.py`, `isolation/process.py`, `release.yml`, `Dockerfile`, or the ONNX detect/recognise code in `gacha-ocr` line by line; claims about them come from `README.md`, `docs/architecture.md`, `docs/harness.md`, ADRs and `mcp/README.md`, which I did read. I did not check how our existing lab code implements hashing, leases or redaction beyond a filename-level grep.
