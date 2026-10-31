# lab-autopilot: user directives (2026-10-31)

Source: the user, in session. These bind the architect's design.

1. Goal: extend the lab MCP and the lab driver to match what we lift from ramenvr/autopilot (Apache 2.0; concepts free, copied code needs attribution). Research is in the session scratchpad: `deep-control.md`, `deep-brain.md`, `deep-bridge.md`, `deep-unreal.md`; our stack inventory is `inventory-lab.md` in this folder.
2. Autopilot officially supports Unreal 5+ only. We back-port any functionality to the Stargate Worlds client (UE3-era, engine build about 3004) using our deep knowledge of SGW.
3. New client-side code is approved, provided it breaks no game functionality and has no negative side effects.
4. That code is **lab only** and never ships to users: it is never in the signed manifest or the launcher update path, and a normal client must not contain it (gate it behind a lab-only build or loader).
5. DLL hooks may be expanded to reach further and wider into the client as needed.
6. Target ("Pinocchio on strings without the long nose"): full puppet control of the client, and no lies. Every action result must be honest and corroborated: the driver cannot claim success the evidence does not support (evidence gate, post-condition reads, server-side corroboration, INCONCLUSIVE otherwise).
7. Lab use still needs a user OK before any live run.
