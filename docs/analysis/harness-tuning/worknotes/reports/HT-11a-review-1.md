## Verdict

fix

## Fix notes

N1. docs/agents/ai-sdlc-example.md:50 — the text says "AH-11m ran before that contract, so the real line carried a report path", but line 56 of the same file says the line the coder actually returned is not recorded; the doc asserts what it says nothing records — in line 50 replace "AH-11m ran before that contract, so the real line carried a report path. Under it, the line has this shape:" with "AH-11m ran before that contract, so its line would have ended in a report path where this one has `-`. Under the contract, the line has this shape:" — verify: Grep `carried a report path` in docs/agents/ai-sdlc-example.md returns nothing, and the file is still CRLF with no bare LF.

N2. .claude/skills/campaign-packet/SKILL.md:3 (then run `python tools/agent-skills/sync.py` to refresh `.agents/skills/campaign-packet/SKILL.md`) — the front-matter description still says every worker works "with report files and one-line returns", which tells a reader that packet-coders write report files — in the `description:` value replace "with report files and one-line returns." with "with one-line returns (every worker except a packet-coder also writes a report file)." — verify: `python tools/agent-skills/sync.py --check` prints "agent skills mirror is current", and Grep `with report files and one-line returns` over `.claude/skills` and `.agents/skills` returns nothing.

## Not fixed

- Commit subject: the spec's commit line is `HT-11a: document the Haiku coder contract - code and commits, one line, resume from the branch`; the commit is `HT-11a coder contract in the campaign skill and docs`. The coder cannot amend under the repo rules, and the PR title comes from the ship call. Orchestrator call: reword at ship (`ship.ps1 pr -m`), no fix note.
- `python tools/docs-gen/regen.py --check --skip-crate-graph` reports stale `gen:docs-md-count` (docs/readme.md), `gen:re-findings-count` (two READMEs) and `gen:harness-hooks` (docs/agents/ai-harness.md, README.md). None is in a hunk of this diff, and the spec says HT-11b runs `regen.py`. Out of scope.
- docs/agents/rules-and-gotchas.md:109 ("hand off by worknote") is left alone by the spec; it covers other workers.
- Other places the brief named were checked and are clean: `.claude/agents/packet-coder.md`, `.claude/agents/orchestrator.md` and the `.codex` twins already say no report or worknote (HT-07). `README.md:124` does not tell a coder to write a report. Remaining "worknote" and "report" hits in development-workflow.md (129, 130), ai-sdlc.md (18, 97, 153, 154) and the skill (106-107, 209-210, 235) are orchestrator, architect or non-coder text.
- The "why" (D-HT3) is stated in the skill's § 3 and again in development-workflow.md, ai-sdlc.md and AGENTS.md as a one-clause citation. The spec says "once, in the skill's § 3"; the repeats are citations, not restatements. Left as is.

## Checks run

- CRLF: all 7 changed files have 0 bare LF (.agents mirror included).
- `sync.py --check`: current. `unittest` in tools/agent-skills (29 tests, includes test_coder_contract.py) and tools/docs-gen: OK. Both sets of runs left the worktree clean (`git status --porcelain` empty).
- Anchors `#3-dispatch-a-wave`, `#4-review-loops-then-ship`, `#sessions-passes-and-escalation` and `ai-sdlc.md#reports-and-one-line-returns` resolve.
- Wording matches C4: line shape, suffix order, `Rejected <note-id>:` in the commit body, `--allow-empty`, `base=` continuation, routing by `reason=`, `fix-<N>` role dropped.
- Not reviewed: doc-update-map row 62 for the PR body is the implementer's to list; the report does not name it.