# Wave 3 ruling (review pass 1, verdict decision)

Architect, 2026-10-22. Source: `.claude/reports/launcher-uat-fixes/wave3-review-1.md` in `lf-w3`.

## D1: log chunk rate vs a 40-chunk upload — option (b), launcher-side pacing

- No server change. The wave 3 limits stay exactly as reviewed and security-approved: 20 chunks per log session per minute, the shared per-install window of 20 per minute, and every C-LF6 row.
- LF-14 paces sends: `MIN_CHUNK_INTERVAL_MS = 3_500` between chunk send starts (no sleep before the first chunk, none when the previous send already took longer). That is at most 18 chunks in any 60 s window, so the install's event session keeps headroom. A 429/503 still ends `Partial` (R16); no backoff loop.
- Pacing goes through an injectable `Pacer` (`now_ms`, `sleep_ms`); tests use a fake clock and no real sleeps. LF-19 reuses the same pacer for game rows.
- Why not (a): it reopens the server limits that were just approved. Why not (c): every full-backlog first press would end `Partial`, which is the bug this campaign fixes. D-LU4 holds: nothing is removed, the 2,048-byte floor is untouched, and only send timing changes.
- Cost: a full backlog takes about 2.5-4 minutes (40-60 chunks), well inside the 900 s token; a 401 still re-mints once. LF-15 must not put a timeout under 15 minutes around `Upload`. LF-16's busy text says it can take a few minutes.

## Notes applied to the spec

- N1: LF-14 item 3 now splits at 384 KiB uncompressed NDJSON and halves any batch whose gzip is over 64 KiB; test added.
- N2: C-LF6 has the per-address row (400,000, `CIMMERIA_TELEMETRY_LOG_ROWS_PER_IP_DAY`, `log_ip_budget`) and which ceilings count rows received or rows kept. `log_ip_budget` was added to LF-12 item 5 and the LF-17 telemetry bullet.
- N3: README D-LF8 has the current numbers and the D1 outcome.
- D2: C-LF4 and LF-17 now say "launcher event targets", checked after decode and before any budget charge; other unknown targets are dropped and counted.
- N4 (code): a packet-coder applies it in `lf-w3` as written in the review. Then re-run S2 and S3 `chunk`. That is the only code change wave 3 still needs.

## Next

Apply N4 in `lf-w3`, then run wave review pass 2 (scope: N4 only). The spec changes above affect waves 4 and 5 only.

Ruling check (`.claude/reports/launcher-uat-fixes/wave3-ruling-check.md` N1) required one more spec edit: LF-14 Tests now name an incompressible-batch test (pseudo-random hex rows, halved until every gzip body is at most 64 KiB) and its revert proof, so the halving branch is pinned.
