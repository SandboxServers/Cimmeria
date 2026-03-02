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
| 02 | ue3_exec_annotator.py | `int*` strings → UE3 natives | 500-1000 | **1,275 int* found, 1,006 renamed** | DONE |
| 03 | bigworld_source_annotator.py | Source paths → BW functions | 100-300 | **17 paths + 17 Mercury, 23 renamed** | DONE |
| 04 | event_signal_annotator.py | Event_NetOut/NetIn → handlers | ~420 | **975 events (479 out + 496 in), 419 renamed** | DONE |
| 05 | mercury_annotator.py | Mercury:: debug strings | 30-50 | **120 strings, 38 renamed, 79 RTTI vtables** | DONE |
| 06 | cme_framework_annotator.py | CME:: symbols | 50-100 | **42 strings, 28 renamed, 5 classes** | DONE |
| 07 | vtable_annotator.py | Named vtables → vfunc_N | 2000-5000 | **~13,970 total vfuncs (~9,600 new)** | DONE (partial, cancelled) |
| 08 | lua_binding_annotator.py | toLua registration | 100-200 | **72 Lua strings, 0 renamed (no bindings)** | DONE |
| 09 | string_discovery.py | Broad `class::method` strings | 500-2000 | **1,310 `::` strings, 1,364 renamed, 143 classes** | DONE |
| 10 | xref_propagation.py | Call graph inference | 1000-3000 | **3,333 renamed (3 passes), 226 classes** | DONE |

**Expected total**: 4,000-12,000 functions (~5-14% of all functions, ~40-60% of server-relevant)
**Actual total**: 101,909 named (60.6% of 168,239 non-thunk functions)

## Manual Naming

Functions named through manual Ghidra analysis (decompilation, cross-referencing).

| Date | System | Functions Named | Notes |
|------|--------|-----------------|-------|
| — | — | — | No manual analysis sessions yet |

## Progress Summary

```
Total functions:     ~85,000
Auto-named (scripts): ~15,478  (scripts 01-07)
  Script 01 (RTTI):
    - RTTI classes:     9,700
    - vtables labeled:  8,961
    - vfunc_0 renamed:  4,364
  Script 02 (UE3):
    - int* strings:     1,275
    - functions renamed: 1,006
  Script 03 (BW Source + Mercury):
    - BW paths:         17 (3 unique files)
    - Mercury strings:  17
    - functions renamed: 23
  Script 04 (Event Signals):
    - events found:     975 (479 out + 496 in)
    - funcs renamed:    419
  Script 05 (Mercury):
    - Mercury strings:  120
    - RTTI vtables:     79
    - funcs renamed:    38
  Script 06 (CME):
    - CME strings:      42
    - CME classes:      5
    - funcs renamed:    28
  Script 07 (VTable propagation — partial):
    - ~13,970 total vfunc entries
    - ~9,600 new (beyond script 01)
    - cancelled partway through U*Factory classes
  Script 08 (Lua bindings):
    - 72 Lua strings, 0 renamed
    - No structured binding tables (Lua vestigial)
  Script 09 (String discovery):
    - 1,310 `::` strings, 143 unique classes
    - functions renamed: 1,364
  Script 10 (Xref propagation — LOW confidence):
    - 3 passes: 713 + 1,350 + 1,270
    - functions renamed: 3,333
    - unique classes inferred: 226
Manual-named:              0

Ghidra final tally (from script 10 setup):
  Total non-thunk functions: 168,239
  Named after all scripts:   101,909 (60.6%)
  Unnamed remaining:          66,330 (39.4%)
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
