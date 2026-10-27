# LP-06b lab proof: composed cache, in game

**Verdict: pass.** With a composed `Cache.en-US`, the client reported the served version of all 21 categories on both launches, ran no resync, and showed server content in game: an overridden dialog (category 5) and an overridden item (category 4). All 8 composed PAKs equal a fresh compose entry for entry after both client runs. The Dialogs reading 20823 seen in LP-06 did not recur (31461 on both launches). One gap: the server-side `cooked_data.*` events are not in the server log files, so "21 `up_to_date`, 0 `sync_start`" is shown from the client side (see Notes).

## Setup

- Local server at the `lcp-w1` head (release build), own test database `sgw_lcp_06b`, one lab account, one character (SGU Soldier, created for this run).
- Profile cache wiped (backed up, 24 files), then `cooked-pak-tool compose` from `data/cache` over the client's shipped source cache; the client was down for the compose (checked: no client running, no lease holder). Compose wrote 8 PAKs (ids 1-5, 11, 12, 13).
- Lab client on the per-instance profile. The login server row had to be set to the local row with real mouse clicks (the `lab_login` flow timed out at `server_list` twice, same as LP-06).

## Step 2: every `CookedDataDialogs*` file the client could see (before the client started)

| Location kind | File | `MetaData` version | Entries |
|---|---|---|---|
| Instance profile cache, live (before wipe) | `CookedDataDialogs.pak` | 31461 | 5425 |
| Instance profile cache, leftover | `CookedDataDialogs.pak.bak-20261010` | 63239 | 5411 |
| Instance profile cache, leftover | `CookedDataDialogs.pak.lifted-20261010` | 63239 | 5411 |
| Default Documents profile, live | `CookedDataDialogs.pak` | 44069 | 5407 |
| Default Documents profile, leftovers | `.bak-20261010`, `.lifted-20261010` | 63239 | 5411 |
| Second Documents profile (Firesky2), live | `CookedDataDialogs.pak` | 7696 | 10 |
| Client's shipped source cache | `CookedDataDialogs.pak` | 7660 | 5406 |

No file anywhere carries 20823. The lab client reads the instance profile only. The wipe removed the live file and the two leftovers from the instance cache (restored afterwards); the client's startup copy brought the source Dialogs PAK in (7660), and the composed file replaced it.

## Expectations

| Expectation | Result |
|---|---|
| Every `version_reply` is `up_to_date` | PASS from the client side: `cooked.versions_held` lists 21 real versions equal to the served ones, `resync_pending 0`, on both launches. Server-side reply events not captured (Notes). |
| No `sync_start`, no resync | PASS: 21 `cooked.version_set` events per launch, all with `previous` equal to `version`; no write with a changed version; no source-archive writes in launch 2. Character select appeared without the ~30 s freeze. |
| `versions_held` for Dialogs | PASS: 31461 on both launches (LP-06 showed 20823 once). The client's first read of the cache Dialogs `MetaData` reported entry index 5424 and version 31461, matching the composed file; the 7660 read is the source cache. Cause of LP-06's 20823 stays unexplained; it did not recur on a wiped, freshly composed cache. |
| One overridden dialog shows its server value | PASS: dialog 60110 (not in the stock cache; added by the served Dialogs) opened on a targeted NPC through the `.net_dialog` console command and displayed the served text ("Vault expansion. I can add 10 slots ..." with the "Expand vault" button). The stock cache has no such dialog, so the client had to read it from the composed file. The server log shows the console command accepted. |
| One overridden item shows its server value | PASS: item 9000 (not in the stock cache) granted with `/gmgiveitem 9000 1`; the client inventory lists it as "Armor Piercing Rounds" with the `Bullet_Armor_Piercing` icon, the served name and icon. The server log shows `gmGiveItem: granting item to GM` for it. |
| All 8 composed PAKs equal a fresh compose after the client quits | PASS: ids 1-5, 11, 12, 13. For each, the entry name sets are equal (1994, 1887, 1051, 6075, 5425, 218, 102, 30 entries) and no entry body differs; the `MetaData` value is equal. Compared after launch 1 and again after launch 2. File bytes were not compared (F21). |
| Second launch, cache untouched: 21 `up_to_date`, 0 `sync_start` | PASS from the client side: same as the line above; 21 metadata-only same-version `cooked_cache.write` events (tier `cache`), none with a changed version (F21, accepted by ruling LP-06-1). |
| `cooked-pak-tool check` after the client rewrote the PAKs | PASS: 21 of 21 `up_to_date`, each `source_version` equal to its served version (38330, 221523122, 61901, 44303, 31461, 7454, 7648, 6615, 6819, 5802, 782034074, 12136, 39841, 3600, 2315, 2202, 2311, 2167, 1967, 1404, 1), `base_missing` and `base_differs` 0. |
| Restore the profile cache | Done (24 files). Lease released, local server stopped. |
| Screensaver | The registry still has it enabled (timeout 600 s); I only ended its process for the run, so nothing to re-enable. |

## Notes

- **Server events.** `cooked_data.version_reply` and `sync_start` did not appear in `logs\*.log` for either launch (the server exports them over OTLP only, and the lab-server MCP could not connect). The client-side proof is equivalent for the outcome: the client held the served version for all 21 PAKs and never began a resync.
- **Dialog 60110 rendering.** The served text shows "I can addERROR: syntax error, unexpected SGW_WHITESPACE, expecting SGW_CHARACTER or SGW_COLON" in the dialog body, so the client's text markup parser stops after "add". This is a content or markup problem in that served dialog text, not a cache problem; the text arrived from the composed file. Worth a separate look at the seed text.
- The login flow needs the server row set by hand (`lab_ensure_in_world` times out at `server_list` while the combo shows the first row); unchanged from LP-06.
- No client-copy path, client bytes, addresses or account names are recorded here.