## Summary

Wave 5 of the launcher-uat-fixes campaign (U5, LF-18): retire the old blob log-upload path now that "Upload debug logs" goes through the `logs`-purpose ingest session (U4, #1987).

- Removes the blob zip builder, the SAS upload and its offsets file from the engine, shell and frontend.
- Removes `LAUNCHER_LOG_SAS_URL` from the workflows, the macOS build script and the release docs.
- Docs: the legacy blob parts of `launcher-distribution-setup.md` are gone; `docs/readme.md` and `launcher-guide.md` no longer list the SAS as a key. The archived `docs/client/sgw-launcher.md` is left as history (owner call).
- The ingest path, limits, scrub and the 2,048-byte message floor are untouched.

Owner confirmed (2026-10-22) that nobody uses log upload today, so removing the blob path breaks nothing in the field.

## Tested / untested

Engine (924) and shell (147) test runs, clippy and fmt were green locally per the fix report; the admission test for the operation gate fails when reverted. One packet review loop (2 passes, ship). No live UAT: the laptop UAT of upload is owed.

Known, not from this PR: the docs-gen check reports stale generated counts; the campaign close-out (LF-20) regenerates them.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NdhQoumCZtZKGCMdPDbWJF
