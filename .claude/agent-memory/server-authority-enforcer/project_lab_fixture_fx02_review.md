---
name: project-lab-fixture-fx02-review
description: FX-02 lab fixture store/apply review (2026-10-10) - no client stage path; take-before-ownership lets any account burn a staged fixture via playCharacter(foreign id)
metadata:
  type: project
---

FX-02 (lab-fixtures) staged-fixture store: `base-session/.../world_entry/lab_fixture.rs` (pub stage/take, re-exported from cimmeria-services); apply in `base-world-entry/.../lab_fixture/` called from `handle_play_character` after the entry latch.

Cleared: no client-reachable caller of `stage_fixture` (only tests in FX-02; FX-04 lab endpoint is the intended one); access level read from `ConnectedClientState.access_level` under the connected lock; ownership precheck `sgw_player.account_id == session account_id` before writes; all SQL bound.

Finding: `take_fixture(player_id)` runs before the GM and ownership checks, keyed on the client's playCharacter id. A foreign id fails `query_world_entry` (`AND account_id = $2`), which resets `world_entry_sent`, so one connection can loop ids and discard every staged fixture. Fix shape: store account_id in StagedFixture, take only on match.

Update 2026-10-11: take-on-match adopted (store keys account_id, mismatch leaves staged). FX-04 `server_stage_fixture` (lab-mcp tools/fixture.rs) cleared: bearer-gated, SELECT-only + bound, exactly-one name match, accesslevel and account_id from the row, apply rechecks session GM + row owner. Residuals: the gate is "any GM account", so a lab token holder can stage onto a human GM's offline main (last-word match widens this); `online_players` filters `world_entry_sent`, so a character mid-Play reads as offline (the accepted race; the effect is a re-apply at the next Play within the TTL, same account only); access_level is the session's login-time value, so a GM demoted mid-session still applies.

**How to apply:** when FX-04 lands, check the endpoint's GM/offline gates and whether take-on-match was adopted. Related: [[exploit_address_reuse_session_takeover]] for deferred (held) Play actions keyed by addr.
