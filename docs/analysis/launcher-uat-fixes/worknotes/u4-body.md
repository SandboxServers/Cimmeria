## Summary

Wave 4 of the launcher-uat-fixes campaign (U4): "Upload debug logs" ingests launcher and game log lines into SigNoz through a `logs`-purpose launcher session.

- LF-13: log line reader and per-word scrub for launcher logs.
- LF-14: paced chunk sender (>= 3,500 ms between chunks, halving to fit 64 KiB gzip, 429/503 ends Partial, offsets advance only after 2xx).
- LF-15: engine flow (inspect / upload), one `LauncherSender::logs()` per app run.
- LF-16: upload UI text (consent note, busy text, Sent / Partial / NothingNew / NoLogs / Unreachable); a failed mint is a retryable network error.
- LF-19: game log upload (streaming reader, per-word scrub, offsets with tail fingerprint, symlink checks).
- LF-17: operations docs (refusal reasons, `CIMMERIA_TELEMETRY_LOG_ROWS_PER_IP_DAY`, limits).

## Decisions (D-LU4)

Hardening only, no data removed. Owner risks recorded: chat and names leave the machine (per-word scrub only); messages over 2,048 bytes are cut by design; rows over 4,096 bytes after escaping are dropped server side; older lines beyond 20,000 new lines in one file are not uploaded; daily ceilings; secrets shorter than 32 characters or split by dots are kept.

## Tested / untested

Unit and flow tests per packet, packet reviews (up to 3 passes), security passes for LF-14 and LF-19, and a wave review (ship). No live UAT yet: the laptop UAT of upload (U4/U5) is owed and needs the colo server release.

## Docs

Rows touched are listed in `.claude/reports/launcher-uat-fixes/LF-17-coder.md`.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NdhQoumCZtZKGCMdPDbWJF
