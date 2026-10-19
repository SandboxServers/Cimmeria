# Azur Lane bot findings for lab driving

> Type: research note. Audience: whoever plans the next lab-driver packets.
> Date 2026-10-18. Sources were shallow-cloned and only read, never run.
> **Verified** = I read the code or doc named. **Inferred** = my judgment about fit for our lab.
> Not read: the ALAS asset-extraction tooling (`dev_tools/`), the sidecar's ONNX internals, the fork's `alas.py` diff. The sidecar's OCR interface comes from its README and router only.

## 1. What each repo is

- **LmeSzinc/AzurLaneAutoScript (upstream, ALAS).** A mature Python bot that plays a mobile game from emulator screenshots: template/colour matching, a page graph, a task scheduler. Everything is image-based because the game offers no introspection. It is the pattern library, not a dependency.
- **aurablacklight/AzurLaneAutoScript (fork).** Verified: upstream master merged on top of itself, plus `ai_sidecar/` and `module/ai_hook/hook.py` (a requests-based client that posts events with a base64 screenshot to the sidecar, 60 s debounce on `cycle_start`, `(5, 30)` s timeout, fails open to "continue"). The sidecar README says the real hook lives on branch `recover/ai-hook-2026-07-16` (not read).
- **aurablacklight/rust-gacha-ocr-sidecar.** A Rust axum server. `POST /api/events` takes an ALAS event and returns a directive (continue/skip/reprioritize/pause); `POST /ocr/recognize|detect|detect-and-recognize|analyze` serve PaddleOCR PP-OCRv5 ONNX models. A game plugin runs cheap text rules first, then a Haiku LLM with the screenshot only if no rule decides. Core policy turns a decision into an executable directive. Verified: `router.rs`, `rules.rs`, `plugin.rs`, `CONTEXT.md`, README, two incident/ADR docs.

## 2. Ranked transferable ideas

Context (verified): we already have Lua-level truth (`client_ui_state`, `client_wait_for`, `client_batch`), `lab_ensure_in_world` as a state-machine entry, `lab_finish_dialog`, `lab_uat_run` with spec files, golden runs, chaos, record and watch campaigns, `lab_screenshot_region` and `lab_pixel_probe`. Round 1 of the Haiku UAT ([README](../haiku-lab-uat/README.md)) lost its time to lab gaps and recipe discovery, not to screen recognition. So image ideas rank below control-flow ideas.

### 1. Click-loop and stuck guards as a runner-level contract (S-M)
- **Bot:** `module/device/device.py` `click_record_check` (12 clicks on one button, or 6+6 between two, raises `GameTooManyClickError`) and `stuck_record_check` (60 s of no `handle_control_check`, 180 s for buttons on a long-wait list, raises `GameStuckError`; distinguishes "game died" from "stuck"). `alas.py:run` maps each error to one recovery. Verified.
- **Today:** the lab-driver brief says "a Haiku driver retries the same failing call, or concludes BLOCKED" (haiku-lab-uat README, verified). `client_ui_sequence` and `lab_uat_run` have per-step timeouts, but I found no repeat-click counter (inferred from a grep of `crates/lab/src`; a miss is possible).
- **Proposal:** in the lab daemon, keep a ring of the last ~15 drive calls per lease (tool + target window/entity). On the 4th identical call with an unchanged `client_ui_state` hash, return `repeat_no_effect` with the evidence (state hash, last chat line, server log tail) instead of executing. Drivers then escalate after 3 tries rather than 30. This also bounds cost.

### 2. Interval-gated "appear, then click" instead of blind clicks (S)
- **Bot:** `module/base/base.py` `appear_then_click(button, interval=3)` plus `interval_reset/clear`. A click fires only when the target is seen and not clicked within `interval`. Verified.
- **Today:** `client_ui_click` and `client_ui_sequence` click by name; `wait_window` exists. Whether a click is skipped when the target is absent or already gone is not something I checked.
- **Proposal:** add a `click_if_visible {window, once_per_s}` step kind to `client_ui_sequence` and `client_batch`: Lua visibility check, click, return `clicked|absent|throttled`. Drivers drop the read-then-click round trip (two turns become one).

### 3. Popup sweeper keyed by Lua window names (M)
- **Bot:** `module/ui/ui.py` `ui_additional` (about 130 lines): an ordered list of known popups, each with the one click that clears it (game tips, info dialogs, "mistaken click" pages), tried whenever the current page is unknown. The ordering encodes precedence ("has a popup_confirm variant so must take precedence"). `ui_get_current_page` loops it with a timeout that resets on every handled popup. Verified.
- **Today:** the knowledge is scattered across the lab-driver gotcha sheet and hand-written recipes: close dialogs with `DialogMod.onDialogDoneClicked(DialogWin, DialogWin)`, "Escape does not close Flash windows: read the close button's rect", `lab_finish_dialog` for the intro only (verified in `SKILL.md`, round-1 recipes).
- **Proposal:** a data file `tools/lab/popups.toml` (window name, visibility Lua, dismiss action, precedence) and one tool `lab_clear_popups` that loops `client_ui_state` against the table until no known blocker remains, returning what it closed and any unknown top-level window verbatim. Call it as the first step of every spec row and after any failed click. Grows by appending rows when a driver reports an unknown window. Since we read window names from Lua, we need no image matching.

### 4. Spec rows resume from a named state, not from the start (M)
- **Bot:** `alas.py:get_next_task`/`loop` plus `module/config/config.py:get_next_task`. Every task stores `next_run`; failures reschedule rather than abort; `failure_record` allows 2 retries, the 3rd requests human takeover; a task starts by `ui_ensure(page_x)`, so it is correct from any starting state. Verified.
- **Today:** `lab_ensure_in_world` gives a known start; `lab_uat_run` batches rows on a `run_dir` and `batch.jsonl`; FX campaign adds fixtures (docs/analysis/lab-roadmap, verified). I did not check whether a crash mid-batch resumes at the failed row.
- **Proposal:** make each row declare `precondition` (a Lua predicate plus a fixture) and have the runner assert or re-establish it before steps, never assume the previous row left it. Add `lab uat resume <run_dir>` that skips PASS rows and retries failed ones at most 2 times with the bot's 3-strikes rule. Check FX/SV ledgers first; this may already be covered.

### 5. Screenshot ring buffer dumped on failure (S)
- **Bot:** `device/screenshot.py` keeps a `deque` of the last N frames; `alas.py:save_error_log` writes the last 60 frames plus the log slice since the last section rule to `log/error/<ts>/`. Sensitive-info scrubbing is applied. Verified.
- **Today:** screenshots are taken on demand and the skill says "one before an important click and one after a failure". The failure picture is whatever the driver remembered to take.
- **Proposal:** the daemon keeps the last ~20 low-res frames per instance (cheap: it already captures on demand; capture is the cost, so sample only on drive calls) and `lab_uat_run` writes them to the row's evidence folder on FAIL/NATIVE_SHORTFALL. Evidence for graders without re-driving. Interacts with the screensaver/display preflight; sample only while the client is foreground.

### 6. Structured "unknown state" event with context, and a safe-default policy (S, design only)
- **Bot/sidecar:** `docs/incidents/2026-07-16-commission-pause.md` (verified): an LLM answering `pause` to a screenshot-only event killed the scheduler. Fixes: the event carries task, error text, failure count, click history; model-supplied error text is wrapped as untrusted; a pause on a named task becomes a skip; a failed decision answers "continue" (`CONTEXT.md`, `hook.py`).
- **Today:** we escalate Haiku to Sonnet by hand. Our `lab_uat_report` and driver reports are free text.
- **Proposal:** when a driver reports BLOCKED, require a fixed record (row, step, last 5 calls, `client_ui_state` hash, last server log line, screenshot path) so the Sonnet escalation starts with context. Cheap to add to the `lab-driver` agent brief now.

### 7. Screenshot-to-decision cost ladder: rules, then OCR, then vision LLM (M, defer)
- **Sidecar:** README "How decisions are made": text rules over OCR output decide first; the LLM sees the image only if they cannot (`vision_enabled`). Server-class detection models took about 9 s per frame on CPU; the mobile det model is about 4.8 MB (README, verified); OCR calls took 6-30 ms in the incident doc. ADR-0005: at about 2k prompt tokens per decision, prompt caching saved nothing.
- **Today:** Lua introspection comes first, which is the same ladder with a better first rung. Images matter only for Flash minigames and anything drawn outside CEGUI. `client_minigame_play` and the LiveWire design already read tap data and pixels.
- **Proposal:** do not build OCR now. If a Flash window ever has text that Lua cannot reach, call the existing sidecar's `/ocr/recognize` on a `lab_screenshot_region` crop (HTTP, no Python). Park as a spike, not a packet.

## 3. Not worth adopting

- **Template/colour matching, `Button` assets, offset search (`base/button.py`, `template.py`).** It exists because the bot has no introspection. We name CEGUI windows and read rects from Lua; pixel matching would be strictly less reliable. Keep `lab_pixel_probe` for HUD colours only.
- **Page graph with BFS (`ui/page.py` `init_connection`).** Clean, but our UI is not a page graph: windows overlap and the world is the base state. Replace it with the popup table (idea 3) plus `lab_ensure_in_world`. Revisit only for the login/char-select/world chain, which `lab_ensure_in_world` already covers.
- **Scheduler with `next_run`, server-update windows, hoarding (`config.get_next_task`).** Built for daily-quest cadence. Our runs are one-shot batches. Take only the retry rule (idea 4).
- **Emulator control layer (ADB, scrcpy, nemu_ipc, minicap).** Not applicable to a Windows client.
- **LLM decision sidecar for navigation.** Their own incident shows the failure mode (a model told to treat stuck states as critical). Our drivers are already LLM agents; adding a second model hop adds cost and no new information over `client_ui_state`.
- **ONNX/Paddle stack as a dependency, prompt caching work.** ADR-0005 shows caching only pays above the model's cacheable minimum; our driver prompts are not the cost, turns are.

## 4. Suggested first three packets

1. **LD-01 `repeat_no_effect` guard** (idea 1, S-M): drive-call ring and identical-call detector in the lab daemon, with a test that the 4th identical call with unchanged state is refused. Revert proof: remove the check, test fails.
2. **LD-02 `click_if_visible` step plus structured BLOCKED record** (ideas 2 and 6, S): one new step kind in `client_ui_sequence`/`client_batch`, and the fixed escalation record in `.claude/agents/lab-driver.md`. Docs: live-research-lab guide and lab-uat skill rows.
3. **LD-03 `popups.toml` and `lab_clear_popups`** (idea 3, M): seed the table from the round-1 recipes (dialog close, Flash windows, vendor popup, entity-introduction hold). Leave resume/ring-buffer (ideas 4 and 5) as follow-ups after checking the SV/FX/LR ledgers for overlap.

All three are lab-daemon changes and need no client patch. Any live acceptance run needs the user's OK first.
