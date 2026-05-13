# Agent Memory — Game Archaeology Specialist

- [Mercury cipher chain (W-auth)](mercury-cipher-chain.md) — PacketEncrypter vtable, AES-256-CBC+HMAC-MD5 via CryptoPP, zero IV per packet, key derivation confirmed no-KDF.
- [Mercury protocol internals](mercury-protocol-internals.md) — Nub/Bundle/Channel function map, 48 renamed functions, RTTI classes.
- [CME anomalies resolved](cme-anomalies-resolved.md) — BM uses Pattern B; GiveInventory NetOut is server-only (no client subscriber); SGWHomeless is class_SGWHomeless (editor dev tool, not catch-all).
- [Faction / alignment system](faction-alignment-system.md) — EFaction 34-value enum, hostile sentinel=10, GameBeing+0x134/0x135 field layout, 1-byte wire format, combat gate logic.
- [Crafting / DHD / Loot mechanics](crafting-dhd-loot-mechanics.md) — VCrafting class, EmitNetOut_onDialGate 6-glyph resolution, StargateTriggerFailed (new event), VLootables cache-warm pattern, ring transporter fields.
- [DataType registry system](datatype-registry-system.md) — Two-registry model (01f126b8/01f126b4), 17 primitive subclasses, MD5 type hashing, CME property system, sub-slot confirmed. High-half 015a8b40–015bffe0 = non-protocol editor tooling (PE debug, Win32ThreadEx, GFx importer, unrpc SpawnPoint API).
- [Timer system extended map](timer-system-extended.md) — 8 Event_NetIn_TimerUpdate subscribers (not 5); types 6/14/16 newly found; CooldownManager has no type-gate; GameEntityManager ctor is non-standard (3 data params, 0x10 bytes).
- [World entry resolved questions](world-entry-resolved.md) — Q1: ENABLE_ENTITIES=1 byte; Q2: PostLoad handler=0x00de8430; Q3: World_Loaded emitter=0x005541a0; Q4: ClientReady wire-send=0x00d43dc0; Q5/Q6 still open.
