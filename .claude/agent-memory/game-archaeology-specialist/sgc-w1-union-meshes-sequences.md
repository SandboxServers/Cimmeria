---
name: sgc-w1-union-meshes-sequences
description: US-02 (2026-10-18) SGC_W1 Union facts - Failsafe/ViewScreen02/chair/crate mesh exports, name_id fallbacks, sequence 10047 = 6100 MakeGate and 10060 = 6113 CrossGate
metadata:
  type: project
---

**Facts (2026-10-18, client read only, CookedPC `Maps\SGC_W1` and `Packages\Em-Props.upk`):**
- `Em-Props.upk` holds export names `EM-SGC_Failsafe00`, `EM-ViewScreen02`, `EM-Office_Chair00`, `EM-SupplyCrate00/01/02` (string scan, package link inferred).
- `texts.sql` 10726 `DN_Ms_A01_SGC2_L_Failsafe` = "Failsafe". No "Supply Crate" or "Computer Terminal" moniker exists; 8369 ("Stargate Control") is the fallback.
- Sequences 10047 (event 6100, `Stargate_MakeGate`) and 10060 (event 6113, `Stargate_CrossGate`) bracket the server-playable dial moment. Chevrons 6106-6112 are client-driven (NA35 §1.4). The Python Kismet decoder cannot read `SeqEvent_Stargate` properties (unpack error at offset 598), so gate 27 binding is inferred.

**Where it is written up:** `docs/analysis/union-sgc-continuation/meshes.md`.