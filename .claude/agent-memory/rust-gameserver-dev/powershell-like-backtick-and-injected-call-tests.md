---
name: powershell-like-backtick-and-injected-call-tests
description: PowerShell `-like` treats a backtick as an escape, so a pattern quoting `lab start` never matches; lab CLI commands test best with the MCP call injected as a scriptblock
metadata:
  type: reference
---

- **`-like` and backticks.** In a `-like` wildcard pattern the backtick is the escape character, even in a single-quoted string: `'*start it with `lab start`*'` does not match the text it spells. Use `.Contains(...)` or `-match [regex]::Escape(...)` for messages that quote commands (LR-08 `test-record.ps1`, 2026-10-11).
- **Testing a lab CLI command without a daemon.** Put the whole action in the `*-lib.ps1` as a function taking the tool call (`& $Call <tool> <args> <timeout>`, which throws `"<tool> failed: <text>"` like `uat-lib.ps1`'s `Invoke-McpTool`) and the daemon probe as scriptblocks; the command script only wires `New-McpSession`/`Invoke-McpTool` and prints. The stub can then script replies and throws per call. A child `pwsh -File <cmd>.ps1` run is safe only for paths that exit before the probe (usage errors), since a child sees the real User-scope daemon token. See [[lab-cli-child-test-daemon-isolation]].
- The `Call` scriptblock in the command script reads `$url`/`$token` by dynamic scope from the function that built it; no `GetNewClosure` needed.
