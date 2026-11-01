# HT-06 tests review 1

Range e1de6742f..b334964f7. Method: copied the hook, `settings.json`, `.codex` and `tools/agent-skills` to a scratch tree, applied one mutation at a time (the files are CRLF, so multi-line patterns were converted), and ran `test_report_contract` plus `test_hooks.WiringTest`. Baseline: OK. Nothing in the worktree was edited. Scripts: `scratchpad/mut.py`.

## Findings

Verdict on the revert proofs: real, not theatre. The spec's "fails on revert" claim holds.

- T1. Hook reverted or no-op (`reason = None` in `main`): killed by 6+ tests, including the spec's named cases 1, 3, 4, 5 and 13. With the hook file absent, the subprocess exits non-zero and every case fails. Wiring reverted (SubagentStop removed, wrong script, `py.sh` dropped, `|| exit 0` dropped, event renamed to `Stop`): each killed by `test_settings_wires_report_contract_on_subagent_stop`. A direct `; python` call and a dropped `py.sh` are also killed by the widened `test_every_python_hook_goes_through_the_launcher`.
- T2. Report mode is pinned. Killed: mode forced to line (many tests), mtime check removed, check C removed, check B removed, `-` treated as matching, A reason without the path, backtick strip removed, buried-line scan limited to first or last line, the transcript last-text fallback removed, the derived transcript path removed, `candidate -> paths[0]` and `candidate -> stale[0]` (only case 12 catches these).
- T3. Line mode is pinned. Killed: mode forced to report, mode forced to line, `==` instead of `in` for haiku (case 17), L reason losing "Write no file". Cases 13/14/15 each fail under the matching mode flip.
- T4. Cap exemption is pinned in both modes. Cap check removed, marker changed, cap applied only in line mode (reviewer test fails) or only in report mode (coder test fails): all killed. The cap fixtures use a `tool_result` record as the spec requires.
- T5. `stop_hook_active` removal is killed by `test_stop_hook_active_passes`.
- T6. Fail-open: only malformed JSON is tested. `except Exception: raise` is killed. `except ValueError` only, survives (T9). A block exiting 2 and a changed `decision` key are killed.
- T7. Survivors that are equivalent or harmless, no action: non-dict payload guard removed (the broad `except` still returns 0); the "no brief" early exit removed (an empty brief gives no paths, same result); `errors="replace"` -> strict (the fixtures are ASCII); the settings `timeout` removed or set to 1, `matcher` added, `type` changed (the wiring test does not look at them).
- T8. Survivors that are real gaps in spec'd behavior, none hit by cases 1-20:
  - `candidate = paths[-1]` instead of "last stale path" survives. Case 12 only has the stale path last. Add a pass-2 brief whose last path is fresh and an earlier one stale, with no line, and assert the reason names the stale one.
  - `target = matched[-1]` vs `matched[0]` survives (two brief paths with the same suffix are never used).
  - Slack of 5 s is unpinned: `0` and `1e8` both survive. Add one report with an mtime 3 s before the first timestamp (passes) and one 30 s before (blocks C).
  - Empty-file check (`st_size == 0`) removed survives. The spec says "missing, empty or older"; add a zero-byte report and expect C.
  - Relative path resolution: "cwd only", "root only", "root first", "root ignores `CLAUDE_PROJECT_DIR`" and "cwd ignored" all survive. Case 11 runs with `project_env=False` and `cwd == root`, so the cwd/root order cannot be seen. Use a `cwd` different from the root with the report only under one of them.
  - An explicit `agent_transcript_path` is never exercised (only the derived path). Removing it survives.
  - "Brief is the first user text" and "first timestamp" survive (fixtures have one user record and one timestamp). Add a transcript with a second user record and a later timestamp.
  - `## Reporting` gating survives: the only no-op agent test is `Explore`, which has no agent file. Add an agent file without `## Reporting` and expect no output.
  - Haiku match is case-sensitive: `.lower()` removal survives. Add `model: Haiku`.
- T9. Fail-open covers only unparseable stdin. An exception inside `contract_reason` (unreadable agent file, bad transcript) is untested; `except ValueError` only would pass. Add a payload that raises inside (for example `agent_type` with an agent file that is a directory, or `cwd` of the wrong type) and assert exit 0 with no output.
- T10. Test quality otherwise good: the tests run the hook as a subprocess with an exact decoded `decision == "block"`, assert on reason fragments (`no return line`, the path, `Write no file`, `assertNotIn(".claude/reports")` for L), and cases 1-20 are all present and map one to one. Case 8/9 cover the transcript fallback and the derivation.
- T11. `WiringTest` change: the new test checks the event, the script name, `py.sh` and `|| exit 0`. The widened launcher test now walks every event and every agent frontmatter, and is killed by the `py.sh` drop. It does not pin `timeout` (10 s, spec'd).

## Fix notes

None blocking. Optional hardening, in order of value: T8 candidate selection, T8 cwd/root, T8 slack and empty file, T9 internal-exception fail-open, then the rest of T8.

## Not checked

- No cargo; the hook was not run in a live Claude Code session. SubagentStop payload shape (`agent_transcript_path`, `last_assistant_message`) is assumed from the spec.
- The other checks in the packet (`sync.py --check`, `test_harness_tables.py`, full `test_*.py` discover) were not run here.
- CRLF of the new files and the 250-line cap (the hook is 217 lines) were not audited beyond reading.
- LINE and RP verbatim equality with the C1 copy in `tools/token-profile` (HT-09 owns it).

Verdict: ship
