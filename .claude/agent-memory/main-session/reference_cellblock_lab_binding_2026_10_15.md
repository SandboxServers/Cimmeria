---
name: Cellblock lab endpoint binding
description: Match the client gameplay socket, lab daemon target and Codex server MCP target before attributing server evidence.
type: reference
---

# Cellblock lab endpoint binding (2026-10-15)

Source: [fresh Human lab run](../../../docs/analysis/cellblock-continuation/route-baseline.md#2026-10-15-fresh-human-lab-run).

The Codex `lab-server` MCP connection and the lab daemon's server target can
point at different running servers. A zero-session result from the former did
not mean the fresh client was offline: the client and a local server owned
opposite ends of the same gameplay socket, and a read-only call to that local
server found the character in world 12. Check endpoint binding and character
identity before claiming a packet tap, log or database row corroborates a
client action. Do not silently combine evidence from the two endpoints.

The same run reached Livewire by a native door click, but both screenshot and
minigame helper calls stalled. The client stayed responsive and later appeared
elsewhere in world 12 without mission advance. The cause is unverified; a
fresh core-route timing run must start after that lab issue is isolated.
