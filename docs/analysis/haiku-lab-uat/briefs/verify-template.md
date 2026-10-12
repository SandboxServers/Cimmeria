# ClickHouse verification template

When the SigNoz MCP is unavailable (it returned 401 on 2026-10-12), pipe read-only SQL on stdin to `ssh cimmeria-colo 'docker exec -i signoz-clickhouse clickhouse-client -n'`.

```sql
SELECT toDateTime(intDiv(timestamp,1000000000)) t, severity_text, scope_name,
  substring(body,1,160) b, attributes_string, attributes_number, attributes_bool
FROM signoz_logs.distributed_logs_v2
WHERE timestamp BETWEEN toUnixTimestamp('<YYYY-MM-DD HH:MM:SS>')*1000000000
                    AND toUnixTimestamp('<YYYY-MM-DD HH:MM:SS>')*1000000000
  AND resources_string['service.name']='cimmeria-server'
  AND scope_name='<scope>'
  AND toInt64(attributes_number['player_id'])=<PLAYER_ID>
ORDER BY timestamp LIMIT 50 FORMAT Vertical
```

- `toInt64()` on `attributes_number['player_id']` is required (BAD_GET otherwise).
- WARN/ERROR: drop the `scope_name` line and add `AND severity_text IN ('WARN','ERROR')`.
- `console.feedback` text was not found in `body` or `attributes_string['text']` for the bank reply; the client chat log is the evidence for reply text.
- DB: `public.sgw_player` (`bank_slots`, ...); `resources.*` tables need the schema prefix.
