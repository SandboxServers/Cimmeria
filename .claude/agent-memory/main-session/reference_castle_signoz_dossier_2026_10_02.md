---
name: reference-castle-signoz-dossier-2026-10-02
description: "Recent CellBlock and Castle playthrough telemetry compared with Lomiada's world dossiers"
metadata:
  type: project
---

Reviewed 2026-10-02 in SigNoz `cimmeria-server` logs (14-day window), against
`SGW_FINAL_DEVELOPER_HANDOFF.zip` world dossiers. Added `cimmeria-client`
telemetry on 2026-10-03 UTC. Full, sanitized timeline and query anchors:
`docs/analysis/playtests/2026-10-02-castle-signoz-dossier/README.md`.

- Sep 28 dev: zero-mission CellBlock entry → mission 688 completion → Castle
  missions 701, 702, 703, 704, 706, 708 completion → accepted Harset dial,
  gate sequences 10145/10158, deferred transition and Harset entry. Five
  CellBlock deaths and a re-entry did not block the main chain. Romney 703
  completed before Zuritska 702; a Sep 29 colo run completed them in reverse.
- Sep 29 colo: fresh CellBlock → Castle, 701–703 completed, 704 active at
  CommsRoom; four Castle deaths and no later 704 completion or dial observed in
  the queried window. This is not enough to diagnose a soft-lock.
- Three successful Castle→Harset dials were found (two Sep 25, one Sep 28),
  all in dev. The Sep 28 open was ~0.16 s after acceptance and the crossing
  hold ~1.59 s, matching the current 100 ms and 1.5 s timers. The dial log's
  “opens in 4s” text is stale. These are server sends, not proof of rendered
  gate/camera effects.
- Dossier mission lists omit live CellBlock 682/683/685 and Castle 706; the
  zero Castle map-linked Kismet rows are a coverage limit, since the seeded
  gate EventSet 10011 sent both sequences. Optional/faction missions 567,
  1376, 1520, 1521, 1664 were not evidenced by these main-path traces and
  have no Castle content chains under `db/resources/Content/Seed/castle*.sql`.
- Every sampled Castle world-entry log reports `access_level=2`; do not
  generalize these playtests into an ordinary-player completion rate.
- Sep 29 colo client: both CellBlock ring Matinees (`SeqAct_Interp_0`, 6.05 s)
  and the Straegis Matinee (`SeqAct_Interp_1`, 10.01 s) activated and reached
  full length with `cut_short=false`; Castle chunks became visible at world
  entry. The 704 CommsRoom step and objective updates reached the client.
  This supports client execution, not exact rendered frames or audio.
- The same client logged 303 `UnitFrames.lua` nil-`colors` errors during Castle
  combat, plus repeated reticule and Castle player-controller load failures.
  No 704 Zuritska `DialogDisplay` event was found around three server dialog
  sends; the same hook did capture an earlier CellBlock dialog. Treat this as
  a bounded delivery/instrumentation question, not proof of a soft-lock.
  Follow-up interact bursts targeted a hostile NPC during combat.
- No client capture was found for the three successful dev Castle gate runs;
  gate vortex, chevrons, crossing presentation and sound remain unverified.
  Native client `ts_ms` is event time; ingest timestamp can lag. Governor
  rollups make absent hot-stream events inconclusive.
