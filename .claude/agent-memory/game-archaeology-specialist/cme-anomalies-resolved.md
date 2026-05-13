---
name: cme-anomalies-resolved
description: Three CME EventSignal architectural anomalies resolved in W-anom session 5 — BM Pattern B, GiveInventory server-only, SGWHomeless editor class
metadata:
  type: project
---

Resolved 2026-05-13 by live decompile via Ghidra MCP.

**Anomaly 1 — Black Market (BM emitters):**
All 4 BM emitters (`0x00e59970`, `0x00e59c70`, `0x00e59da0`, `0x00e59f70`) use Pattern B
from `cme-event-signal.md`. Not a third mechanism. They call `thunk_FUN_0054c900` (lazy
CME system singleton at `DAT_01ee2678`) then dispatch via `(*this+8)(system, 1)` (vtable slot 2
of typed event object). Pattern B ctors at `0x00e5c1a0`/`0x00e5c440`/`0x00e5c6e0`/`0x00e5c980`.

**Why:** `CallbackImpl` never exists for Pattern B — the anomaly was that prior sessions didn't
recognize BM as Pattern B.

**Anomaly 2 — GiveInventory:**
`Event_NetOut_GiveInventory` (`0x00d97750`/`0x00d97830`) has a TypedEmitInfo but zero client
subscribers. It is a server-side-only GM tool signal — the client defines the signal infrastructure
but no handler is registered. The actual GiveInventory path is `Event_SlashCmd_GiveInventory`
which has full `CallbackImpl__vfunc_2` at `0x00c964d0` bound to `SGWTextCommandMgr`.

**Anomaly 3 — VSGWHomeless / SGWHomeless:**
Not a catch-all. RTTI confirms class name `class_SGWHomeless`. It is an in-editor developer tool
manager: a static singleton (`DAT_01ef2380`) that subscribes to 22 `Editor_*` events via
`FUN_00d3efb0`, registers as "editor" mode (`FUN_0057b800("editor")`), and provides handlers
for editor viewport manipulation. Some handlers are dev placeholders that open browser URLs:
- `Editor_ViewWireframe` → `http://www.stargateworlds.com/`
- `Editor_ShadowStats` → `http://beta.stargateworlds.com/`

RTTI accessor cluster: `0x00d40740`–`0x00d415c0` (30 functions, 0x80 spacing).

W5-C checkpoint addresses `0x00d3daXX` were mid-function addresses inside SGWPIEScriptManager
subscription helpers — not SGWHomeless entry points. The W5-C brief was incorrect about those
being VSGWHomeless function start addresses.

**Why:** The "homeless" name was the developers' term for events without a dedicated screen-level
handler; SGWHomeless bundles them. It compiled into the release binary because SGW shipped with
the UE3 editor subsystem partially intact.

**Key files:**
- `docs/reverse-engineering/findings/architectural-anomalies.md` — full findings
- `docs/reverse-engineering/address-map.md` — "Architectural anomalies" subsection added
- `docs/reverse-engineering/v5-campaign/worker-anom.checkpoint.json` — checkpoint
