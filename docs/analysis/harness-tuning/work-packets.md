# harness-tuning: work packets, dispatch rules and wave 1

> **Type:** Reference · **Audience:** The orchestrator, packet-coders and packet-reviewers of this campaign · **Updated:** 2026-11-01 (re-sliced for D-HT3 and D-HT10; wave 2 moved to packets-wave2.md)
> **Companions:** [Ledger](README.md), [wave 2 and contracts C3-C7](packets-wave2.md), [waves 3-5](packets-wave3.md), [resume card](RESUME.md)

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
| 1 | HT-01..HT-05, HT-02a | one PR from `ht-w1` | 01, 02, 03, 04, 02a, 05 | now (HT-01 Ready) |
| 2 | HT-06..HT-10, HT-11a, HT-11b | one PR from `ht-w2` | 10, 08, 06, 07, 11a, 09, 11b | now; runs alongside wave 1 (no shared files) |
| 3 | HT-12a, 12b, 13, 15a, 15b, 15c | one PR from `ht-w3` | 13, 12a, 12b, 15b, 15c, 15a | wave 2 merged |
| ruling | HT-16 (architect, no code) | none (ledger and packet text only) | n/a | wave 1 merged |
| 4 | HT-14, HT-17a, HT-17b, HT-18 | one PR from `ht-w4` | 14, 17a, 17b, 18 | wave 3 merged; HT-17a/b specs in packets-wave3.md (HT-16 ruled) |
| 5 | campaign review, then HT-19 | the review's fixes ship as one PR, then HT-19's PR | n/a | wave 4 merged and the re-measure window full (README § Targets) |

Inside a wave:

- **Wave 1.** HT-01 first. HT-02, HT-03 and HT-04 then run in parallel, each starting from HT-01's reviewed head. HT-05 starts from `ht-w1` once 01..04 are merged into it.
- **Wave 2.** HT-06, HT-07, HT-08 and HT-10 run in parallel from `main`. Once 10, 08, 06 and 07 are merged into `ht-w2`, HT-09 (agent files and a test) and HT-11a (the skill and docs) start from it in parallel. They share no file. HT-11b starts from `ht-w2` after 11a and 09 are merged. Seven packets: HT-11a and HT-11b are documentation-writer passes, so the wave reviewer holds five coder packets plus two doc diffs.
- **Wave 2 packets and contracts C3-C7.** See [packets-wave2.md](packets-wave2.md).
- **Waves 3-5.** See [packets-wave3.md](packets-wave3.md).

Shared files in wave 2:

- `test_hooks.py`: HT-06 edits `WiringTest` and HT-08 edits `ResumeCardTest`.
- `orchestrator.md`: HT-07 edits three paragraphs (`- Coder loop`, `Every brief carries:`, `- On a one-line return`) and HT-08 edits `## State on disk`; HT-09 edits `## Waiting` after both.
- `campaign-packet/SKILL.md`: HT-11a, then HT-11b.

The wave reviewer runs `git merge-tree` on the HT-06/HT-08 and HT-07/HT-08 pairs.

## Contract

C1 and C2 are here. C3-C7 (the wave-2 contracts) are in [packets-wave2.md](packets-wave2.md#contract-continued-c3-c7).

### C1. Shared definitions: `tools/token-profile/metrics/transcripts.py` (HT-01)

```python
RP = re.compile(r"[^\s`'\"()<>]*\.claude[\\/]reports[\\/][^\\/\s`'\"()<>]+[\\/][^\\/\s`'\"()<>]+\.md")
LINE = re.compile(r"^(?P<id>\S+)\s+(?P<status>done|blocked|failed)\s+(?P<sha>[0-9a-f]{7,40}|-)\s+"
                  r"(?P<report>\S+\.md|-)(?P<extra>(?:\s+[a-z_]+=\S+)*)\s*$")
SHORT_CHARS = 200
CAP_MARKER = "Context cap reached"      # the user-level Haiku cap's deny text (D-HT3); never changed by this campaign
CAP_RE = re.compile(r"^(?:PreToolUse:\S+ hook error: )?" + re.escape(CAP_MARKER))   # the same pattern as C3 step 5
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
def cap_denied(d: Dispatch) -> list[ToolUse]                                   # tool uses whose result is a cap deny (CAP_RE)
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
- **`cap_denied`.** The tool uses `t` with `CAP_RE.match(d.results.get(t.id, "").lstrip())`. This is C3 step 5's cap deny: the result text (a `content` string, or its text blocks joined, as `text_of` gives it) starts with the marker, or with `PreToolUse:<tool> hook error: ` and the marker. The marker anywhere else is not a cap: a Read or Grep result that quotes it, the brief, or assistant text. `is_error` is not required. The hook and this reader must agree on which dispatches are capped (ruling HT-01-1).
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

`returns.json` holds `groups`: one entry per known agent type (an agent file under `--repo`'s `.claude/agents/`, a built-in name or a retired name; see HT-02a), `other` for any other label, `unknown` when there is no type, plus `all`, `all_but_ops` and `report_writers` (the contract dispatches whose type is in `report_writer_types(repo)`). Each entry has these integer fields:

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
  - `is_context_stop` is true for a line with `reason=context`, for prose saying "blocked ... context", and for a dispatch with a real cap deny result (`PreToolUse:Edit hook error: Context cap reached: 90,515 tokens > 90,000. STOP NOW. Do not call any more tools.`);
  - `cap_denied` is empty, and `is_context_stop` false, for a done coder whose Read result quotes the marker mid-text (`    54\tCAP_MARKER = "Context cap reached"`); `read_dispatch` on a transcript whose deny `tool_result` content is a list of `text` blocks gives one `cap_denied`; when `.claude/hooks/report_contract.py` exists, its `CAP_RE.pattern` equals C1's (skipped otherwise);
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
  5. a coder with a cap deny result (the text `PreToolUse:Edit hook error: Context cap reached: 90,515 tokens > 90,000. STOP NOW.`, which C1 `cap_denied` matches) and prose at the end;
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
  3. one with a cap deny result (`PreToolUse:Edit hook error: Context cap reached: 90,515 tokens > 90,000. STOP NOW.`) and prose;
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

### HT-02a: bucket free-form agent-type labels in `metrics returns`

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-02a-type-buckets` · worktree `ht-02a`, starting from `ht/w1` at 212b80321
- **Files:** only these two:
  - `tools/token-profile/metrics/returns.py` (160 lines now; stay under 220);
  - `tools/token-profile/metrics/test_returns.py` (189 lines now).
- **Why:** on real data, `agentType` holds one-off labels that are campaign and task names. As group keys they break C2's rule of no names in the output.
- **Contract:**
  - Add constants:
    - `BUILTIN_TYPES = frozenset({"general-purpose", "Explore", "Plan", "claude", "fork", "claude-code-guide", "statusline-setup"})`;
    - `RETIRED_TYPES = frozenset({"packet-review-fixer"})`.
  - Add `known_types(repo) -> set[str]`. It returns `{p.stem for p in (Path(repo) / ".claude" / "agents").glob("*.md")} | BUILTIN_TYPES | RETIRED_TYPES`, with a docstring.
  - Add `_group_key(t: str, known: set[str]) -> str`. It returns `t` when `t in known` or `t == "unknown"`, else `"other"`.
  - `_groups(rows, writers, known)`: in `names = [r["type"], "all"]`, replace `r["type"]` with `_group_key(r["type"], known)`. The `ops` check and the `writers` check keep using the raw `r["type"]`. `_coder_lines` is unchanged.
  - `run`: call `_groups(rows, report_writer_types(args.repo), known_types(args.repo))`.
  - Module docstring, line 4: change "no text, paths or session ids." to "no text, paths, session ids or free-form labels: an agent type with no agent file and no built-in or retired name is counted under `other`."
  - The stdout line, the schema name and every other field stay unchanged.
- **Test:** add one new test method in `test_returns.py`, with its own temp dir. Leave the existing seven-dispatch test untouched.
  - Set up a `--repo` with one agent file, `packet-reviewer` (sonnet, `## Reporting`).
  - Write four dispatches with agent types `cm10-closeout-docs`, `nt-50a`, `general-purpose` and `packet-review-fixer`. Briefs have no report path; final text is `x`.
  - Assert, on the loaded `returns.json`:
    - `groups["other"]["dispatches"] == 2`;
    - `groups["general-purpose"]["dispatches"] == 1`;
    - `groups["packet-review-fixer"]["dispatches"] == 1`;
    - `groups["all"]["dispatches"] == 4`;
    - neither `cm10-closeout-docs` nor `nt-50a` appears anywhere in the file's text.

  **Why it fails on revert:** without `_group_key`, there are keys `cm10-closeout-docs` and `nt-50a` and no key `other`.
- **Checks** (from the worktree, in the foreground):
  1. `python -m unittest discover -s tools/token-profile -p "test_returns.py"`
  2. `python -m unittest discover -s tools/token-profile -p "test_*.py"`
- **Docs:** none. HT-05's fix pass edits the docs.
- **Commit:** `HT-02a: metrics returns - bucket free-form agent-type labels under other`
- **Rules (R-PY, inlined):**
  - Host windows. Use the PowerShell tool only, never Bash. Use absolute paths and `git -C <wt>`, and run every check from the worktree, in the foreground.
  - Never run cargo. Never `git stash`, `git push`, `git worktree prune` or `gh pr`.
  - Edit only the two listed files. `git -C <wt> add` them, then commit with the message above.
  - Hard cap 700 lines (soft cap 500). No `helpers`/`utils` names. Give each new function a docstring. Match the surrounding style.
  - Keep each file's line endings. Check them with `git -C <wt> diff --stat` and the file's existing CRLF or LF.
  - Privacy: fixtures use made-up ids (`s1`, `a1`). No IPs, hostnames, accounts, local user paths or session ids.
  - Return exactly one line: `HT-02a <done|blocked|failed> <commit-sha|-> -`, plus `reason=`, `at=` or `not_run=` when they apply. No report file and no worknote. At `Context cap reached`, call no tool and return `HT-02a blocked <sha|-> - reason=context`.

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

     Explain each gap over 10% in one line. Three expected gaps: the ledger's LINE accepted any SHA token while C1 needs hex or `-`; the ledger's coder row counted contract dispatches outside context stops, while `coder_lines` counts every coder dispatch without a cap denial; and if the ledger counted a cap denial wherever the marker appeared, C1 counts only a result that starts with the deny (ruling HT-01-1), so `cap_denials` and context stops can come out lower. The README is not edited; the orchestrator copies the numbers.
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
