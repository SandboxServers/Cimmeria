---
name: cooked-override-client-cache-persistence
description: The client writes pushed cooked overrides to its disk cache; an override the server later removes or renumbers is never evicted. Bisect world-entry crashes with SigNoz versionInfoRequest logs.
metadata:
  type: project
---

Found 2026-09-27 while bisecting the Castle_CellBlock world-entry crash (builds b22907eb5 and 38296335f).

- **Pushed overrides persist on the client's disk.** A category-5 push moves the client's reported `client_version` to the server's version. The next session reports that version, gets 0 `invalid_keys` and no push. So a bad override keeps crashing the client after the push that delivered it.
- **Removed keys are never evicted.** `cooked_data.rs` puts only the server's current override ids in `invalid_keys`. When #938 renumbered 100100/100101 to 60100+, the client kept 100100/100101. `invalidate_all = true` would empty the whole category, and the client does not lazy-fetch, so it is not a safe eviction tool.
- **The bisect tool is SigNoz.** Group `body = 'Pushing overridden element fragments'` by `service.version, element_id, addr`. Read `Responding to versionInfoRequest` (`category_id`, `client_version`, `version`, `invalid_keys`) per session to see what each client holds. The create-player bytes (`world_data/phases.rs`) did not change in that window.

**Why:** an override bug outlives the server fix on every client that received it.
**How to apply:** when a crash follows an override change, check what the client caches, not only what the current build sends. Any removal or renumber of an override needs an eviction plan.
