---
name: reference_lab_mcp_token_cost_2026_10_10
description: Where lab-driver (Haiku) tokens go, the windowed-launch and native-camera facts, and the stale MCP tool list after a labd restart (2026-10-10)
metadata:
  type: reference
---

Measured 2026-10-10 from lab-driver transcripts (the `usage` fields of each turn):

- **The floor is fixed context, not results.** A 4-call probe (lease, status, player state, release) ended at 37.3k context, 33.9k of it before the first result: Claude Code's subagent prompt plus the agent's tool schemas (27 tools, 28k chars). Tool results added about 3k. Cut turns (composites, `lab_uat_run`) and schemas before trimming results.
- **Inline images are the expensive results.** A 1280x720 capture is over 1k tokens on every later turn; the lab now saves images to `%LOCALAPPDATA%\cimmeria-lab\screenshots` and returns the path unless `image: true`.
- **labd restart does not refresh a session's tool list.** Sessions and their subagents keep the tools fetched before the restart until `/mcp` reconnects `cimmeria-lab` (a lab-driver reported "lab_ensure_in_world is not in my available tool set"). Tracked in #1307.
- **Windowed lab client.** The stock `SystemOptions.xml` defaults `windowedMode` false, so a profile that never saved options opens borderless at the desktop resolution (5120x1440 here) and `PrintWindow` captures black. The supervisor launches `-windowed ResX=1280 ResY=720` (`CIMMERIA_LAB_WINDOW`); the lab profile also has `windowedMode` saved. Window positions saved at the old resolution can leave windows off screen; the lab now moves a slot's host window on screen first.
- **Native camera.** Found by scanning the level's actors for the `ASGWController_Player` vtable (the old WorldInfo+0x35c chain is 0 live); the turn handler does not clamp the stored pitch (it reached 1563 degrees), so the lab clamps pitch turns to +-78.75 degrees. Findings: `docs/reverse-engineering/findings/cegui-mouse-input-feed.md` section 10.

Graduated to `docs/guides/live-research-lab.md` (Fewer calls; The display). Related: [[project_live_research_lab]].
