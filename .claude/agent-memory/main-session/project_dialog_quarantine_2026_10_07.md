---
name: project-dialog-quarantine-2026-10-07
description: "Stock dialog PAK census weakens three candidate causes of the 2026-09-27 dialog quarantine."
metadata:
  type: project
---

Read-only census of committed `data/cache/CookedDataDialogs.pak` (2026-10-07): 1,920 dialogs have multiple screens, 361 buttons use type 4, ten dialogs contain SpeakerID 754, and two contain SpeakerID 843. The highest ScreenID found was 120383; authored 200000-range screens are outside that observed range. These facts and the earlier headless-Ghidra tracing are reconciled in `docs/reverse-engineering/findings/cooked-dialog-override-crash.md`. The crash cause remains unproven; the next useful step is controlled client reproduction with clean and affected caches, not another static map-load trace.
