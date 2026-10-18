# Hook timing: py.sh launcher (AH-02)

Date: 2026-10-18.

Machine: Windows 11 Pro for Workstations (10.0.26300), PowerShell 7.
Git Bash: `GNU bash, version 5.2.37(1)-release (x86_64-pc-msys)`, from `Git\bin\bash.exe` (`bash --version`, first line).
Python: `Python 3.13.0` (`python --version`).

Method: `tools/agent-skills/measure_hooks.ps1` runs each command through Git Bash with a JSON payload on stdin, after 3 warmup runs, then 30 timed runs. Each pair ran twice, in the order before, after, before, after. The table averages the two rounds' medians.

Payloads:

- shell_guard: `{"tool_name":"PowerShell","tool_input":{"command":"git status"},"cwd":"<worktree>"}`
- resume_card: `{"hook_event_name":"SessionStart","source":"compact"}`

Raw results (one `measure_hooks.ps1` JSON object per run, times in ms):

```
round=1 label=bare-python {"runs":30,"median_ms":70.8,"p90_ms":84.9,"min_ms":64.0,"exit":0}
round=1 label=shell_guard before {"runs":30,"median_ms":100.0,"p90_ms":118.4,"min_ms":83.1,"exit":0}
round=1 label=shell_guard after {"runs":30,"median_ms":90.2,"p90_ms":97.3,"min_ms":86.0,"exit":0}
round=1 label=resume_card before {"runs":30,"median_ms":114.0,"p90_ms":150.0,"min_ms":94.5,"exit":0}
round=1 label=resume_card after {"runs":30,"median_ms":119.7,"p90_ms":128.7,"min_ms":104.1,"exit":0}
round=2 label=bare-python {"runs":30,"median_ms":70.3,"p90_ms":87.0,"min_ms":63.7,"exit":0}
round=2 label=shell_guard before {"runs":30,"median_ms":96.9,"p90_ms":119.5,"min_ms":89.1,"exit":0}
round=2 label=shell_guard after {"runs":30,"median_ms":101.3,"p90_ms":113.4,"min_ms":92.1,"exit":0}
round=2 label=resume_card before {"runs":30,"median_ms":114.9,"p90_ms":133.2,"min_ms":99.4,"exit":0}
round=2 label=resume_card after {"runs":30,"median_ms":109.2,"p90_ms":127.8,"min_ms":92.6,"exit":0}
```

The bare `python -c pass` baseline averages 70.6 ms per run, so the interpreter start is most of each hook's cost.

| Hook | Median before (ms) | Median after (ms) | Added (ms) | Added (%) |
|---|---|---|---|---|
| shell_guard | 98.5 | 95.8 | -2.7 | -2.7% |
| resume_card | 114.5 | 114.5 | 0.0 | 0.0% |

The "before" commands are the current wiring. The "after" commands are the C2 shapes that AH-03 installs, with `py.sh` in place of the bare `python`. Every run exited 0.

Verdict: go
