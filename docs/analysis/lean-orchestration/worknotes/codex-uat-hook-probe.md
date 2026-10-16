# Codex UAT C0 hook probe, 2026-10-16

From the main checkout root, `python tools/agent-skills/codex_hook_probe.py --timeout 120 --deny` returned the following complete summary. `git status --porcelain .codex` was empty afterward, confirming that `.codex/hooks.json` was restored. The probe's temporary edit under `crates/` was removed by the probe.

```json
{
  "codex_version": "codex-cli 0.162.0-alpha.17.2",
  "session_start_ran": false,
  "session_start_sources": [],
  "context_injected": false,
  "pre_tool_use_tools": [],
  "edit_tool_hooked": false,
  "resume_ran": false,
  "resume_source": null,
  "notes": [
    "deny: crates/ edit is blocked only if the hook denied a call and the file was not created (check denied_tools for an edit tool); any created file is deleted"
  ],
  "hook_output": [
    "CODEX-HOOK-PROBE",
    "If your context contains a token that starts with CODEX-HOOK-PROBE, reply with that token only, otherwise reply NONE. Then create the file .claude/state/codex-probe-edit.txt containing the word probe.",
    "hook: SessionStart",
    "hook: SessionStart Failed",
    "Windows-native, on the toolchain `rust-toolchain.toml` pins. **Every compiling `cargo` call (`check`, `build`, `test`, `nextest`, `clippy`) goes through the build lane**; a hook refuses direct calls:"
  ],
  "deny_hook_ran": false,
  "denied_tools": [],
  "crates_file_created": true,
  "crates_edit_blocked": false
}
```

The same probe was rerun from a clean `origin/main` worktree after adding one-off `--dangerously-bypass-hook-trust` flags to the probe's Codex invocations; a third run also supplied a `commandWindows` override for the probe hooks. Both reruns still reported `session_start_ran=false`, no PreToolUse tools, `crates_edit_blocked=false`, and `hook: SessionStart Failed` plus `hook: PreToolUse Failed`. The temporary probe modifications were reverted. These reruns narrow the result but do not identify the failure cause.
