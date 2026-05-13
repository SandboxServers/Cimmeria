---
name: project-annotation-sweep-s3
description: W-rename campaign COMPLETE — all 489 OnEvent_* MemberCallback RTTI accessor functions renamed to MemberCallbackRtti_* in SGW.exe. 0 rejections. Verification confirmed 0 OnEvent_ remain.
metadata:
  type: project
---

## Session-3 annotation sweep (2026-05-13)

Swept three clusters for annotation-script naming bugs in SGW.exe:

**Cluster 1 (CallbackImpl__vfunc_2):** No mismatches. Clean.

**Cluster 2 (TypedEmitInfo__vfunc_0):** No mismatches. Clean.

**Cluster 3 (SGWNetworkManager MemberCallback, 0x00d44d60+):** 20 initial corrections applied (mangled
template names at 0x00d46ce0–0x00d47660 that had been assigned `OnEvent_NetOut_*__SGWNetworkManager`
rather than the correct mangled template form).

## W-rename campaign (2026-05-13, COMPLETE)

The Session-3 sweep revealed a systemic issue: the annotation scripts named ALL 469 `MemberCallback`
vtable slot-2 RTTI accessors as `OnEvent_<Event>__<Subscriber>`. This incorrectly implies handler
behavior. Slot-2 is a pure RTTI accessor returning `TypeDescriptor*`.

**W-rename campaign scope:** 489 functions total (469 `OnEvent_*` + 20 mangled-name SGWNetworkManager).

**Result:** 489/489 renamed to `MemberCallbackRtti_<Event>__<Subscriber>`. 0 rejections.
NamingConventions.java issues advisory warnings (unrecognized verb, underscores) but is non-blocking.
Verification: `search_functions("OnEvent_")` returned 0 results after campaign.

**Subscriber classes covered:** SGWNetworkManager, SGWScriptedWindow, Detail_CookedKismetEventSetData,
VGameBeing, VCharacterCreation, VSequenceManager, VGameProxyPlayer, VCoverInfo, VGameAppearanceManager,
and others.

**Why:** `OnEvent_` implies a handler. Slot-2 functions are RTTI-only. Future V5 documentation workers
looking at cross-references to these functions need to know immediately they are not dispatch targets.

**Corrected naming schema:**
```
MemberCallbackRtti_<EventName>__<SubscriberClass>
```

**Docs updated:**
- `docs/reverse-engineering/findings/cme-event-signal.md` — "Naming convention correction (Session 3)" subsection
- `docs/reverse-engineering/v5-campaign/WORKER_BRIEF.md` — "Naming conventions" section with slot table
- `docs/reverse-engineering/v5-campaign/worker-rename.checkpoint.json` — final checkpoint (status: COMPLETE)
