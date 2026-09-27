---
name: project-shadow-lighting-handoff
description: "External shadow/character-lighting handoff (2026-09-21) reviewed, not applied: a client-only ini/XML freeze; shadow resolution is capped at 1014 and is not the lever for the Human Female face defect; next target is ApplyToPawn's light-environment path"
metadata:
  type: project
---

On 2026-09-21 an external handoff (`SGW_SHADOW_HANDOFF_2026-09-21.zip`) arrived with a Claude-facing analysis, a repro guide, apply and verify PowerShell scripts, and a manifest. It was reviewed only; nothing was imported or applied to a client.

**What it is:** a freeze of client-side render config, not a fix, and it has no server surface:

- `BaseEngine.ini`: `ShadowFilterQuality=2`, `ShadowFilterRadius=3.5`, `MaxShadowResolution=2048`, VSM and Branching PCF off, `DepthBias=.012`.
- `GameplayGame.ini`: `UseLightEnvironment=true`.
- `SavedSystemOptions`: `ShaderModel=0`, `tqCharacter=1`, `allowDynamicShadows=true`, max/min shadow resolution 2048/32.

The open defect is the Human Female's dark, mask-like face and over-bright white sleeves. The pack itself concludes it's the appearance-compositing / light-environment path, not shadow-map resolution.

**Follow-up RE:** `docs/reverse-engineering/findings/render-thread-options.md` (PR #836) shows:

- the `max`/`minShadowResolution` system options are dead;
- the shadow depth buffer is a fixed 1024, so `MaxShadowResolution=2048` behaves like 1024 (ceiling 1014);
- `ShaderModel` is startup-only.

So shadow resolution is not the lever. The next real target is LightEnvironment/ShadowParent handling around `CompositedAppearanceProxy::ApplyToPawn` (`0x00ec0840`), which no doc covers yet.

**Pack's standing don'ts** (all tested and rejected): no `UseLightEnvironment=false`, no `DepthBias=.018`, no `allowDynamicShadows=false`, and no in-game console diagnostics for this task.

**If the apply script is ever run:**

- It keys off the current directory, which must be the client's `Working` folder.
- It backs up four files with timestamp suffixes.
- It round-trips both XML files through `[xml].Save()`, which rewrites `SystemOptions.xml`'s single-quoted attributes to double quotes and re-indents it. That's cosmetic.
