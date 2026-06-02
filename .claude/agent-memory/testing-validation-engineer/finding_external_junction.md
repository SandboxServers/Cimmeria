---
name: finding-external-junction
description: Fresh worktrees lack external/ — junction-link from the main repo before any cargo test that pulls in detour/recast/SDL
metadata:
  type: reference
---

`external/` is not in git (populated by `setup.ps1`). Newly-created worktrees fail to compile crates with C++ deps (cimmeria-entity → recast/detour) until the junction is created.

**PowerShell incantation that works on Windows 11:**

```powershell
New-Item -ItemType Junction -Path "<worktree>\external" -Target "C:\Users\Steve\source\projects\Cimmeria\external"
```

**Why not `cmd /c mklink /J`:** the cmd subshell output goes to stdout but the working-directory state is lost between Bash tool calls, so when I ran it inside the worktree I couldn't observe what happened. PowerShell's `New-Item -ItemType Junction` is the more reliable form.

**How to apply:** any time I create a temp audit worktree (e.g. `audit-prNNN`), follow up with the junction creation before kicking off any compile. cimmeria-services tests fail differently — they don't need `external/` directly but they transitively depend on cimmeria-entity which DOES.
