---
name: name-flags-none-property-list
description: Client stores a package name entry without RF_LoadForClient as FName None (name-table loader 0x4bad20), which silently ends tagged property lists; cause of the 2026-10-05 client patch 010 freeze (AV at SGW.exe 0x4bc6a0); evidence, guards and method
metadata:
  type: project
---

**Fact.** The client's name-table loader, `FUN_004bad20` (the "serializing name
map" step of the linker tick `FUN_004beca0`), ANDs each name entry's flags with the
linker's 64-bit load-context mask (linker+0xe8) and stores FName `(0,0)` (= `None`)
when nothing is left. Load bits: `0x0007_0000_0000_0000`, client is
`0x0001_0000_0000_0000`; the usual entry value is `0x0007_0010_0000_0000`. The FName
reader `0x4bc660` (`ULinkerLoad::operator<<(FName&)`) does NOT test flags: it reads an
int, indexes the NameMap with no bounds check, treats `(0,0)` as None and still
consumes the 4-byte Number. A tagged-list tag named by a non-client entry therefore
reads as `None` and ends the list; the rest of the object is read from the wrong
offset. Signature: FName index like `0xB5000000` (the `None` index 0xB5 shifted 3
bytes), out of range, AV at `0x4bc6a0` (`cmp [names+idx*8],0`), then the main thread
hangs. Tag reader `0x4b6a60`, tag loop `0x4b56e0`, async package steps `0x4c8060`
(`UnAsyncLoading.cpp`). A nested `None` ending the outer list is inferred from the
captures and the counts below, not from a decompile of the struct tag loop.

**Why (the incident).** Client patch 010 cloned Castle's ring rig into Ihpet; the
cloned `LightingChannels` struct names `Dynamic`, editor-only in Ihpet's table
(`0x0004_0010_0000_0000`) but client-loadable in Castle's (`0x0007_0010_0000_0000`).
Fixed in 011 (`PatchSession::ensure_name_with_flags` adds a second, loadable entry;
no flag edit). A second entry for one string is fine: flags are per linker entry and
both map to the same global FName.

**Evidence that it is the whole story.** Client-loaded exports (object flags with
`RF_LoadForClient`) naming an entry the client does not load: 48 of 765 in 010's chunk
(40 InterpActor + 8 ParticleSystemComponent components, depth 1 inside
`LightingChannels`), 0 in 011's, 0 in stock Castle (1,571), 0 in stock Ihpet (237),
0 in 007's Castle map. Stock packages DO name editor-only entries (90 tags in stock
Castle, 3 in stock Ihpet) but only on objects without `RF_LoadForClient` (`Brush`
0x0234_0001_..., `DrawLight*Component` 0x0004_0001_..., `InterpCurveEdSetup`
0x0034_...), which the client never serializes. So `upk_patch audit-names <pkg>` is
empty on any unmodified package and is a self-test.

**How to apply.** Any cross-package object clone (upk_patch) or hand-built package
must carry load bits per name; a structural parse can't see this. `clone-objects`
audits its new objects (fails on a hit or an unwalkable object).

**Method that worked (reuse).** Bisect by cloning single roots with `upk_patch` and
having a lab agent load each in the client (OK: ring base mesh actor, Kismet only;
AV: InterpActor, Emitter), then an x32dbg first-chance capture of the AV (names.Num
identifies the linker: 349 = Ihpet + rig names) and a Ghidra decompile of the call
chain. Telemetry `client.os.exception` (address, rva) proved the failure was
010-specific (no other hit in 30 days). My first attribution of the flag test to
0x4bc660 was wrong (found in review); read the name-table loader, not just the AV
site. See [[headless-ghidra-decompile-workaround]].
