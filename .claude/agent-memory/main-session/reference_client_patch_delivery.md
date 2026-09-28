---
name: reference_client_patch_delivery
description: How client changes reach players — LoginInternal.lua (not .rdata) picks the server, ASLR is one PE byte, patch sets are deltas against the player's stock files; what ships and what deliberately doesn't
metadata:
  type: reference
---

Learned 2026-09-27 while diffing a known-good QA client against the stock archive.org
install (full hash comparison, 5,983 stock files) and building `cimmeria-patchset`.

- **Server selection is `LoginInternal.lua`**, at
  `Working\SGWGame\Content\UI\Startup\Login\`, defining `LoginMod.loadServerSystems()`
  (`name -> http://host:8081`). The stock file points at CME's QA/production servers.
  The old launcher `.rdata` "hostname patch" was wrong: the only ASCII
  `www.stargateworlds.com` in SGW.exe is inside the SOAP namespace
  `http://www.stargateworlds.com/xml/sgwlogin`; the auth server's requests carry that
  namespace. Removed in the patch-set PR (client-launch `patch_rdata.rs` deleted).
- **The known-good SGW.exe differs from stock by one byte**: 0x186 (DllCharacteristics
  low byte, e_lfanew 0x128), DYNAMIC_BASE cleared = ASLR off. The client-patches DLL
  assumes image base 0x00400000. The launcher's `client_setup::aslr` reproduces the
  known-good exe byte for byte. The bootstrap's remote-desktop JZ->JMP patch is NOT in
  the known-good exe.
- **`SGWLogConfig.xml` is read by SGW.exe itself** (UTF-16 string + log4cxx
  DOMConfigurator imports), not by AtreaLoader. Stock ships none, so no
  `SGWDebugLog.log`. We ship our own version (004-log-config).
- **Patch sets** (`data/client-patches/`): bsdiff deltas against the player's stock
  files, never CME bytes. `upk_normalize` (open + finish through cimmeria-upk's patcher)
  turns the 394 KB raw delta for a `upk_patch`-built map into 912 bytes.
  Shipped: 001 dialog portraits, 002 castle ring transport, 003 the three merged PAKs
  (+ CookedBehaviorEvents stub; identical to `data/cache/`), 004 log config.
- **Owner decision: the "super build" extras stay out of the launcher**: the 2008
  content the public builds dropped (Maps\Agnos, Agnos_Library, derek.upk,
  StrAegis.upk, Wep_AG.upk; nothing in server data or other packages references them;
  the Straegis pet uses stock MOB_StraegisFighter) and the historical CellBlock worlds
  (GM-only destinations). Both are safe to lack unless a player loads those maps.
- Undecided as of 2026-09-27: `eula.lua` 19 s login delay, `prp_gen.fev/.fsb` sound
  bank (unknown provenance; holds the ring-transport FMOD event).

Related: [[reference_client_rar_and_cache_tiers]].
