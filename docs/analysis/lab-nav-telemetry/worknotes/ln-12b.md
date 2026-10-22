# LN-12b worknote

- Step 0 (baseline): `python -I crates/client-patches/overlay/test/run_lupa.py` on the branch before edits: `27 scenarios, 0 failed`.
- Step 1 (done): BlackMarket.lua `now()` accumulates frame deltas in `onPreRender` (dt in (0,1)); `openSession` clears `searchSuspended`. Stub `env.frameDelta`, `tick` in 0.25 s frames, renamed scenario plus two new scenarios. Overlay test: `29 scenarios, 0 failed` (+2 as the packet expects).
- Revert proof (old `now()` body, fix otherwise kept): `29 scenarios, 3 failed`: "paging: Next with more results", "no reply: pending request times out", and "Search re-enables after 4 s of frame deltas" (the 5 s hitch case). The renamed reopen scenario and the F8 scenario still pass, because the `openSession` reset alone clears the suspension. Recorded in the report.
- Step 2 (next): README.md bullets (CRLF check), black-market-client-io.md U13 row, black-market.md Remaining Work bullet, archaeology memory line. Then final commit with the brief's message.
