## Verdict

decision

Reviewed head b334964f7. Checks 1-4 pass in the worktree (111 tests OK, sync current, 7 harness-table tests OK). The hook matches C3 and C1 (LINE and RP are byte-identical to the work-packets copy); the settings.json wiring and the WiringTest change match the spec.

## Fix notes

N1. tools/agent-skills/test_report_contract.py:test_relative_brief_path_resolves_against_cwd (and run_hook) — case 11 passes with the cwd resolution removed: the subprocess runs with cwd=self.root, which is also the payload cwd and CLAUDE_PROJECT_DIR, so a bare `Path(p)` resolves identically — theatre — make the process cwd differ from the payload cwd: in `run_hook` add the parameter `proc_cwd=None` and change `cwd=str(self.root)` to `cwd=proc_cwd or str(self.root)`; in the test, change the call to `self.run_hook(self.payload(final=GOOD), project_env=False, proc_cwd=tempfile.gettempdir())` — verify: temporarily change `resolve()` in `.claude/hooks/report_contract.py` to `return Path(path.replace("\\", "/"))`; the test must then fail (block C), and pass again after restoring.

N2. tools/agent-skills/test_report_contract.py:ReportModeTest — the explicit `agent_transcript_path` branch of `find_transcript` is never exercised (every `payload()` omits it, so only the derived path is tested; spec case 9 covers the derived path, but the explicit branch is shipped untested) — add to `ReportModeTest`:
    def test_explicit_agent_transcript_path_is_used(self):
        self.brief_transcript()
        explicit = self.root / "elsewhere.jsonl"
        explicit.write_text(json.dumps(user(BRIEF)) + "\n", encoding="utf-8")
        data = self.payload(final=GOOD, agent_transcript_path=str(explicit))
        data["transcript_path"] = str(self.root / "nonexistent.jsonl")
        self.assertIn("is missing, empty or older than this run", self.run_hook(data))
— verify: change `find_transcript` to ignore `agent_transcript_path` (always derive); the new test must fail (derived path is missing, hook exits silently), and pass again after restoring.

## Not fixed

D1 (decision, needs a C3 step 5 change). `.claude/hooks/report_contract.py:70` — the cap check is `CAP_MARKER in raw` over every transcript line, as C3 step 5 words it ("any of its lines contains"). That includes the brief and every `Read`/`Grep` result. `Context cap reached` occurs 21 times in `docs/analysis/harness-tuning/work-packets.md` and in the hook and its test. A subagent that reads any of those files sets `cap=True` and the hook then fails open for that whole dispatch (reviewers of this campaign, and any coder whose brief or Read output quotes C4). The benefit target "no report write < 2%" is lost on those dispatches, silently. Suggested spec change: count the marker only in a `tool_result` block of a `user` record (it is the user-level hook's deny text), which still satisfies tests 16 and 18. Owner/architect call because the spec states the broader rule; no code change was made.

D2 (out of scope, note only). `resolve()` on a Linux or WSL host turns a Windows-style absolute brief path (`C:/...`) into a relative path under cwd, so block C would fire until `stop_hook_active` releases it. Windows is the validated host and the spec is silent; not actionable now.
