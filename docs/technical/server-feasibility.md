# Server Emulator Feasibility Assessment

Analysis of what's available in sgw.exe for building a Stargate Worlds server emulator.

## What We Have (High Confidence)

### Complete Message Catalog
- **167 Server→Client messages** (Event_NetIn) — every message the server can send
- **254 Client→Server messages** (Event_NetOut) — every message the client can send
- **256 slash commands** — all player-invokable commands
- This is a **complete interface contract** — the server must implement handlers for all NetOut messages and generate appropriate NetIn responses

### Login Flow (Fully Mapped)
- SOAP endpoints, XML schema types, and error codes
- Session key exchange mechanism (AES)
- BaseApp connection handshake
- All 31 error codes for error handling

### Mercury Protocol Structure
- Class hierarchy from RTTI
- Debug strings revealing message framing, length formats, and error handling
- Bundle/Packet/Channel architecture
- Reply correlation mechanism

### BigWorld Server Architecture
- Server role topology: LoginApp → BaseAppMgr → BaseApp → CellApp → DBMgr
- Entity management via `entity_manager.cpp` (assertions reveal state machine)
- Server connection via `servconn.cpp` (assertions reveal protocol expectations)
- Entity migration (supersede method between BaseApp and CellApp)

### Game System Contracts
- Every game system's network interface is known (what messages it sends/receives)
- Stargate travel flow is fully mapped
- Combat queue interactions are clear
- Organization, mail, trading, minigame APIs are defined

### AtreaRL Loader (Already Built)
- Binary patching system already handles client modifications
- Network sniffer captures real traffic for protocol analysis
- AES key extraction enables traffic decryption

## What's Missing (Gaps for Server Implementation)

### Message Wire Formats
- We know the **names** of all 421 messages but NOT their **binary wire formats**
- Each message's serialization (field types, field order, sizes) must be determined
- Approach: Use AtreaRL's PCAP captures + Ghidra decompilation of `SGWNetworkManager` serialization methods

### Mercury Protocol Internals
- Bundle framing bytes and opcodes
- Packet header format
- Channel establishment handshake
- Reliability/ordering semantics
- Fragment reassembly
- The debug strings give hints but not complete formats

### Server-Side Game Logic
- Combat formulas (damage, healing, resist, crit, etc.)
- Ability definitions and effect processing
- Mission state machines and trigger conditions
- MOB AI behavior trees
- Spawn system rules
- Loot tables and drop rates
- Crafting recipes and success rates
- Level/XP curves

### Database Schema
- No direct evidence of DB schema in the client binary
- Entity properties give hints but not table structure
- Would need to design schema based on the message contracts

### Content Data
- Map/zone definitions
- NPC/MOB definitions
- Item database
- Ability/skill databases
- Mission/quest scripts
- Stargate address registry
- Organization rank/permission templates

### BigWorld Specifics
- Cell partitioning scheme
- Entity distribution across CellApps
- Load balancing algorithms
- Entity migration protocol details

## Feasibility Rating: MODERATE-HIGH

### Why It's Feasible
1. **Complete interface contract** — We know every message the server must handle
2. **Login flow is solved** — Authentication and session establishment are mapped
3. **Existing loader** — AtreaRL already patches the client for emulator use
4. **PCAP capability** — Can capture and decrypt real traffic for wire format analysis
5. **Rich RTTI** — Extensive class metadata for understanding object models
6. **BigWorld is documented** — BigWorld Technology has public documentation from other licensees
7. **Unreal Engine 3** — Well-understood engine with extensive community knowledge

### Why It's Challenging
1. **Wire format RE** — 421 messages need individual format analysis (biggest task)
2. **No server code** — All game logic must be reimplemented from scratch
3. **No content data** — Abilities, items, missions need to be extracted or recreated
4. **Mercury protocol** — Proprietary protocol needs full reverse engineering
5. **Scale** — This is an MMO; proper multi-process server architecture is complex

### Recommended Approach
1. **Phase 1: Protocol** — Use AtreaRL PCAP captures to decode Mercury wire format
2. **Phase 2: Login** — Implement SOAP auth server + BaseApp handshake
3. **Phase 3: World Entry** — Get client to enter world (entity creation, map load)
4. **Phase 4: Movement** — Entity position updates, visibility system
5. **Phase 5: Chat** — Communication system (relatively self-contained)
6. **Phase 6: Combat** — Core combat loop (biggest gameplay system)
7. **Phase 7: Content** — Missions, NPCs, items, stargates
8. **Phase 8: Social** — Organizations, mail, trading, auction house

### Parallel Workstreams
- **Wire format analysis** (Ghidra + PCAP) can run alongside server development
- **Content extraction** from cooked data packages can proceed independently
- **Client-side modding** via AtreaRL can add debug features to aid RE
