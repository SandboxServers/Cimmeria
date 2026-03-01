# Function Naming Progress

> **Last updated**: 2026-03-01
> **Binary**: SGW.exe (32-bit x86 PE, MSVC)
> **Total functions in Ghidra**: ~85,000

---

## Overview

This document tracks progress on naming functions in the SGW.exe binary using the 10 Ghidra annotation scripts and manual analysis.

## Script Results

Run scripts in order (01-10). Later scripts depend on names established by earlier ones.

| # | Script | Target | Expected Yield | Actual Yield | Status |
|---|--------|--------|----------------|--------------|--------|
| 01 | rtti_annotator.py | RTTI `.?AV...@@` → vtable/class | 200-400 | **9,700 RTTI, 8,961 vtables, 4,364 vfuncs** | DONE |
| 02 | ue3_exec_annotator.py | `exec*` strings → UE3 natives | 500-1000 | — | NOT RUN |
| 03 | bigworld_source_annotator.py | Source paths → BW functions | 100-300 | — | NOT RUN |
| 04 | event_signal_annotator.py | Event_NetOut/NetIn → handlers | ~420 | — | NOT RUN |
| 05 | mercury_annotator.py | Mercury:: debug strings | 30-50 | — | NOT RUN |
| 06 | cme_framework_annotator.py | CME:: symbols | 50-100 | — | NOT RUN |
| 07 | vtable_annotator.py | Named vtables → vfunc_N | 2000-5000 | — | NOT RUN |
| 08 | lua_binding_annotator.py | toLua registration | 100-200 | — | NOT RUN |
| 09 | string_discovery.py | Broad `class::method` strings | 500-2000 | — | NOT RUN |
| 10 | xref_propagation.py | Call graph inference | 1000-3000 | — | NOT RUN |

**Expected total**: 4,000-12,000 functions (~5-14% of all functions, ~40-60% of server-relevant)

## Manual Naming

Functions named through manual Ghidra analysis (decompilation, cross-referencing).

| Date | System | Functions Named | Notes |
|------|--------|-----------------|-------|
| — | — | — | No manual analysis sessions yet |

## Progress Summary

```
Total functions:     ~85,000
Auto-named (scripts): 4,364  (script 01 only so far)
  - vtables labeled:  8,961
  - vfunc_0 renamed:  4,364
  - RTTI classes:     9,700
Manual-named:              0
Total named:           4,364  (5.1%)
Remaining unnamed:   ~80,636
```

**Note**: Script 01 yielded 24x the expected result (9,700 vs 200-400 estimated).
The binary is extremely rich in RTTI — UE3, BigWorld, and CME classes all present.

## Priority Targets

Functions most valuable to name for server implementation:

### Tier 1 — Direct Protocol Impact
- Event_NetOut/NetIn handlers (script 04) — every client-server message
- Entity property serialization — wrong property IDs = crash
- Mercury channel management — connection lifecycle

### Tier 2 — Game System Logic
- Combat damage pipeline functions
- Ability targeting mode validation
- Effect application/removal
- Mission state machine transitions

### Tier 3 — Engine Understanding
- BigWorld entity lifecycle (create, destroy, migrate)
- Space management and cell boundaries
- Area of Interest calculations

## How to Update

After running a script:
1. Record the actual yield in the table above
2. Update the progress summary counts
3. Note any issues or unexpected results
4. Spot-check 10 named functions for accuracy

After manual analysis:
1. Add a row to the Manual Naming table
2. Update progress summary
3. Document key findings in the relevant system doc under `docs/gameplay/` or `docs/protocol/`
