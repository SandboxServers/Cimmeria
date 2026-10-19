# player-capture-bots: architect amendment report (2026-10-18)

> Type: architect report. Ledger: [README.md](README.md). Resume card: [RESUME.md](RESUME.md).

Status: **done**. No code was written and the user was not asked. Wave 1 was not touched: PC-01, PC-02, PC-06, PC-10, PC-11, `work-packets-w1.md` and the frozen `interaction.pose` contract section in `work-packets.md` are byte-for-byte unchanged. The pose row gains `mercury_seq` through a new section, "Addendum A1", placed after the frozen list.

## Owner rulings applied

| Ruling | Where it landed |
|---|---|
| 1. Consent is closed ("telemetry is telemetry") | D-PC17 is withdrawn. PC-16 no longer edits `launcher-guide.md`, `client_changes.rs` or `launcher-core`. F13 keeps only the fact that the DLL hash is compiled into the launcher, so a DLL change needs a launcher release. PC-18 says the opt-in covers the camera. F16 drops `launcher-guide.md`. |
| 2. Releases are pre-authorized | D-PC22 is added and D-PC8 reworded. The "ask the user" release gates on PC-07 and PC-07b are gone. The named procedure is in `work-packets-w3b-docs-gates.md` § Release procedure. Lab use (PC-19, PC-25 calibrate) and colo export (PC-25) still ask the user. |
| 3. Friction goes to SigNoz, never Discord | PC-11 already does this and is unchanged. The README intro, IDEAS row 3, the PC-07 check (DEBUG rows in SigNoz, no Discord post) and RESUME say so. |
| 4. Clock skew and join key | Two keys with no wire patch, in three join tiers (below). PC-15, PC-16, PC-17, PC-21b and the camera contract are amended, and PC-16b, PC-26, PC-27 and PC-21c are new. |

## Join design (D-PC18 to D-PC20)

- **Sequence key (D-PC19).**
  - The cell router already receives each client cell method call with its Mercury `packet_seq` (F17). PC-26 records it per actor for `interact` (cell method 74) and `useItem` (39), and stamps it on the pose row as `mercury_seq`, field 30, after `pose_seq`.
  - On the DLL side, PC-16b reuses the ability trace's `Channel::send` / `Nub::send` sequence join (F18). A `ReachProbe` around the camera send gives the camera row a `send_id` and writes `client.camera.sent_seq` with the packet range.
  - The match is exact for `interact` and best effort for `useItem`, whose pose row waits for the base's `ItemUsed`.
- **Clock (D-PC18).**
  - Each DLL upload carries `X-Cimmeria-Client-Sent-Ms` (PC-16). This is DLL-to-admin-api HTTP, not the game wire.
  - PC-27 measures the session offset from the header, falling back to the existing NT-40 newest-row rule. It caps the server time at the time the chunk was received.
  - PC-17 writes `server_ts_ms` and `clock_offset_ms` on camera rows. The clock error drops from up to one flush interval to the upload's network delay (F19).
- **Join order (D-PC20).**
  1. Sequence, with a 60 s guard against a reused sequence.
  2. Server time within 500 ms.
  3. Raw client time within `--join-window-ms` (default 1 s), only for rows without `server_ts_ms`.

  Every join records its method. A camera whose sequence disagrees with the pose's is never matched by clock. PC-21b implements the join and PC-21c the keys and CLI. PC-07b reports the share joined by each method and the measured error of the clock join.
- **Trust.** Every client value only places or labels the client's own rows. A bad stamp reads as absent, and no chunk is ever refused. server-authority-enforcer reads PC-27 and PC-17.

## Waves (D-PC21)

| Wave | Packets | Review |
|---|---|---|
| 3a | PC-15, then PC-16, then PC-16b (one PR from `pc-16b`); PC-26 (one PR); PC-27, then PC-17 (one PR from `pc-17`) | one packet-review-fixer, and server-authority-enforcer on PC-27 and PC-17 |
| 3b | PC-05, PC-18 (documentation-writer), after 3a merges | one reviewer |
| gates | PC-07 after wave 2 and `/release`; PC-19 after 3a (ask first); PC-07b after PC-19, `/release-launcher` and `/release` | read-only agents |
| 4b | PC-21, then PC-21b, then PC-21c (one PR from `pc-21c`); PC-24 in parallel | one packet-review-fixer |

Every new packet is S-sized (about 40k working tokens) and carries its own rules: PowerShell only, the lane command, i686 lanes for the DLL, file caps, CRLF, `nt:id-only`, and privacy.

## Files changed

- `work-packets.md`:
  - header list, stacked chains, and the wave 3a and 3b rules;
  - the live-gate rules (releases pre-authorized, with the PowerShell `/release` note);
  - new Addendum A1;
  - the camera contract (`send_id`, `client.camera.sent_seq`, the upload stamp, `server_ts_ms` and the join order);
  - the packet index.
- `work-packets-w3-camera.md`: rewritten as wave 3a's DLL chain (PC-15, PC-16, PC-16b). The disclosure work is gone; the send stamp and PC-16b are added.
- `work-packets-w3-join.md`: new; PC-26, PC-27 and PC-17.
- `work-packets-w3b-docs-gates.md`: new; PC-05, PC-18, the release procedure, and PC-07, PC-19 and PC-07b.
- `work-packets-w4-tools.md`: PC-21b split into PC-21b (join) and PC-21c (keys and CLI); the intro, 4b row, PC-25 and PC-09 are updated. It is 476 lines.
- `README.md`: the intro, F13, F16, new F17 to F19, D-PC8, D-PC17 withdrawn, new D-PC18 to D-PC22, and the ledger (wave 1 Writing; new 3a, 3b and 4b rows; gate wording).
- `RESUME.md`: rewritten (wave 1 in flight; ops created `pc-01`, `pc-06`, `pc-10` and `pc-11`).
- `IDEAS.md`: rows 1, 3 and 4.
- `architect-revision-report.md` is left as the historical record. Its D-PC17 and ±1 s join text is superseded by this report.

All of these are CRLF.

## Flags for the orchestrator

- **No new BlockedDecision.** D-PC10 (content names) and D-PC12 (`.bug` cards) still stand unless the owner vetoes them.
- **Ops may refuse `gh pr comment`.** Its allowlist may not include it. If ops refuses the `/release` comment, give a general-purpose agent the exact PowerShell commands in `work-packets-w3b-docs-gates.md` § Release procedure. Never post from Git Bash, which rewrites `/release` into a path.
- **The `Channel::send` / `Nub::send` hooks are unverified live.** No live run has graded `client.ability.sent_seq` yet. PC-19 is the first proof. If they fail, the sequence tier finds nothing and the join falls back to server time (PC-27), so nothing breaks; the fix goes back to the architect.
- **The colo release restarts the colo** and rebuilds its database from the seeds, which disconnects players. Ops times the release accordingly; it needs no approval.
- **Saved view.** If the live "Playtest: friction" saved view filters on WARN, only the owner can change it. PC-05 documents this and PC-07 reports it.
