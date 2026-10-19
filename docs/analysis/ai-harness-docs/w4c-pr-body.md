## Summary

Every agent with a shell now picks it from the `Platform:` line of its environment (ai-harness-docs W4c: AH-25, AH-26 and AH-27; D-AH53). The owner decided on 2026-10-18 that the packet workers and the build lane get a Bash path, but only on Linux and WSL; on Windows the default stays PowerShell.

| Platform line | Shell |
|---|---|
| `win32` | PowerShell and the `.ps1` scripts; never Bash, not even Git Bash |
| `linux` (WSL reports `linux`) | Bash and the `.sh` twins, with the same arguments |
| none | as `win32` |
| anything else | `blocked: platform <p> is not supported (Windows, Linux and WSL only)` |

Every packet now carries a `Host:` line (`windows` by default, `linux`, or `both`). A coder whose platform does not match returns `blocked`, and it never translates a command between shells, except the lane: `lane.sh` and `lane.ps1` take the same arguments.

**Owner-visible:** this changes `packet-coder`, `packet-reviewer` and `architect`. A running Claude Code session keeps the old agent text: relaunch after merge.

## What changed

- **Agents (AH-25):** packet-coder, packet-reviewer and architect gain the Bash tool and the platform rule; the two domain agents that named only `lane.sh` now name both lanes. New `tools/agent-skills/test_platform_rule.py`.
- **Skills and AGENTS.md (AH-26):** lane-build gains § Linux and WSL hosts; campaign-packet gains the `Host:` bullet and a platform-keyed worker rule; AGENTS.md names the `.sh` twins.
- **Docs (AH-27):** development-workflow § WSL and Linux hosts, rules-and-gotchas, ai-sdlc § Platforms and the README's platform line say the same thing.

## Tests

- `python -m unittest discover -s tools/agent-skills -p "test_*.py"`: OK. `test_platform_rule.py` fails on the agent files as they were on `main` (revert proof in the packet worknote).
- `python tools/agent-skills/sync.py --check`: exit 0.

**Not tested:**

- No agent session has run on Linux or WSL. The status stays "not validated": AH-15's Linux leg is the first run that would test it.
- The live-DB scripts have not been run on Linux.
- Codex: its hook matcher is `Bash` only, and whether its environment carries a `Platform:` line is not verified.

## Docs (doc-update-map rows)

- **AI-harness wiring:** agent frontmatter and rules, skills, AGENTS.md, the README platform line and the workflow prose.
- **AI-harness configuration:** the rules-and-gotchas line.
- **The pre-PR checklist, build commands, or repo invariants:** lane-build § Linux and WSL hosts. `.github/copilot-instructions.md` is not changed: it is not an agent of this harness.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01WCmH1WPLJdizxaciRv4aJM
