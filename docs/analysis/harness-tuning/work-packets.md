# harness-tuning: work packets, waves 1 and 2

> **Type:** Reference · **Audience:** The orchestrator, packet-coders and packet-reviewers of this campaign · **Updated:** 2026-10-31 (re-sliced for D-HT3 and D-HT10)
> **Companions:** [Ledger](README.md), [waves 3-5](packets-wave3.md), [resume card](RESUME.md)

## Dispatch rules

- **Campaign** `harness-tuning`. Branch prefix `ht/` (HT-01 registers it in `tools/token-profile/campaigns.json`). Packet worktree `ht-<nn>` on branch `ht/HT-<nn>-<slug>`. Integration worktrees `ht-w1`..`ht-w4` on branches `ht/w1`..`ht/w4`. Create each with `pwsh tools/build-lane/mk-worktree.ps1 <branch> <name>`, one `ops` dispatch per worktree.
- **Chained packets.** `mk-worktree` has no base option. For a packet that "starts from X", `ops` creates the worktree, then merges X's reviewed head into it with `merge-noff.ps1`. Record every packet's **start SHA** in the ledger row. It is both the reviewer's range base and a continuation coder's `base=`.
- **Ledger files were committed by the ledger PR; later edits stay in the main checkout** until the next wave PR or HT-19 commits them: `README.md`, `work-packets.md`, `packets-wave3.md`, `RESUME.md`. Briefs give the spec as the main checkout's absolute path. No packet commits these four files. HT-05 commits only new paths under `baseline/`, and HT-18 only its results file plus one line in the lean-orchestration ledger.
- **packet-coder (Haiku) briefs name no report path** (owner, D-HT3). A coder writes code, commits passing work and returns one line (C4): `<packet> <done|blocked|failed> <commit-sha|-> -`, with optional `reason=<word>`, `at=<k>` and `not_run=<k>,...`. It writes no report, worknote or summary, including on a fix pass. Until HT-07 merges, `packet-coder.md` on `main` still mentions reports and worknotes, so each coder brief adds: "No report file and no worknote. At `Context cap reached`, call no tool and return the one line with `reason=context`."
- **Continuations.** A coder that returns `reason=context`, or returns no usable line, gets a fresh packet-coder with `continuation N of 3, base=<start SHA>` in the brief. It resumes from the branch (C4). At most 3 coder sessions per packet and per fix pass. After that comes one Sonnet implementer (`rust-gameserver-dev` for tools, `documentation-writer` for docs) with the same base and a report path, then an architect re-slice.
- **Reports** are written only by report-writing agents (packet-reviewer, documentation-writer, rust-gameserver-dev, testing-validation-engineer, architect), to `<worktree>\.claude\reports\harness-tuning\<packet>-<role>.md` (role `review-<N>`, `tests-<N>`, `impl`, `ruling-<N>`). Their line is `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`, plus the reviewer's suffixes. Only those agents and the orchestrator write `docs\analysis\harness-tuning\worknotes\<packet>.md`. Before a worktree retires, the orchestrator copies any report or worknote worth keeping into the main checkout.
- **Reviewer briefs** also carry the coder's last return line, so a `not_run=` list reaches `## Not checked` (D-HT8).
- **No cargo in this campaign.** Every check is a `python` command run from the worktree, in the foreground. The lane isn't needed: these checks compile nothing.
- **Agents.** `packet-coder` unless the packet names another. HT-11a, HT-11b and HT-19 go to `documentation-writer`. HT-15a and HT-15c go to `rust-gameserver-dev`, HT-15b to `documentation-writer`, and HT-16 to `architect`.
- **Review.** One packet-reviewer loop per packet (skill § 4). Each wave has one wave loop over its integration worktree, and the campaign has one loop before HT-19. HT-06, HT-07 and HT-08 also get a parallel `testing-validation-engineer` pass on their revert proofs (report `<packet>-tests-<N>.md`).
- **Peer state.** Until HT-08 merges, the orchestrator of this campaign must **not** write `.claude/state/active-campaign`: another campaign's resume card reads that file. Read `RESUME.md` by hand after a compaction. Once HT-08 merges, write the pointer when you start or resume the campaign in a session.

## Waves

| Wave | Packets | Ship | Merge order | Opens when |
|---|---|---|---|---|
| 1 | HT-01..HT-05 | one PR from `ht-w1` | 01, 02, 03, 04, 05 | now (HT-01 Ready) |
| 2 | HT-06..HT-10, HT-11a, HT-11b | one PR from `ht-w2` | 10, 08, 06, 07, 11a, 09, 11b | now; runs alongside wave 1 (no shared files) |
| 3 | HT-12a, 12b, 13, 15a, 15b, 15c | one PR from `ht-w3` | 13, 12a, 12b, 15b, 15c, 15a | wave 2 merged |
| ruling | HT-16 (architect, no code) | none (ledger and packet text only) | n/a | wave 1 merged |
| 4 | HT-14, HT-17, HT-18 | one PR from `ht-w4` | 14, 17, 18 | wave 3 merged; HT-17 needs HT-16 |
| 5 | campaign review, then HT-19 | the review's fixes ship as one PR, then HT-19's PR | n/a | wave 4 merged and the re-measure window full (README § Targets) |

Inside a wave:

- **Wave 1.** HT-01 first. HT-02, HT-03 and HT-04 then run in parallel, each starting from HT-01's reviewed head. HT-05 starts from `ht-w1` once 01..04 are merged into it.
- **Wave 2.** HT-06, HT-07, HT-08 and HT-10 run in parallel from `main`. Once 10, 08, 06 and 07 are merged into `ht-w2`, HT-09 (agent files and a test) and HT-11a (the skill and docs) start from it in parallel. They share no file. HT-11b starts from `ht-w2` after 11a and 09 are merged. Seven packets: HT-11a and HT-11b are documentation-writer passes, so the wave reviewer holds five coder packets plus two doc diffs.
- **Waves 3-5.** See [packets-wave3.md](packets-wave3.md).

Shared files in wave 2:

- `test_hooks.py`: HT-06 edits `WiringTest` and HT-08 edits `ResumeCardTest`.
- `orchestrator.md`: HT-07 edits three paragraphs (`- Coder loop`, `Every brief carries:`, `- On a one-line return`) and HT-08 edits `## State on disk`; HT-09 edits `## Waiting` after both.
- `campaign-packet/SKILL.md`: HT-11a, then HT-11b.

The wave reviewer runs `git merge-tree` on the HT-06/HT-08 and HT-07/HT-08 pairs.

## Contract

### C1. Shared definitions: `tools/token-profile/metrics/transcripts.py` (HT-01)

```python
RP = re.compile(r"[^\s`'\"()<>]*\.claude[\\/]reports[\\/][^\\/\s`'\"()<>]+[\\/][^\\/\s`'\"()<>]+\.md")
LINE = re.compile(r"^(?P<id>\S+)\s+(?P<status>done|blocked|failed)\s+(?P<sha>[0-9a-f]{7,40}|-)\s+"
                  r"(?P<report>\S+\.md|-)(?P<extra>(?:\s+[a-z_]+=\S+)*)\s*$")
SHORT_CHARS = 200
CAP_MARKER = "Context cap reached"      # the user-level Haiku cap's deny text (D-HT3); never changed by this campaign
WRITE_VERBS = ("WriteAllText", "Set-Content", "Out-File", "Add-Content", ">")

@dataclass
class Request:      # one API request; each field is the max over the streamed lines with this message.id
    message_id: str; input: int; cache_read: int; cache_create: int; output: int
    # property context -> input + cache_read + cache_create
@dataclass
class ToolUse:
    id: str; name: str; input: dict; request_index: int
@dataclass
class Dispatch:
    session_id: str; agent_id: str; agent_type: str | None; first_ts: str | None
    brief: str; final_text: str
    requests: list[Request]; tool_uses: list[ToolUse]; results: dict[str, str]   # tool_use_id -> result text

def records(path) -> Iterator[dict]          # parsed JSONL lines; bad lines skipped; utf-8, errors="replace"
def text_of(content) -> str                  # a str, or the joined "text" fields of a list of blocks
def norm_path(p: str) -> str                 # strip quotes, "\\" -> "/", lowercase
def report_suffix(p: str) -> str             # norm_path(p) from ".claude/reports/" on; "" if absent
def read_dispatch(path, session_id, agent_id) -> Dispatch
def iter_dispatches(projects_root, prefix, since=None, until=None) -> Iterator[Dispatch]
def iter_main_transcripts(projects_root, prefix) -> Iterator[tuple[str, Path]]   # (session_id, path)
def brief_report_paths(brief: str) -> list[str]                                # RP matches, in order
def return_line(final_text: str) -> tuple[str, re.Match | None]
def is_context_stop(d: Dispatch) -> bool
def report_written(d: Dispatch, paths: list[str]) -> bool
def cap_denied(d: Dispatch) -> list[ToolUse]                                   # tool uses whose result holds CAP_MARKER
def report_writer_types(repo) -> set[str]   # agent names whose .claude/agents/<name>.md has "## Reporting" and a non-haiku model:
```

Rules:

- **Agent type.** `agentType` in the `.meta.json` beside the transcript (`agent-<id>.meta.json`).
- **`brief`.** The text of the first `user` record whose content holds text, not only `tool_result` blocks.
- **`final_text`.** The text blocks of the last `assistant` record that has any text.
- **`first_ts`.** The first record's `timestamp`.
- **Window.** `since <= first_ts < until`, compared as strings. Dates are `YYYY-MM-DD`.
- **`iter_dispatches`** uses `ingest.store.discover` and keeps the rows of kind `subagent`.
- **`return_line`.** Strip each line. When a line starts and ends with a backtick, strip one backtick from each end. Then:
  - `strict`: the whole stripped text is one line and it matches.
  - `line_first`: the first non-empty line matches and more lines follow.
  - `buried`: a later line matches.
  - `prose`: no line matches and the text is at least `SHORT_CHARS` long.
  - `short`: no line matches and the text is non-empty but shorter.
  - `empty`: there is no text.

  A **usable** line is `strict`, `line_first` or `buried`.
- **`is_context_stop`.** The status is `blocked` (from LINE; without a line, the word `blocked` in the text) and either `reason=context` is in `extra` or "context" appears in `final_text` (case-insensitive); or `cap_denied(d)` is non-empty.
- **`report_written`.** True when either of these holds:
  - a `Write`, `Edit` or `MultiEdit` has a `report_suffix(file_path)` equal to the `report_suffix` of a brief path;
  - a `PowerShell` or `Bash` command contains a `WRITE_VERBS` entry and the file name of a brief path.
- **`report_writer_types`.** Reads `<repo>/.claude/agents/*.md`. The frontmatter is the block between the first two `---` lines; `model:` is its value, lowercased. A type counts when the body holds `## Reporting` and the model doesn't contain `haiku`. Agent types with no file (built-ins, retired agents such as `packet-review-fixer`) never count.

### C2. Metrics subcommands (HT-01 wires them; HT-02/03/04 fill them in)

```
python tools/token-profile/metrics returns|coder-context|resume-cards
    [--projects-root DIR] [--repo DIR] [--since YYYY-MM-DD] [--until YYYY-MM-DD] --out DIR
```

- `--projects-root` defaults to `ingest.store.default_projects_root()`. `--repo` defaults to the current directory. When that path lies inside `.claude/worktrees/<x>`, use the checkout above `.claude`. The prefix is `ingest.store.project_prefix(repo)`.
- The command writes `<out>/<name>.json` (`returns.json`, `coder-context.json`, `resume-cards.json`) as `{"schema": "cimmeria-harness-<name>/1", "window": {"since": ..., "until": ...}, ...}`. The output holds counts, shares and medians only: no text, paths, session ids or campaign names. Before writing, the JSON text goes through `report.scrub.Scrubber().assert_clean(text, "<name>.json")`. On a `PrivacyError`, the command prints `status=error privacy` to stderr, writes nothing, and exits 3.
- stdout gets one line, `status=ok <name> <key>=<value> ...`. Exit 0 on success, 2 on a usage or IO error.
- `metrics/harness_cli.py`: `NAMES = ("returns", "coder-context", "resume-cards")`, `add_parsers(sub)`, and `dispatch(args) -> int | None`, which returns None when `args.cmd` isn't one of the three. Each module exposes `run(args) -> int`.

`returns.json` holds `groups`: one entry per `agent_type`, plus `all`, `all_but_ops` and `report_writers` (the contract dispatches whose type is in `report_writer_types(repo)`). Each entry has these integer fields:

```
dispatches contract_dispatches strict line_first buried prose short empty no_usable_line no_report_write
context_stops cap_denials continuation_dispatches redispatches reviewer_lines not_assessed_suffix
tokens_total tokens_fresh no_usable_tokens_total no_usable_tokens_fresh no_report_tokens_total no_report_tokens_fresh
```

It also holds `coder_lines`, over **every** packet-coder dispatch, with or without a report path (after HT-07, coder briefs name none):

```
dispatches capped capped_usable_line uncapped no_usable_line_uncapped dash_report_lines not_run_suffix
```

`capped` means `cap_denied(d)` is non-empty. `dash_report_lines` counts usable lines whose report group is `-`.

`coder-context.json` covers packet-coder dispatches only. It has groups `all` and `stopped` (a context stop or any cap denial). Each group has:

```
coders stops stop_rate cap_denied
result_chars_share {Read, Grep, Glob, shell-read, shell-git, lane-check, lane-clippy, lane-test, lane-fmt, lane-other, shell-other, Edit/Write, other}
read_targets_share {docs/analysis, docs-other, crates, tools, .claude, other}
median {first_context, last_context, output_tokens, requests, result_tokens, lane_calls, reads, greps, edits}
```

It also holds `continuations` and `continuation_tokens_fresh` (the briefs matching `(?i)continuation \d of 3`, and the sum of their fresh tokens) and the median fresh tokens per continuation.

`resume-cards.json`:

```
compactions firings matched wrong mixed undetermined read_resume_before_dispatch compactions_followed_by_dispatch
by_tier {active: {firings matched wrong mixed undetermined}, inferred: {...}}
```

### C3. `report_contract.py` (HT-06)

This is a SubagentStop hook. It reads these input keys: `stop_hook_active`, `agent_type`, `agent_id`, `agent_transcript_path`, `transcript_path`, `last_assistant_message` and `cwd`. The project root is `CLAUDE_PROJECT_DIR`, or else `cwd`.

1. If `stop_hook_active` is true, exit 0 with no output.
2. If `.claude/agents/<agent_type>.md` doesn't exist or doesn't contain `## Reporting`, exit 0. That covers ops, lab-driver, hands-on and the built-in agents.
3. **Mode.** `line` when the frontmatter `model:` value contains `haiku` (case-insensitive; today only packet-coder), else `report`.
4. Find the transcript: `agent_transcript_path`, or else `<transcript_path without .jsonl>/subagents/agent-<agent_id>.jsonl`. `final` is `last_assistant_message`, or else the last assistant text in the transcript. If there is neither a transcript nor `final`, exit 0.
5. **The cap.** If the transcript exists and any of its lines contains `Context cap reached`, exit 0. The user-level cap has denied a tool, and the hook never asks for more (D-HT3).
6. **Line mode.** If a line of `final` matches LINE (strip one pair of backticks first), exit 0. Otherwise block with reason L. Line mode never reads the brief and never checks a file.
7. **Report mode.**
   - Take the brief (the first user text) and `paths = RP.findall(brief)`. If there is no transcript or no path, exit 0.
   - Pick the **candidate**: the last brief path whose file is missing, empty, or older than the run; if no path is like that, the last brief path. Resolve a relative path against `cwd`, then against the root. "Older than the run" means an mtime earlier than the transcript's first `timestamp` minus 5 s.
   - Check, in order, and block on the first failure:
     - **A**: no line of `final` matches LINE;
     - **B**: the line's report has a `report_suffix` that matches no brief path (a `-` matches none);
     - **C**: the matched brief path's file is missing, empty or older than the run.
8. Otherwise exit 0.

To block, print `{"decision": "block", "reason": <text>}` to stdout and exit 0. Any exception exits 0 with no output (fail open). The reasons, with `<path>` the candidate as the brief wrote it:

- L: `report_contract: end your reply with exactly one line: <id> <done|blocked|failed> <commit-sha|-> - (add reason=<word> when blocked or failed). Write no file and call no tool.`
- A: `report_contract: your final message has no return line. Write your report to <path> if you have not, then end with exactly one line: <id> <done|blocked|failed> <commit-sha|-> <path>. Call no other tool after that.`
- B: `report_contract: your return line names <got>, but the brief's report path is <path>. Write the report there and end with exactly one line naming it. Call no other tool after that.`
- C: `report_contract: <path> is missing, empty or older than this run. Write your full report there, then end with exactly one line: <id> <done|blocked|failed> <commit-sha|-> <path>. Call no other tool after that.`

`LINE` and `RP` are copied verbatim from C1. HT-09's test checks that the two copies match.

### C4. The Haiku coder contract (HT-07; D-HT3, D-HT14)

- **The line.** `<packet> <done|blocked|failed> <commit-sha|-> -`, then optional suffixes, each once, in this order:
  - `reason=<context|checks|spec|host|platform|tool>` on `blocked` or `failed`;
  - `at=<k>`, the packet check or step number where it stopped;
  - `not_run=<k>,...`, the packet checks (by number) it did not run. When it's absent, every check ran.

  The SHA is the branch's last commit, or `-` when the session committed nothing.
- **No report.** No report file, worknote or summary, even when a brief names a report path. The commits are the coder's record, and the reviewer's report is the packet's record.
- **Commits.** Commit at the end with the brief's message. A finished part may be committed earlier, with the subject suffix `(part N)`, but only after that part's checks pass; never commit a failing state. A deviation from the spec is a commit-body line `Deviation: <what> - <why>`. On a fix pass, a rejected note is a commit-body line `Rejected <note-id>: <reason>`. A fix pass that rejects every note commits with `--allow-empty`, so the reasons reach the reviewer.
- **The cap.** Past 90k, the user-level hook denies every tool with `Context cap reached`. Its text asks for a "final report". For a packet-coder, that report is exactly the line `<packet> blocked <sha|-> - reason=context`. Uncommitted edits stay in the worktree.
- **A continuation** (`continuation N of 3, base=<sha>` in the brief) runs `git -C <wt> log --oneline <sha>..HEAD`, `git -C <wt> status --porcelain` and `git -C <wt> diff --stat`, compares them with the spec's steps, and starts at the first step not done. It keeps uncommitted edits that match the spec, never redoes committed work, runs every check and commits.
- **The orchestrator** routes a coder's `blocked` or `failed` by `reason=`:
  - `context`, or no usable line at all → a continuation;
  - `checks` → the Sonnet implementer;
  - `spec` → `architect`;
  - `host`, `platform` or `tool` → the packet's host note.

### C5. Per-session resume card (HT-08)

- Input: `transcript_path` from stdin. When it is missing, or the file is, there's no card. The global pointer file's **contents are no longer read**.
- **Tier 1.** Take this transcript's last `Write`, `Edit` or `MultiEdit` whose `norm_path(file_path)` ends with `/.claude/state/active-campaign`. The text is `content` for Write and `new_string` for Edit; strip a BOM and take the first non-empty line. When that text is empty, there is **no card** and tier 2 is skipped. Otherwise apply today's name check (not `""`, `.` or `..`, no `/` or `\`) and require `docs/analysis/<name>/RESUME.md`; the card starts with today's `POINTER`.
- **Tier 2** applies only when the session never wrote the pointer. Take the last 40 `Agent` tool uses. For each, collect the names in `json.dumps(input).lower()` that match `docs[\\/]+analysis[\\/]+([a-z0-9][a-z0-9.-]*[a-z0-9])` or `\.claude[\\/]+reports[\\/]+([a-z0-9][a-z0-9.-]*[a-z0-9])`, keeping only names that have a `RESUME.md`. That gives a set per prompt. `naming` is the number of prompts with at least one name, and `top` is the name in the most sets, with count `c`. A card is printed when `naming >= 3` and `3*c >= 2*naming`. It starts with `Context was compacted. Inferred campaign (from this session's dispatches): {name}. Read docs/analysis/{name}/RESUME.md and the ledger README before acting; if this is not your campaign, ignore this card.`
- Both tiers append the first 40 lines of RESUME.md, as today.
- **Reading.** Read only lines that contain `active-campaign` (tier 1) or `"Agent"` (tier 2). For a file over 200 MB, seek to its last 200 MB and drop the partial first line.

### C6. Not checked (HT-09)

- **Reviewer report.** A fourth section after `## Not fixed`: `## Not checked`, listing `- NOT ASSESSED — <what> — <reason>` lines, or `- none`. Every check in the coder's `not_run=` list that the reviewer didn't verify itself goes here.
- **Reviewer return line.** Ends with ` not_assessed=<n>` after `verdict=` (and after `verdicts=` at wave scope).
- **Rejected notes.** From pass 2, the reviewer reads the coder's reasons from the fix commit's body (`Rejected <note-id>: ...`), not from a report.
- **Orchestrator.** It passes the coder's last line into the reviewer's brief. On `verdict=ship` with `not_assessed=<n>` and n > 0, it opens the report's `## Not checked` section and copies its items into the packet's ledger row, or a worknote, before shipping.
- **Coders** write no report. Their only skipped-check signal is the `not_run=` suffix (C4), because the owner's rule leaves no other place (D-HT8).

### C7. ID prefixes (HT-10)

```python
DECISION_ROW = re.compile(r"^\|\s*\**(D-[A-Z]{1,4})\d+[a-z]?\b")
PACKET_ROW = re.compile(r"^\|\s*\**([A-Z]{2,4})-\d{2,3}[a-z]?\b")
```

A prefix "belongs" to every folder `docs/analysis/<c>/` whose top-level `*.md` files have a first table cell matching either pattern. A prefix that belongs to more than one folder fails unless it is in `GRANDFATHERED` (prefix → frozenset of folders) **and** its folders are a subset of that entry. `NOT_PACKETS` (a set) lists tokens that aren't packet IDs, such as `UTF`, `SHA` and `AES`. Bare `D1`..`D8` never match.

## Rules inlined in every packet

Each packet below repeats the block it needs, because packet-coder skips CLAUDE.md. **R-PY** is the block for a Python-only packet:

> **Rules (R-PY).**
> - Host: windows. Use the PowerShell tool only, never Bash. Use absolute paths and `git -C <worktree>`, and run every check from the worktree, in the foreground.
> - This packet compiles nothing, so never run cargo. Never `git stash`, `git push`, `git worktree prune` or `gh pr`.
> - Edit only the files listed. `git -C <wt> add <listed files>`, then commit with the brief's message. A finished part may be committed earlier as `(part N)`, but only after its checks pass.
> - Files: soft cap 500 lines, hard cap 700. No `helpers`/`utils` names. Give each module and public function a docstring. Match the surrounding style.
> - Keep each file's line endings. New text files are CRLF; write them with `[IO.File]::WriteAllText('<abs path>', $text.Replace("`r`n","`n").Replace("`n","`r`n"))`. **Exception:** `.github/workflows/*.yml` is LF (`.gitattributes`).
> - Privacy: no IPs, hostnames, credentials, account names, local user paths or session ids in code, tests, fixtures or output.
> - Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> -`, plus `reason=`, `at=` or `not_run=` when they apply. Write no report, worknote or summary. When a tool is denied with `Context cap reached`, call no tool and return `<packet> blocked <sha|-> - reason=context`; the next coder resumes from the branch.

**R-AGENT** adds, for packets that edit `.claude/agents/*.md`: after editing, run `python tools/agent-skills/sync.py`, then `python tools/agent-skills/sync.py --check`, and `git add` every `.codex/agents/*.toml` that `git -C <wt> status --porcelain .codex` lists.

## Wave 1: measurement

### HT-01: transcript reader and CLI wiring

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-01-reader` · worktree `ht-01`
- **Files:**
  - new `tools/token-profile/metrics/transcripts.py` (C1, at most 300 lines);
  - new `tools/token-profile/metrics/transcript_fixtures.py`;
  - new `tools/token-profile/metrics/harness_cli.py`;
  - new stubs `metrics/returns.py`, `metrics/coder_context.py` and `metrics/resume_cards.py`;
  - `metrics/cli.py`;
  - `tools/token-profile/campaigns.json`;
  - new `tools/token-profile/metrics/test_transcripts.py`.
- **Contract:** C1 and C2 in full (above). Copy `RP`, `LINE` and `CAP_MARKER` exactly.
- **Steps:**
  1. Write `transcripts.py` to C1. Import `discover` and `default_projects_root` from `ingest.store`, as `cli.py` does (`sys.path` already holds `tools/token-profile`).
  2. `transcript_fixtures.py` (not a test file) holds:
     - `FIXTURE_PREFIX = "C--fixture-repo"`.
     - `write_subagent(root, session_id, agent_id, agent_type, brief, steps, final_text, ts="2026-10-10T10:00:00Z")`. `steps` is a list of `(tool_name, tool_input, result_text, context_tokens)`. Each step writes one assistant record with a `tool_use` and `usage` (`input_tokens=context_tokens`, `output_tokens=100`), then one user record with the `tool_result`. A final assistant text record follows. The function writes `<root>/<FIXTURE_PREFIX>/<session_id>/subagents/agent-<agent_id>.jsonl` and the `.meta.json` with `{"agentType": agent_type}`, and returns the path.
     - `write_main(root, session_id, records)`, which writes `<root>/<FIXTURE_PREFIX>/<session_id>.jsonl`.
     - `write_agent_file(repo, name, model, reporting=True)`, which writes `<repo>/.claude/agents/<name>.md` with that `model:` and, when `reporting`, a `## Reporting` heading.
  3. `harness_cli.py` holds the C2 arguments for the three names. Its `dispatch` imports the matching module's `run`. The repo-root rule: when `.claude` and `worktrees` are adjacent parts of the path, use the parent of `.claude`.
  4. Each stub is `def run(args): print("status=error not-implemented", file=sys.stderr); return 2`, with a module docstring that names the packet that fills it (HT-02/03/04).
  5. In `cli.py` `main()`, call `harness_cli.add_parsers(sub)` after the `rep` parser. Right after `args = ap.parse_args(argv)`, add `code = harness_cli.dispatch(args)` and `if code is not None: return code`. Import it with `from . import harness_cli`.
  6. In `campaigns.json`, add `"harness-tuning": {"prefixes": ["ht/"]}` in the existing style.
- **Test** (`unittest`, a temp dir with fixtures):
  - `return_line` covers all six kinds, including a backtick-wrapped line, a `reason=context` suffix, and a coder line with `-` as the report (`HT-01 blocked - - reason=context`);
  - `brief_report_paths` finds an absolute Windows path and a relative path;
  - `report_written` handles Write, a backslash Edit, and a PowerShell `WriteAllText`;
  - `is_context_stop` is true for a line with `reason=context`, for prose saying "blocked ... context", and for a dispatch with a `Context cap reached` result;
  - `report_writer_types` keeps a sonnet agent with `## Reporting`, and drops a haiku one, one without `## Reporting`, and a type with no file;
  - `iter_dispatches` honours `since`/`until` and reads `agent_type` from the meta;
  - usage is the max per `message.id`, so two streamed lines with the same id count once;
  - the CLI: `main(["returns", "--out", tmp])` exits 2 with `not-implemented`, which proves the wiring;
  - `registry` still runs.

  **Why it fails on revert:** without `transcripts.py` the imports fail, and without the `cli.py` hook `main(["returns", ...])` exits 2 with an argparse error, not `not-implemented`.
- **Checks:**
  1. `python -m unittest discover -s tools/token-profile -p "test_transcripts.py"`
  2. `python -m unittest discover -s tools/token-profile -p "test_*.py"` (the whole suite stays green)
  3. `python tools/token-profile/metrics registry --repo <wt> --check`: it must not list harness-tuning. A problem that names only other folders is not yours; ignore it.
- **Docs:** none here (HT-05 owns row 60).
- **Commit:** `HT-01: harness metrics transcript reader and CLI wiring`
- **Benefit / re-measure:** an enabler. It makes F1-F7 and F3a reproducible from the repo instead of throwaway scripts. HT-05's baseline proves it.
- **Rules:** R-PY (above), inlined:
  - Host windows, PowerShell only, absolute paths, foreground checks, no cargo, stash, push, prune or `gh pr`.
  - Edit only the listed files.
  - At most 700 lines (soft 500); no helpers/utils names; docstrings.
  - New files CRLF.
  - No IPs, paths, accounts or session ids, and fixtures use made-up ids (`s1`, `a1`).
  - One line back (`HT-01 <status> <sha|-> -`, plus `reason=`/`at=`/`not_run=`), no report or worknote. Commit `transcripts.py`, the fixtures and their tests as part 1 once their test passes. At `Context cap reached`, call no tool and return `HT-01 blocked <sha|-> - reason=context`.

### HT-02: `metrics returns`

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-02-returns` · worktree `ht-02`, starting from HT-01's reviewed head
- **Files:** `tools/token-profile/metrics/returns.py` (replace the stub; at most 300 lines) and new `metrics/test_returns.py`.
- **Contract:** C1 functions (use them; don't redefine `LINE`, `RP` or `CAP_MARKER`). The C2 `returns.json` fields:
  - A **contract dispatch** has `brief_report_paths(brief)` non-empty.
  - `no_usable_line` = prose + short + empty.
  - `no_report_write` means `not report_written(d, paths)`.
  - `cap_denials` counts dispatches with `cap_denied(d)` non-empty.
  - `continuation_dispatches` counts briefs matching `(?i)continuation \d of 3`.
  - `redispatches` counts non-continuation contract dispatches whose first brief report path has a `report_suffix` already seen earlier in the window (sort by `first_ts`).
  - `reviewer_lines` counts usable lines from `packet-reviewer`, and `not_assessed_suffix` counts those whose `extra` has `not_assessed=`.
  - `tokens_total` is the sum of input, cache_read, cache_create and output; `tokens_fresh` is input, cache_create and output. The `no_usable_*` and `no_report_*` token fields sum the same over those subsets.
  - Groups: each `agent_type` (`None` becomes `"unknown"`), plus `all`, `all_but_ops` and `report_writers` (`report_writer_types(args.repo)`).
  - `coder_lines` as in C2: over every `packet-coder` dispatch. `not_run_suffix` counts usable lines whose `extra` has `not_run=`.
  - The stdout line: `status=ok returns contract=<n> writers_no_line=<pct> writers_no_report=<pct> coder_no_line_uncapped=<pct> coder_capped=<n>`, with pct to one decimal.
- **Test** (unittest, fixtures from `transcript_fixtures`; a temp `--repo` with agent files `packet-reviewer` (sonnet, `## Reporting`), `packet-coder` (haiku, `## Reporting`) and `ops` (haiku, none)). Seven dispatches:
  1. a reviewer with a strict line and a written report;
  2. a reviewer with prose and no write;
  3. a reviewer with a buried line and a PowerShell write;
  4. a coder with no report path, ending `HT-01 done abc1234 -`;
  5. a coder with a `Context cap reached` result and prose at the end;
  6. a coder continuation brief with no line;
  7. an ops dispatch whose brief names a report path.

  Assert:
  - every count in `all`;
  - `report_writers` holds only the reviewer dispatches;
  - `coder_lines`: dispatches 3, capped 1, uncapped 2, no_usable_line_uncapped 1, dash_report_lines 1;
  - the stdout line;
  - the written JSON contains no fixture session id;
  - a fixture brief holding a fake absolute user path still passes, because no path is written.

  **Why it fails on revert:** the stub returns 2 and writes no file.
- **Checks:** `python -m unittest discover -s tools/token-profile -p "test_returns.py"`, then the whole token-profile suite (as in HT-01).
- **Docs:** none (HT-05).
- **Commit:** `HT-02: metrics returns - report-contract and coder-line counts`
- **Benefit / re-measure:** it measures F2, F3 and F3a, the yardstick for the HT-06 targets. Re-measured at HT-19 with `--since <wave-2 merge>`.
- **Rules:** R-PY, inlined as in HT-01 (Windows PowerShell, no cargo, listed files only, ≤700 lines, CRLF new files, no private data in output, one line back and no report or worknote, the context line at `Context cap reached`).

### HT-03: `metrics coder-context`

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-03-coder-context` · worktree `ht-03`, starting from HT-01's reviewed head
- **Files:** `tools/token-profile/metrics/coder_context.py` (replace the stub; at most 300 lines) and new `metrics/test_coder_context.py`.
- **Contract:** C1, and the C2 `coder-context.json`. Only `agent_type == "packet-coder"` dispatches count. `stopped` means `is_context_stop(d)`. The category of a tool use:

```python
def category(name, inp):
    if name in ("PowerShell", "Bash"):
        c = (inp or {}).get("command", "")
        if "lane.ps1" in c or "lane.sh" in c:
            if " fmt" in c: return "lane-fmt"
            if "clippy" in c: return "lane-clippy"
            if "nextest" in c or " test" in c: return "lane-test"
            if " check" in c: return "lane-check"
            return "lane-other"
        if re.search(r"\bgit\b", c): return "shell-git"
        if "Get-Content" in c or "cat " in c or "Select-String" in c: return "shell-read"
        return "shell-other"
    if name in ("Read", "Grep", "Glob"): return name
    if name in ("Edit", "Write", "MultiEdit"): return "Edit/Write"
    return "other"
```

  - **Read target** (by result chars, on `norm_path(file_path)`): the first of `/docs/analysis/` → `docs/analysis`, `/docs/` → `docs-other`, `/crates/` → `crates`, `/tools/` → `tools`, `/.claude/` → `.claude`, else `other`.
  - **Shares** are fractions of the group's total result chars, rounded to 3 places.
  - **Medians** use `statistics.median`, with 0 for an empty group:
    - `first_context` and `last_context` are the first and last request context;
    - `output_tokens` is the sum of output;
    - `requests` is the request count;
    - `result_tokens` is the result chars // 4;
    - `lane_calls`, `reads`, `greps` and `edits` count calls by category.
  - `stop_rate` = stops / coders, rounded to 3 places.
  - `continuations`, `continuation_tokens_fresh` and `median_continuation_fresh` (C2).
  - The stdout line: `status=ok coder-context coders=<n> stops=<n> stop_rate=<x> read_share=<x> grep_share=<x> continuations=<n>`.
- **Test:** four packet-coder fixtures:
  1. one with a Read of `C:\x\docs\analysis\c\README.md` (4,000 chars), a Grep (1,000) and a lane check (100), ending done;
  2. one that stops with `reason=context`;
  3. one with a `Context cap reached` denial and prose;
  4. a continuation brief that ends done.

  Plus one packet-reviewer, which must be ignored. Assert `coders=4`, `stops=2`, `stop_rate`, `continuations=1`, the Read and docs/analysis shares in `all`, and the medians. **Why it fails on revert:** the stub returns 2.
- **Checks:** `python -m unittest discover -s tools/token-profile -p "test_coder_context.py"`, then the whole token-profile suite.
- **Docs:** none (HT-05).
- **Commit:** `HT-03: metrics coder-context - what fills a packet-coder's context`
- **Benefit / re-measure:** it measures F4 and F6, the inputs to HT-16's ruling and HT-17's target.
- **Rules:** R-PY, inlined as in HT-01.

### HT-04: `metrics resume-cards`

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-04-resume-cards` · worktree `ht-04`, starting from HT-01's reviewed head
- **Files:** `tools/token-profile/metrics/resume_cards.py` (replace the stub; at most 300 lines) and new `metrics/test_resume_cards.py`.
- **Contract:** C2 `resume-cards.json`. The work runs over `iter_main_transcripts`, with each file's `records` in order.
  - **Compaction:** a record with `type == "system"` and `subtype == "compact_boundary"` whose `timestamp` falls in the window.
  - **Card:** among the records after the boundary and before the first `assistant` record, any `attachment` record whose `json.dumps` matches `CARD = re.compile(r"(Active|Inferred) campaign[^:]*: ([a-z0-9][a-z0-9.-]*[a-z0-9])")`. The tier is `inferred` when group 1 is `Inferred`, else `active`. Count at most one firing per boundary.
  - **Session campaign:** over the last 40 `Agent` tool uses before the boundary, apply the C5 tier-2 counting (both regexes, sets per prompt). The RESUME.md existence filter is not applied here.
  - **Verdict:** `naming < 3` gives `undetermined`; `3*c < 2*naming` gives `mixed`; otherwise `matched` when the card names `top`, else `wrong`.
  - `read_resume_before_dispatch`: after the boundary and before both the next `Agent` use and the next boundary, there is a `Read` whose `file_path` ends with `RESUME.md`.
  - `compactions_followed_by_dispatch`: an `Agent` use comes before the next boundary.
  - The stdout line: `status=ok resume-cards compactions=<n> firings=<n> wrong=<n> wrong_pct=<x>`.
- **Test** (`write_main` fixtures):
  1. a session whose 4 Agent prompts name `docs/analysis/alpha/` and whose card says `Active campaign: beta.` is **wrong**;
  2. the same with a card naming `alpha` is **matched**;
  3. 2 alpha and 2 beta prompts is **mixed**;
  4. 1 prompt is **undetermined**;
  5. an `Inferred campaign (from this session's dispatches): alpha.` card lands in `by_tier.inferred`;
  6. a boundary followed by a Read of `.../RESUME.md` and then an Agent use counts as `read_resume_before_dispatch`;
  7. a boundary outside the window is ignored.

  **Why it fails on revert:** the stub returns 2.
- **Checks:** `python -m unittest discover -s tools/token-profile -p "test_resume_cards.py"`, then the whole token-profile suite.
- **Docs:** none (HT-05).
- **Commit:** `HT-04: metrics resume-cards - wrong-campaign card rate`
- **Benefit / re-measure:** it measures F7, the yardstick for HT-08 (44% → ≤ 5%).
- **Rules:** R-PY, inlined as in HT-01.

### HT-05: baseline run and token-profile docs

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-05-baseline` · worktree `ht-05`, starting from `ht-w1` with 01..04 merged
- **Files:**
  - new `docs/analysis/harness-tuning/baseline/returns.json`, `coder-context.json` and `resume-cards.json`;
  - new `docs/analysis/harness-tuning/baseline/measurements.md` (CRLF);
  - `tools/token-profile/README.md` § Campaign metrics;
  - `docs/guides/token-profiling.md` § Campaign metrics.
- **Steps:**
  1. From the worktree, run each subcommand with `--since 2026-10-03 --until 2026-11-01 --repo C:\Users\Steve\source\projects\Cimmeria --out <wt>\docs\analysis\harness-tuning\baseline`. `--repo` points at the main checkout so `report_writer_types` reads today's agent files. If any line says `status=error`, stop and return `HT-05 failed <sha|-> - reason=checks at=1`.
  2. Write `measurements.md` with a table: metric | ledger value (README F1-F7, F3a) | measured | within 10%? | note. The rows:
     - contract dispatches 1,692;
     - strict 65.5%;
     - no usable line 15.2%;
     - no report write 13.4%;
     - report writers: 691 dispatches, no usable line 14.6%, no report write 5.5%;
     - coder no usable line, cap not fired: 34/696 (4.9%);
     - context stops 167/863;
     - cap denials 240;
     - continuations 87, 6.9M fresh;
     - wrong cards 14/32;
     - the Read share 50.9%, the Grep share 26.4%, and the median output 37.1k.

     Explain each gap over 10% in one line. Two expected gaps: the ledger's LINE accepted any SHA token while C1 needs hex or `-`; and the ledger's coder row counted contract dispatches outside context stops, while `coder_lines` counts every coder dispatch without a cap denial. The README is not edited; the orchestrator copies the numbers.
  3. Add a "Harness metrics" paragraph or table under § Campaign metrics in both docs. Cover:
     - the three subcommands and their arguments (C2);
     - the output files;
     - the `report_writers` group and `coder_lines`;
     - the aggregates-only rule and the privacy gate;
     - the re-measure use (`--since <merge date>`).

     Keep each doc's style; both are CRLF.
- **Test:** none new; this is a docs and data packet. The check is the whole token-profile suite, plus `python -m json.tool` on each JSON file.
- **Docs:** doc-update-map row "The token profiler": the README and guide § Campaign metrics.
- **Commit:** `HT-05: harness-tuning baseline and harness metrics docs`
- **Benefit / re-measure:** the committed baseline that HT-19 compares against.
- **Rules:** R-PY, inlined as in HT-01. Also:
  - `docs/**/*.md` stays CRLF.
  - The JSON files must hold no paths, ids or names; the gate enforces it, so never bypass it.
  - Don't edit the ledger README.

## Wave 2: harness changes

### HT-06: report-contract SubagentStop hook

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-06-report-contract` · worktree `ht-06`, from `main`
- **Files:**
  - new `.claude/hooks/report_contract.py` (C3; at most 250 lines; its module docstring's first line names the hook, which `test_harness_tables.py` requires);
  - `.claude/settings.json`;
  - new `tools/agent-skills/test_report_contract.py`;
  - `tools/agent-skills/test_hooks.py`, the `WiringTest` class only.
- **Contract:** C3 in full (above), with `LINE` and `RP` from C1.
- **Steps:**
  1. Write the hook to C3. Use the standard library only. `main()` returns 0, and `if __name__ == "__main__": sys.exit(main())`.
  2. In `settings.json`, add a top-level `"SubagentStop"` array under `"hooks"`, after `"PreToolUse"`:
     `[{"hooks": [{"type": "command", "command": "f=\"$CLAUDE_PROJECT_DIR/.claude/hooks/report_contract.py\"; [ -n \"$CLAUDE_PROJECT_DIR\" ] && [ -f \"$f\" ] || exit 0; set -- \"$f\"; . \"$CLAUDE_PROJECT_DIR/.claude/hooks/py.sh\"", "timeout": 10}]}]`.
     Keep the 2-space indent and the CRLF.
  3. `test_hooks.py` `WiringTest`:
     - in `test_every_python_hook_goes_through_the_launcher`, collect commands from **every** event in `settings["hooks"]`, not just the two, and from the frontmatter of **every** `.claude/agents/*.md` (each `command:` line between the first two `---`);
     - add `test_settings_wires_report_contract_on_subagent_stop`.
- **Test** (`test_report_contract.py`, unittest). Run the hook as a subprocess with `CLAUDE_PROJECT_DIR` set to a temp project. The project holds `.claude/agents/packet-reviewer.md` (`model: sonnet`, `## Reporting`) and `.claude/agents/packet-coder.md` (`model: haiku`, `## Reporting`). Write a fixture subagent transcript at `<tmp>/s1/subagents/agent-a1.jsonl` with `transcript_path=<tmp>/s1.jsonl`.

  **Report mode** (packet-reviewer):
  1. no line, so block A and the reason names the brief's path;
  2. a good line with a fresh report: no output;
  3. a good line with the report missing: block C;
  4. a report whose mtime is set to 2020 by `os.utime`: block C;
  5. a line naming another path: block B;
  6. `stop_hook_active: true`: no output;
  7. a brief with no report path: no output;
  8. no `last_assistant_message`, so the hook reads the transcript;
  9. no `agent_transcript_path`, so it derives the path;
  10. a backtick-wrapped line, and a line buried in prose: no output;
  11. a relative brief path resolved against `cwd`;
  12. a pass-2 brief that names an old report plus a new one: a line naming the new one with the file written passes, and with no line the reason names the new one.

  **Line mode** (packet-coder):

  13. prose with no line: block L; the reason contains `Write no file` and not the brief's path;
  14. `HT-01 done abc1234 -`, with a brief that names a stray report path and no file written: no output;
  15. `HT-01 blocked - - reason=context`: no output;
  16. prose, with a transcript that holds a `tool_result` containing `Context cap reached`: no output;
  17. an agent file with `model: claude-haiku-4-5` behaves as line mode.

  **Every agent:**

  18. a reviewer with no line and a `Context cap reached` result: no output;
  19. `agent_type` `Explore` (no agent file): no output;
  20. malformed stdin: exit 0, no output.

  **Why it fails on revert:** without the hook, cases 1, 3, 4, 5 and 13 see no block output.
- **Checks:**
  1. `python -m unittest discover -s tools/agent-skills -p "test_report_contract.py"`
  2. `python -m unittest discover -s tools/agent-skills -p "test_*.py"`
  3. `python tools/agent-skills/sync.py --check`
  4. `python -m unittest discover -s tools/docs-gen -p "test_harness_tables.py"`
- **Docs:** the hook docstring now; the prose in HT-11b (row 63). Don't run `regen.py` (HT-11b does).
- **Commit:** `HT-06: SubagentStop hook enforces the report contract and the coder line`
- **Benefit / re-measure:**
  - Report writers: no report write 5.5% → < 2%, and no usable line 14.6% → < 3%.
  - packet-coder: no usable line with the cap not fired 4.9% → < 1%.
  - It never fires on a capped Haiku (D-HT3).

  Re-measured by `metrics returns --since <wave-2 merge>` (`report_writers`, `coder_lines`). The cost is one extra turn on a dispatch that breaks the contract; F2/F3a say those already cost an investigation or a re-dispatch.
- **Rules:** R-PY, inlined:
  - Windows PowerShell, absolute paths, foreground checks.
  - No cargo, stash, push, prune or `gh pr`.
  - Listed files only.
  - At most 700 lines; docstrings.
  - `settings.json` keeps CRLF and its indent; new files CRLF.
  - No private data in fixtures.
  - One line back, no report or worknote. Commit the hook plus cases 1-12 as part 1 once they pass. At `Context cap reached`, return `HT-06 blocked <sha|-> - reason=context`.

### HT-07: the Haiku coder contract in the agent files

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-07-coder-contract` · worktree `ht-07`, from `main`
- **Files:**
  - `.claude/agents/packet-coder.md`: the bullets that start `- Edit only the files`, `- Commit once`, `- Your context limit`, `- If the brief says "continuation` and `- If the brief says "review fix`, the `Report, compactly:` line, and the `## Reporting` paragraph. **Leave the `- Shell:` and `- Host:` bullets as they are**: `test_platform_rule.py` checks their phrases.
  - `.claude/agents/orchestrator.md`: the `- Coder loop` bullet, the `Every brief carries:` paragraph under `## Brief format`, and the `- On a one-line return` bullet.
  - the regenerated `.codex/agents/packet-coder.toml` and `orchestrator.toml`;
  - new `tools/agent-skills/test_coder_contract.py`.
- **Contract:** C4 in full (above).
- **Steps.** Replace each named text with exactly this:
  1. **packet-coder, `- Edit only the files ...` bullet:** keep its first sentence. Replace its ending `say exactly what you changed and why in your report.` with `add the line \`Deviation: <what> - <why>\` to the commit body.`
  2. **packet-coder, `- Commit once ...` bullet:** `- Commit on the worktree's current branch: \`git add\` only the packet's files, then \`git commit\` with the message from the brief (it includes the attribution lines), adding any \`Deviation:\` or \`Rejected\` lines to the body. You may commit a finished part earlier with the subject suffix \`(part N)\`, but only after its checks pass; never commit a failing state. Do not amend other commits.`
  3. **packet-coder, `- Your context limit ...` bullet:** `- You write code and commits, never a report, worknote or summary: the commits are your record. Your context limit is 95k tokens. Read only the line ranges you need (use Grep to find them). Past 90k a hook denies every tool with \`Context cap reached\` and asks for a final report: your final report is then exactly one line, \`<packet> blocked <commit-sha|-> - reason=context\`. Call no tool. Leave uncommitted edits in the worktree; the next packet-coder picks them up. A packet gets at most 3 coder sessions.`
  4. **packet-coder, `- If the brief says "continuation ...` bullet:** `- If the brief says "continuation N of 3, base=<sha>": run \`git -C <worktree> log --oneline <sha>..HEAD\`, \`git -C <worktree> status --porcelain\` and \`git -C <worktree> diff --stat\`, compare them with the spec's steps, and start at the first step not done. Keep uncommitted edits that match the spec; never redo committed work. Then finish, run every check the packet lists, and commit.`
  5. **packet-coder, `- If the brief says "review fix ...` bullet:** `- If the brief says "review fix, pass N of 3", read each review report it names and apply only their \`## Fix notes\`, exactly as written, in the files the notes name; never any other file. If a note does not match the code, don't apply it: add \`Rejected <note-id>: <reason>\` to the commit body. Then rerun every check the packet lists and commit with the brief's message (\`<packet> review fixes, pass N\`); if you rejected every note, commit with \`--allow-empty\` so the reviewer sees the reasons. A fix pass that stops on context returns the same context line, and its fresh sessions count up to 3 within that pass.`
  6. **packet-coder, the `Report, compactly: ...` line:** `Return only the line in \`## Reporting\`. No advice, no summary.`
  7. **packet-coder, the `## Reporting` paragraph:** `Return exactly one line: \`<packet-or-task> <done|blocked|failed> <commit-sha|-> -\`. On \`blocked\` or \`failed\`, add \` reason=<context|checks|spec|host|platform|tool>\` and, when it applies, \` at=<k>\` (the check or step number where you stopped). Add \` not_run=<k>,...\` with the number of each packet check you did not run. For the platform, host and tool cases above, the line is \`<packet> blocked - - reason=<platform|host|tool>\`, followed by the \`blocked:\` text on the next line. Write no report file, even if the brief names one. Never message other agents or wait for replies.`
  8. **orchestrator, `- Coder loop ...` bullet:** `- Coder loop (3 Haiku sessions, then one Sonnet implementer): a \`packet-coder\` writes code and commits, never a report or worknote (owner ruling, harness-tuning D-HT3). When it returns \`blocked\` with \`reason=context\`, or returns no usable line, dispatch a fresh \`packet-coder\` with "continuation N of 3, base=<sha>", where the base is the packet's start SHA; it resumes from the branch. After the third stop, dispatch one \`rust-gameserver-dev\` (tools packets) or \`documentation-writer\` (docs packets) with the same base and a report path. If that fails, \`architect\` re-slices. Fix notes never go back into the session that wrote the code: each fix pass is a fresh \`packet-coder\`.`
  9. **orchestrator, `Every brief carries: ...` paragraph:** `Every brief carries: the worktree path, the packet section path (not its text; the agent reads it) and the return line. A \`packet-coder\` brief names no report path; its line is \`<packet> <done|blocked|failed> <commit-sha|-> -\` with optional \`reason=\`, \`at=\` and \`not_run=\` (packet-coder.md § Reporting). Every other implementer's brief carries the absolute report path inside its worktree (\`<worktree>\.claude\reports\<campaign>\<packet>-<role>.md\`, never the root checkout) and the line \`<packet> <done|blocked|failed> <commit-sha|-> <report-path>\`. A reviewer's brief does not use that return line: it names the reviewer return line in [packet-reviewer.md](packet-reviewer.md) (the reviewed SHA and \`verdict=<ship|fix|decision>\`). It also carries the scope (packet, wave or campaign), the commit range, the pass number, the coder's last return line and, from pass 2, the previous report's path. The \`<role>\` is \`impl\`, \`review-<N>\`, \`ruling-<N>\` or \`ruling-check\` (a wave review \`wave<W>-review-<N>\`, the campaign \`campaign-review-<N>\`). Keep briefs under about 25 lines.`
  10. **orchestrator, `- On a one-line return ...` bullet:** `- On a one-line return, update the ledger row; an implementer's \`done\` starts that packet's review loop. A \`packet-coder\`'s \`blocked\` or \`failed\` has no report: route it by \`reason=\` (\`context\` → the coder loop; \`checks\` → the Sonnet implementer; \`spec\` → \`architect\`; \`host\`, \`platform\` or \`tool\` → the packet's host note). For any other agent, open its report only when the return says \`blocked\`, \`failed\` or \`verdict=decision\`.`
  11. R-AGENT: run sync, then check.
- **Test** (`test_coder_contract.py`, unittest, reading the two files from the repo root):
  - **packet-coder.md contains** `Context cap reached`, `reason=context`, `base=<sha>`, `Rejected <note-id>:`, `Deviation: <what> - <why>`, `not_run=`, `--allow-empty`, `(part N)`, and in `## Reporting` the text `<commit-sha|-> -`.
  - **packet-coder.md does not contain** `worknotes/`, `worknote first`, `<report-path>`, `in your report`, `write your report` or `Report, compactly`.
  - **orchestrator.md:** its `- Coder loop` bullet contains `base=<sha>` and `no usable line`, and not `worknote path`. Its `Every brief carries:` paragraph contains `names no report path`.

  **Why it fails on revert:** today's packet-coder.md has `worknotes/` and lacks `Context cap reached`; today's orchestrator bullet has `worknote path`.
- **Checks:**
  1. `python -m unittest discover -s tools/agent-skills -p "test_coder_contract.py"`
  2. `python -m unittest discover -s tools/agent-skills -p "test_platform_rule.py"`
  3. `python -m unittest discover -s tools/agent-skills -p "test_*.py"`
  4. `python tools/agent-skills/sync.py --check`
- **Docs:** the agent files are the doc. The skill and the workflow docs follow in HT-11a.
- **Commit:** `HT-07: packet-coder writes code and commits, returns one line, resumes from the branch`
- **Benefit / re-measure:**
  - The coder's instructions match the owner's rule and the user-level cap. No coder wastes a turn on a write the cap denies (F5: 51 Edits and 12 Writes were the first denied tool).
  - A continuation resumes from the branch rather than from a worknote nobody can write past the cap.
  - The orchestrator routes every coder return without opening a file.

  Re-measured with `metrics returns` (`coder_lines.capped_usable_line / capped`, informational) and with the `no_usable_line_uncapped` target shared with HT-06.
- **Rules:** R-PY + R-AGENT, inlined:
  - Windows PowerShell, absolute paths, foreground checks.
  - No cargo, stash, push, prune or `gh pr`.
  - Listed files only; edit only the named bullets and paragraphs.
  - At most 700 lines; docstrings.
  - Agent `.md` files keep CRLF.
  - Run `python tools/agent-skills/sync.py`, then `--check`, and add the changed `.codex/agents/*.toml`.
  - One line back, no report or worknote. At `Context cap reached`, return `HT-07 blocked <sha|-> - reason=context`.

### HT-08: per-session resume card

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-08-resume-card` · worktree `ht-08`, from `main`
- **Files:**
  - `.claude/hooks/resume_card.py` (C5; at most 200 lines; update the docstring);
  - `tools/agent-skills/test_hooks.py`, the `ResumeCardTest` class only (lines 40-115 today);
  - `.claude/agents/orchestrator.md`, the `## State on disk` paragraph;
  - the regenerated `.codex/agents/orchestrator.toml`.
- **Contract:** C5 in full (above).
- **Steps:**
  1. In `main()`, parse stdin JSON. A malformed payload means no card, exit 0. Then call `build_card(root, transcript_path)`.
  2. Keep `POINTER`, `CARD_LINES` and the name check. Add `INFERRED`, `tier1(transcript)` and `tier2(transcript, root)`.
  3. Replace the orchestrator's State on disk paragraph with: `Keep \`docs/analysis/<campaign>/RESUME.md\` current at every wave boundary. When you start or resume a campaign in a session, Write its folder name to \`.claude/state/active-campaign\`: the resume card after a compaction comes from this session's own last write of that file (or, failing that, from the campaign most of your recent dispatches name). Write an empty file when the campaign closes. After a compaction, read RESUME.md first.`
  4. R-AGENT: run sync, then check.
- **Test** (rewrite `ResumeCardTest`; a helper writes a fixture main transcript and passes `{"transcript_path": ...}` on stdin). Cases:
  1. **The F7 bug:** the global pointer file says `other` and has a RESUME. The transcript has no pointer write and no Agent uses. There is no card.
  2. The transcript's last Write of `<tmp>\.claude\state\active-campaign` says `demo`, and the global file says `other`. The card names `demo`.
  3. Two writes, `alpha` then `demo`: the card names `demo`.
  4. An Edit whose `new_string` is `demo`: the card names `demo`.
  5. A last write that is empty, plus 5 Agent prompts naming `docs/analysis/demo/`: no card.
  6. No pointer write, and 3 of 4 prompts name `demo`: an inferred card, whose text starts with `Context was compacted. Inferred campaign`.
  7. 2 of 4 prompts: no card. 2 of 2 prompts: no card (`naming < 3`).
  8. A prompt that names a campaign with no RESUME.md doesn't count.
  9. Keep today's truncation at 40 lines, the BOM, the path-name rejection (`../demo` written as the content), malformed JSON, and the `build_card` raising case.
  10. A missing `transcript_path`: no card.

  **Why it fails on revert:** the old hook prints a card for case 1 and names `other` in case 2.
- **Checks:**
  1. `python -m unittest discover -s tools/agent-skills -p "test_hooks.py"`
  2. `python -m unittest discover -s tools/agent-skills -p "test_*.py"`
  3. `python tools/agent-skills/sync.py --check`
- **Docs:** the hook docstring; the skill and docs prose in HT-11b.
- **Commit:** `HT-08: resume card comes from this session, not a shared pointer`
- **Benefit / re-measure:** wrong-campaign cards go from 44% to ≤ 5%, with tier 1 and tier 2 reported apart. Re-measured with `metrics resume-cards --since <wave-2 merge>`.
- **Rules:** R-PY + R-AGENT, inlined:
  - Windows PowerShell, absolute paths, foreground checks.
  - No cargo, stash, push, prune or `gh pr`.
  - Listed files only; edit only `ResumeCardTest` in `test_hooks.py` and only `## State on disk` in orchestrator.md.
  - At most 700 lines.
  - CRLF kept.
  - Sync, then check.
  - One line back, no report or worknote. At `Context cap reached`, return `HT-08 blocked <sha|-> - reason=context`.

### HT-09: Not checked, `not_run=`, and the report-format test

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-09-not-checked` · worktree `ht-09`, starting from `ht-w2` with 10, 08, 06 and 07 merged
- **Files:**
  - `.claude/agents/packet-reviewer.md`: the report-sections paragraph, the `**From pass 2:**` bullet, and `## Reporting`;
  - `.claude/agents/orchestrator.md`: `## Waiting`;
  - the regenerated `.codex/agents/*.toml`;
  - new `tools/agent-skills/test_report_formats.py`.
- **Contract:** C6, and C1's `LINE`.
- **Steps:**
  1. **Reviewer sections.** Change "Write the report with three sections" to four. After the `## Not fixed` bullet, add `- \`## Not checked\`: every check the spec or your scope asked for that you did not run or could not judge, one per line as \`- NOT ASSESSED — <what> — <reason>\`; include each check in the coder's \`not_run=\` list (from the brief) that you did not verify yourself. Write \`- none\` when nothing is left.`
  2. **Reviewer, pass 2.** In the `**From pass 2:**` bullet, replace `Judge a rejected note on its merits, and drop it if the coder's reason holds.` with `A rejected note's reason is a \`Rejected <note-id>:\` line in the fix commit's body (\`git log -1 --format=%B\`); judge it on its merits, and drop the note if the reason holds.`
  3. **Reviewer `## Reporting`.** Append ` not_assessed=<n>` (the number of NOT ASSESSED lines) to both return-line templates, after `verdict=`/`verdicts=`.
  4. **Orchestrator `## Waiting`.** Add the bullet `- A reviewer line with \`verdict=ship\` and \`not_assessed=<n>\` above 0: open only the report's \`## Not checked\` section and copy its items into the packet's ledger row (or worknote) before shipping, so the close-out sees what nobody checked.`
  5. R-AGENT: run sync, then check.
- **Test** (`test_report_formats.py`). Load `.claude/hooks/report_contract.py` with `importlib.util.spec_from_file_location` and use its `LINE`. Cases:
  - **Sample lines.** These match:
    - `HT-01 done abc1234 C:\w\.claude\reports\c\HT-01-review-1.md`
    - `HT-01 done abc1234 -`
    - `HT-01 blocked - - reason=context`
    - `HT-01 done abc1234 - not_run=3,4`
    - `wave2 done abc1234 x.md verdict=fix verdicts=HT-06:ship,HT-07:fix not_assessed=2`
    - `` `HT-01 blocked - x.md reason=context` ``

    These don't: `HT-01 done` and `done abc x.md` (bad SHA).
  - **packet-reviewer.md** contains `## Verdict`, `## Fix notes`, `## Not fixed` and `## Not checked`, in that order, plus `NOT ASSESSED —`, `not_run=`, `Rejected <note-id>:` and `not_assessed=<n>`.
  - **orchestrator.md** contains `not_assessed=`.
  - **Templates.** Fill each backticked return-line template in packet-coder.md and packet-reviewer.md: `<packet-or-task>` → `HT-01`, `<done|blocked|failed>` → `done`, `<commit-sha|->` and `<reviewed-head-sha>` → `abc1234`, `<report-path>` → `x.md`, `<ship|fix|decision>` → `ship`, `<n>` → `0`, `<packet>:<ship|fix|decision>,...` → `HT-01:ship`. Each must match `LINE`.
  - **Metrics copy.** `@unittest.skipUnless((ROOT / "tools/token-profile/metrics/transcripts.py").is_file(), ...)`: the metrics `LINE.pattern` equals the hook's.

  **Why it fails on revert:** removing `## Not checked`, the suffix or the `Rejected` reading fails the reviewer case, and a drifting template fails the template case. This closes AH-24-review-2:25 (no test guarded the verdict words or the return-line format).
- **Checks:** `python -m unittest discover -s tools/agent-skills -p "test_report_formats.py"`, then all `test_*.py` in tools/agent-skills, then `python tools/agent-skills/sync.py --check`.
- **Docs:** HT-11b (row 63: the cycle in the skill).
- **Commit:** `HT-09: reviewer Not checked section, not_run passthrough, report-format test`
- **Benefit / re-measure:** unrun checks become visible on the return line, where today about 6 were shipped silently. 100% of reviews after wave 2 carry the section (`metrics returns` `not_assessed_suffix` / `reviewer_lines`). Coder skips show as `coder_lines.not_run_suffix`.
- **Rules:** R-PY + R-AGENT, inlined:
  - Windows PowerShell, no cargo, listed files only, at most 700 lines, CRLF kept, sync then check.
  - One line back, no report or worknote. At `Context cap reached`, return `HT-09 blocked <sha|-> - reason=context`.

### HT-10: unique decision and packet ID prefixes

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-10-id-prefixes` · worktree `ht-10`, from `main`
- **Files:** `tools/agent-skills/test_ledgers.py` and `.github/workflows/agent-skills.yml` (LF).
- **Contract:** C7 in full (above).
- **Steps:**
  1. Write a scan into the test module: `prefix_owners() -> dict[str, set[str]]` over `docs/analysis/*/*.md` (the top level of each campaign folder only, read as utf-8 with errors replaced), using `DECISION_ROW` and `PACKET_ROW`.
  2. Run it once by hand (`python -c "import sys; sys.path.insert(0, r'<wt>\tools\agent-skills'); import test_ledgers as t; print({k: sorted(v) for k, v in t.prefix_owners().items() if len(v) > 1})"`).
  3. Put every prefix it lists into `GRANDFATHERED` with its exact folders, except tokens that aren't packet IDs (`UTF`, `SHA`, `AES` and similar), which go to `NOT_PACKETS`. Add a one-line comment: "frozen 2026-10-31; never add a new campaign here".
  4. Add `test_prefixes_belong_to_one_campaign`. Its failure message names each prefix and its folders and points at the campaign-packet skill § 1.
  5. In `agent-skills.yml`, add `- "docs/analysis/**/*.md"` to both `pull_request.paths` and `push.paths`, after the `RESUME.md` line. Keep LF.
- **Test:** the new test itself. Prove it: temporarily copy a folder's README into `docs/analysis/zz-probe/README.md`, so its prefixes now belong to two folders. The test must fail. Then delete the probe. **Why it fails on revert:** removing the test removes the guard (it is a guard test). The probe proves it catches a new collision; the reviewer repeats it.
- **Checks:** `python -m unittest discover -s tools/agent-skills -p "test_ledgers.py"`, then all `test_*.py` in tools/agent-skills.
- **Docs:** HT-11b writes the SKILL § 1 note.
- **Commit:** `HT-10: one campaign per decision and packet ID prefix`
- **Benefit / re-measure:** no new cross-campaign ID collisions, enforced in CI, so `metrics packets` joins stay right. Checked by the test in CI.
- **Rules:** R-PY, inlined:
  - Windows PowerShell, no cargo.
  - Listed files only.
  - The `.yml` stays **LF**; `test_ledgers.py` keeps CRLF.
  - Don't leave the probe folder behind.
  - One line back, no report or worknote. At `Context cap reached`, return `HT-10 blocked <sha|-> - reason=context`.

### HT-11a: the coder contract in the skill and the workflow docs

- **Agent / host:** documentation-writer · `Host: windows` · branch `ht/HT-11a-coder-docs` · worktree `ht-11a`, starting from `ht-w2` with 10, 08, 06 and 07 merged (in parallel with HT-09)
- **Report:** `<wt>\.claude\reports\harness-tuning\HT-11a-impl.md`; return line `HT-11a <done|blocked|failed> <commit-sha|-> <report-path>`.
- **Contract:** C4, and the packet-coder and orchestrator text HT-07 merged.
- **Files** (all CRLF; line numbers are from 2026-10-31, so find each by its text):
  - `.claude/skills/campaign-packet/SKILL.md`:
    - § 1, the agent table rows for "One packet, one worktree" and "Apply a review's fix notes": the output becomes "local commits and a one-line return".
    - § 3 step 2: a packet-coder brief names no report path. Its line is C4's. A continuation brief carries `base=<start SHA>`. Other implementers keep the report path.
    - § 3 step 4: route a coder's `blocked` by `reason=`.
    - § 4 fix-pass step 2: rejected notes go in the commit body; all rejected → `--allow-empty`. Drop the `fix-<N>` report role (role list in the long paragraph after the table).
    - § 4 ship step 2: worknotes are the orchestrator's.
    - § Sessions, the table row "Coder session" and escalation step 1: a context stop returns the line; the next session resumes from the branch with `base=`.
  - `docs/agents/development-workflow.md`: the coder-report and worknote lines in § Worker lifetime and notifications (around lines 112-113 and 129-130).
  - `docs/agents/ai-sdlc.md`: the coder report and worknote mentions (around lines 18, 57-63, 85, 104 and 154).
  - `docs/agents/ai-sdlc-example.md`: the coder report in the example (around line 50).
  - `docs/agents/ai-harness.md`: the coder report and worknote mentions (around lines 124 and 131).
  - `AGENTS.md`: the coder return contract (around lines 33-35).
  - whatever `sync.py` regenerates.
- **What to write:**
  - Say once, in the skill's § 3, why: the owner's rule (D-HT3) is that Haiku coders write code, not reports, and the user-level cap (Haiku agents only) stays. Cite it as an owner rule; never call the cap a defect.
  - Sonnet and Opus agents keep their reports and worknotes; change only what a packet-coder is told to do.
  - Leave `docs/agents/rules-and-gotchas.md` (it covers other workers) and every status doc alone.
- **Then run:**
  1. `python tools/agent-skills/sync.py`
  2. `python tools/docs-gen/regen.py --check --skip-crate-graph`
  3. `python -m unittest discover -s tools/agent-skills -p "test_*.py"`
  4. `python -m unittest discover -s tools/docs-gen -p "test_*.py"`
  5. `python tools/agent-skills/sync.py --check`

  If step 2 fails on a block you didn't touch, note it in the report; HT-11b runs `regen.py`.
- **Test:** docs only. `test_agent_refs.py`, `test_harness_tables.py`, `test_ledgers.py` and HT-07's `test_coder_contract.py` must pass. Before committing, Grep the listed files for `-coder.md`, `worknote` and `coder report` and account for every remaining hit in the report.
- **Docs:** doc-update-map row 62 (agent workflow). List it in the report for the PR body.
- **Commit:** `HT-11a: document the Haiku coder contract - code and commits, one line, resume from the branch`
- **Benefit / re-measure:** none directly. No doc tells an orchestrator to give a coder a report path, which keeps `coder_lines.dash_report_lines` near 100% of usable coder lines after wave 2.
- **Rules:**
  - Windows PowerShell; no cargo, stash, push, prune or `gh pr`.
  - Never hand-edit a `<!-- gen:NAME -->` block; `regen.py` owns it.
  - `docs/**/*.md` stay CRLF.
  - Files under 700 lines; if `development-workflow.md` would pass 700, move detail to the skill, don't grow the doc.
  - Edit the skill only under `.claude/skills/`, then run `sync.py`.
  - One commit.
  - Don't edit agent files (HT-07 and HT-09 own them).
  - Context: if you run short, commit what passes, write `<wt>\docs\analysis\harness-tuning\worknotes\HT-11a.md` (done, left, next file), write your report, and return `blocked` with `reason=context`.

### HT-11b: wave-2 harness docs and regen

- **Agent / host:** documentation-writer · `Host: windows` · branch `ht/HT-11b-docs` · worktree `ht-11b`, starting from `ht-w2` with HT-06..HT-10 and HT-11a merged
- **Report:** `<wt>\.claude\reports\harness-tuning\HT-11b-impl.md`; return line `HT-11b <done|blocked|failed> <commit-sha|-> <report-path>`.
- **Files** (all CRLF):
  - `docs/agents/development-workflow.md`: § Agent skills and the shell guard;
  - `docs/agents/ai-harness.md`, prose only;
  - `docs/agents/ai-sdlc.md`, if it describes hooks or the resume card;
  - `AGENTS.md`, if it describes hooks or the resume card;
  - `.claude/skills/campaign-packet/SKILL.md`;
  - `tools/agent-skills/README.md`;
  - whatever `regen.py` and `sync.py` regenerate.
- **What to write:**
  - **The hooks:**
    - `report_contract.py`: SubagentStop; blocks once; report mode for report writers and line mode for Haiku agents with `## Reporting`; never after `Context cap reached`; the four reasons.
    - The per-session resume card: both tiers; the pointer is written per session.
    - The user-level Haiku cap as the owner's rule (D-HT3): it is not in the repo, the repo never works around it, and a capped coder returns one line.
  - **Return lines.** `reason=`, `at=`, `not_run=` and `not_assessed=<n>`, plus the reviewer's `## Not checked`.
  - **SKILL § 1:**
    - "A decision prefix (`D-XX`) and a packet prefix (`XX-`) belong to one campaign; `test_ledgers.py` enforces it. Don't start a table row with another campaign's ID; cite it in a later column."
    - "A Decisions table may add **Recommended** and **Evidence** columns."
  - **SKILL § 3:** the SubagentStop hook blocks a missing line or report once.
  - **SKILL § 4:** four report sections; the reviewer copies `not_run=` into `## Not checked`; on `ship` with `not_assessed`, copy the items to the ledger.
  - **SKILL § 5:** the card is per session; the pointer is written per session; it's emptied at close-out.
  - **`tools/agent-skills/README.md`:** add `test_report_contract.py`, `test_coder_contract.py` and `test_report_formats.py` to the test list. Split the long list into a bullet list while you're there, so later packets don't conflict on one line.
  - In `ai-harness.md`'s test notes, note that `test_report_formats.py` closes the AH-24-review-2:25 note.
- **Then run:**
  1. `python tools/agent-skills/sync.py`
  2. `python tools/docs-gen/regen.py --skip-crate-graph`
  3. `python tools/docs-gen/regen.py --check --skip-crate-graph`
  4. `python -m unittest discover -s tools/agent-skills -p "test_*.py"`
  5. `python -m unittest discover -s tools/docs-gen -p "test_*.py"`
  6. `python tools/agent-skills/sync.py --check`
- **Test:** docs only. The existing `test_agent_refs.py`, `test_harness_tables.py` and `test_ledgers.py` must pass.
- **Docs:** doc-update-map rows 62 (agent workflow) and 63 (harness wiring). List them in the report for the PR body.
- **Commit:** `HT-11b: document the report-contract hook, Not checked and the per-session resume card`
- **Benefit / re-measure:** none directly. It keeps docs matching behaviour, which the campaign review checks.
- **Rules:** as HT-11a, plus: don't redo HT-11a's coder-contract text; link to it.
