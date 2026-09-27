---
name: reference-client-idle-send-cadence
description: "Measured 2026-09-19 on the colo: an idle in-world client still sends ~6 packets/s (per-tick AUTHENTICATE), perfStats every 15.0 s; NetInactivityTimeout=15 is the CLIENT's tolerance of server silence. Read before changing any inactivity timeout."
metadata:
  type: reference
---

Measured from SigNoz (`cimmeria-server` + `cimmeria-network`, one live colo session, 2026-09-19) while reviewing PR #711:

- **An idle, stationary in-world client is not silent.** In a 13 s window it sent 77 packets (about 6/s): 58 were `AUTHENTICATE received -- ignored`, the rest bundle messages. So `last_recv` in the base tick-sync loop refreshes continuously for a healthy client.
- `SGWPlayer.perfStats` (base method 29) arrives every **15.0 s ± 60 ms**. It is the only app-level idle message logged at DEBUG.
- World entry `playCharacter → onClientReady` for Castle_CellBlock took about 2.4 s on that player's machine.
- No `client inactive for` (60 s server timeout) events in the prior 30 days.

**Implication:** shortening the server's client-silence timeout would not kick idle players. The exposure is clients whose process is frozen for 15-60 s: slow-disk map loads, debugger pauses (see the x64dbg rule in `docs/agents/rules-and-gotchas.md`), window-drag modal loops.

Don't conflate directions: the original BigWorld server's `client_inactivity_timeout` is 300000 ms (`docs/protocol/login-handshake.md`), while `NetInactivityTimeout=15` (spec R10) is the client's tolerance of **server** silence.

Useful query shape: `service.name = 'cimmeria-network' AND addr = '<ip:port>'`, grouped by `body`. See [[reference-signoz-log-mining]].
