# Known Issues

Issues that exist in both the C++ reference emulator and the Rust rewrite, or are client-side limitations.

## Client / Shared Issues

### KI-1: "Change Server" / Return to Main Menu — fixed in Rust, broken in C++

**Severity**: Was Low, now **FIXED** in Rust server
**Affected**: C++ emulator only (Python `Account.logOff()` was `pass`)
**Root cause**: The C++ emulator's `logOff` handler was a no-op. Both the "Change Server" and "Back" buttons in `CharacterSelect.lua` trigger a logoff:
- "Change Server" calls `relogin()` → sends `logOff (0xC2)` → re-initiates SOAP login
- "Back" calls `disconnect()` → sends `logOff (0xC2)` → shows LoginWin
Without a server response, the client's internal state was inconsistent — the old Mercury session was never cleaned up, so the `relogin()` SOAP call would conflict.
**Fix**: Rust server now sends `LOGGED_OFF (0x37)` with reason=0 when it receives `logOff (0xC2)`. This triggers `EntityManager::loggedOff()` on the client (partial cleanup: clears game entities, preserves login-screen entities). Server destroys entities and removes the session. The client's `relogin()` can then establish a fresh session.
**Pcap evidence**: Sessions `2026-03-04_22-05` and `2026-03-04_22-10` confirmed both buttons send identical `logOff (0xC2)` with 0 bytes payload, and the C++ server only ACKed without any LOGGED_OFF response.

---

## Rust Rewrite — Not Yet Implemented

Features from the C++ reference that are stubbed or missing in the Rust rewrite. These are not bugs — they are known gaps in the rewrite progress.

### ~~KI-2: logOff (0xC2) is a no-op~~ — RESOLVED

**Status**: Fixed. logOff now sends LOGGED_OFF (0x37), destroys entities, and removes the session.

### KI-3: onClientVersion (0xC7) is a no-op

**Severity**: Low
**Description**: The onClientVersion handler logs but does not process the client's version report. In C++, this was used for version mismatch detection — not critical for a single-version emulator.

### KI-4: No immediate ACK-only packet mechanism

**Severity**: Low
**Description**: ACKs for client reliable messages are piggybacked on the next tick sync (100ms interval) rather than sent as a standalone unreliable ACK packet. This adds up to 100ms latency on ACK delivery but is functionally correct.

### KI-5: DISCONNECT (0x0C) does not send a server response

**Severity**: Low
**Description**: When the client sends DISCONNECT, the server logs it but does not send a disconnect acknowledgment or perform session cleanup. The session eventually times out via inactivity.

### KI-6: No disconnect cleanup / session leak

**Severity**: Medium
**Description**: When a client disconnects, its entry in the `ConnectedClientState` HashMap is never removed. Entity IDs are not recycled. Over many connect/disconnect cycles this leaks memory. Requires implementing inactivity timeout eviction.

### KI-7: No duplicate login detection

**Severity**: Medium
**Description**: The same account can log in multiple times simultaneously. C++ tracks active sessions per account and disconnects the existing session on re-login.

### KI-8: No ticket expiration

**Severity**: Medium (security)
**Description**: SOAP login tickets never expire. C++ expires them after 30 seconds. A captured ticket can be replayed indefinitely.

### KI-9: requestCharacterVisuals does not query inventory

**Severity**: Low (cosmetic)
**Description**: Character select screen shows base model without equipped gear. C++ queries sgw_inventory for equipped items and appends their visual components.

### KI-10: createCharacter does not store starting abilities

**Severity**: Medium (gameplay)
**Description**: New characters have an empty ability set. C++ stores charDef.abilities (starting abilities per archetype) on creation.

### KI-11: createCharacter does not store access_level

**Severity**: Low
**Description**: GM accounts create normal characters instead of GM-flagged characters. C++ passes account access_level into the character INSERT.

### KI-12: createCharacter does not validate visual choices

**Severity**: Low
**Description**: Server accepts any visual component combination without validating against charDef allowed components. Client-side validation is the primary guard.

### KI-13: Fragment reassembly not implemented

**Severity**: Medium
**Description**: Mercury fragment reassembly is not implemented. Messages exceeding MAX_BODY (1348 bytes) will fail. Currently no messages hit this limit in practice.

### KI-14: SGWGmPlayer entity type never used

**Severity**: Low
**Description**: All players are created as SGWPlayer regardless of access_level. C++ creates SGWGmPlayer for accounts with access_level > 0.

### KI-15: CellService is entirely stubbed

**Severity**: High (future work)
**Description**: The CellService has no real implementation. All cell-related data (viewport, cell player, forced position) is constructed directly in the BaseApp. This is sufficient for character select and initial world entry but will need a real CellService for gameplay (movement, combat, AoI).
