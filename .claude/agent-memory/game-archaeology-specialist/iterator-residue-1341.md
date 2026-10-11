---
name: iterator-residue-1341
description: 2026-10-10 #1341 dig: the processOrderedPacket iterator residue is sticky per process, only the first packet of a chain is exposed, phase-1 record math, and the decisive stock-vs-hook probe
metadata:
  type: project
---

Findings doc section: `docs/reverse-engineering/findings/client-mercury-receive-path.md` ("The iterator's next-request offset is never initialized").

- Residue slot = `processOrderedPacket` `ESP+0x64` = entry_ESP-0x48; sole caller is the `ClientIncomingMessage` work-item thunk `0x0158d3f0` (queue runs siblings `Nub::send` item `0x0158d3e0`, channel-reg `0x0158d400` at the same depth).
- 27 `request_misparse` rows in SigNoz; 11 distinct residues, all multiples of 8, constant per client process for hours (752 in two processes). Selection bias: only residues that equal a cursor are ever visible.
- `Bundle::iterator::next` `0x01579cd0` rewrites `+0x14 = nextpacket+0x30` on each chain crossing, so only a bundle's FIRST packet is exposed. Server rule: one message per single-packet bundle; one message in the first fragment of a chain.
- Phase-1 flush record = 11 (create) + 26 (avatar) = 37 bytes; cursor 752 = avatar of record 20 (12+37*20); cursor 408 (2026-10-05 event) = create of record 11.
- NOT settled: whether an uninstrumented client has the residue. Telemetry detours (Nub::send, Channel::send etc.) are default-on and a Rust detour frame is a proven residue source (value 1 in the 2026-09-29 bisect). Probe: non-freezing x64dbg BP at `0x0157c9dc` logging `[esp+0x64]` on a no-DLL client vs default.
- Cooked version: signed compare IS real, in UpdateFromSourceArchives 0x00479fa0 (CMP/JG at 0x0047a379): bundled stock version > cache version (signed) copies the stock PAK over the cache. Server versions >= 2^31 are wiped every launch. 216/217 are two archives (bundled vs cache), not duplicate entries.
- `docs/protocol/message-dispatch-table.md` lists msg 0 authenticate as DWORD_LENGTH; live telemetry says width 2 (elem_style 1 param 2). Re-check that row.
- Headless Ghidra worked with the lab clients running (project is opened readOnly; no lock clash).
