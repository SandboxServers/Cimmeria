"""Open the profiler database read-only and scope it to a report window.

Every section reads requests through the temp view `wreq` (requests inside the
window) and tool calls through `wtool`, so the window is applied in one place.
Prices come from one price-table version, chosen here and stamped on the report.
"""

import sqlite3
from pathlib import Path

# Schema versions the reports read. Version 2 (TP-01a) only added columns.
SUPPORTED_SCHEMAS = ("1", "2")


class ReportError(Exception):
    pass


def open_db(path):
    """Open `path` read-only. Temp tables and views still work on a read-only main database."""
    p = Path(path)
    if not p.is_file():
        raise ReportError(f"no profiler database at the given path")
    db = sqlite3.connect(p.resolve().as_uri() + "?mode=ro", uri=True)
    db.row_factory = sqlite3.Row
    return db


def scope(db, since=None, until=None, price_table=None):
    """Create the window views and pick the price table. Returns the chosen price-table version."""
    version = db.execute("SELECT value FROM meta WHERE key = 'schema_version'").fetchone()
    if version is None or version[0] not in SUPPORTED_SCHEMAS:
        raise ReportError(f"schema_version {version[0] if version else None!r}, reports support {', '.join(SUPPORTED_SCHEMAS)}")
    db.execute("DROP TABLE IF EXISTS temp.report_window")
    db.execute("CREATE TEMP TABLE report_window (since TEXT NOT NULL, until TEXT NOT NULL, price_table TEXT)")
    if price_table is None:
        row = db.execute("SELECT price_table FROM profiler_runs WHERE status = 'ok'"
                         " ORDER BY run_id DESC LIMIT 1").fetchone()
        price_table = row[0] if row else None
    if price_table is None:
        row = db.execute("SELECT MAX(version) FROM price_tables").fetchone()
        price_table = row[0] if row else None
    db.execute("INSERT INTO report_window VALUES (?, ?, ?)", (since or "", until or "￿", price_table))
    db.executescript("""
        DROP VIEW IF EXISTS temp.wreq;
        DROP VIEW IF EXISTS temp.wtool;
        DROP VIEW IF EXISTS temp.wprice;
        CREATE TEMP VIEW wreq AS
            SELECT r.*,
                   CASE WHEN r.agent_id IS NULL THEN 'main' ELSE 'subagent' END AS scope,
                   CASE WHEN r.agent_id IS NULL
                        THEN CASE WHEN s.is_coordinator = 1 THEN 'main (coordinator)' ELSE 'main' END
                        ELSE COALESCE(a.custom_agent_type, a.agent_type, 'unknown-agent') END AS agent_type,
                   t.kind AS trigger_kind
            FROM requests r
            JOIN sessions s ON s.session_id = r.session_id
            JOIN triggers t ON t.trigger_id = r.trigger_id
            LEFT JOIN agents a ON a.agent_id = r.agent_id
            WHERE r.ts >= (SELECT since FROM report_window) AND r.ts < (SELECT until FROM report_window);
        CREATE TEMP VIEW wtool AS
            SELECT c.*, w.agent_type, w.scope, w.model
            FROM tool_calls c JOIN wreq w ON w.request_id = c.request_id;
        CREATE TEMP VIEW wprice AS
            SELECT * FROM price_tables WHERE version = (SELECT price_table FROM report_window);
        -- Estimated list-price USD per request, for every request (the per-PR
        -- section counts a PR's whole spend, not only what falls in the window).
        -- usd is NULL when the model has no price in the chosen table.
        CREATE TEMP VIEW rcost AS
            SELECT r.request_id,
                   p.model IS NOT NULL AS priced,
                   r.input_tokens * p.input / 1e6 AS usd_input,
                   r.output_tokens * p.output / 1e6 AS usd_output,
                   r.cache_read * p.cache_read / 1e6 AS usd_cache_read,
                   r.cache_write_5m * p.cache_write_5m / 1e6 AS usd_cache_write_5m,
                   r.cache_write_1h * p.cache_write_1h / 1e6 AS usd_cache_write_1h,
                   (r.input_tokens * p.input + r.output_tokens * p.output + r.cache_read * p.cache_read
                    + r.cache_write_5m * p.cache_write_5m + r.cache_write_1h * p.cache_write_1h) / 1e6 AS usd
            FROM requests r LEFT JOIN wprice p ON p.model = r.model;
        CREATE TEMP VIEW wcost AS
            SELECT w.*, c.priced, c.usd_input, c.usd_output, c.usd_cache_read,
                   c.usd_cache_write_5m, c.usd_cache_write_1h, c.usd
            FROM wreq w JOIN rcost c ON c.request_id = w.request_id;
    """)
    return price_table


def stamp(db, price_table, report_commit=None, since=None, until=None):
    """The version stamp every report carries."""
    runs = db.execute("SELECT run_id, profiler_commit, finished_at FROM profiler_runs"
                      " WHERE status = 'ok' ORDER BY run_id DESC").fetchall()
    window = db.execute("SELECT MIN(ts), MAX(ts), COUNT(*) FROM wreq").fetchone()
    versions = [r[0] for r in db.execute(
        "SELECT DISTINCT cc_version FROM wreq WHERE cc_version IS NOT NULL ORDER BY cc_version")]
    models = [r[0] for r in db.execute("SELECT DISTINCT model FROM wreq ORDER BY model")]
    unknown_records = db.execute("SELECT COALESCE(SUM(unknown_records), 0) FROM profiler_runs").fetchone()[0]
    unknown_shapes = db.execute("SELECT COUNT(*), COALESCE(SUM(count), 0) FROM unknown_shapes").fetchone()
    return {
        "schema_version": db.execute("SELECT value FROM meta WHERE key = 'schema_version'").fetchone()[0],
        "profiler_commit": _short(runs[0]["profiler_commit"]) if runs else None,
        "report_commit": _short(report_commit),
        "ingest_runs": len(runs),
        "last_ingest": runs[0]["finished_at"] if runs else None,
        "price_table": price_table,
        "window": {
            "since": since, "until": until,
            "first_request": window[0], "last_request": window[1], "requests": window[2],
        },
        "claude_code": {"first": versions[0] if versions else None,
                        "last": versions[-1] if versions else None, "all": versions},
        "models": models,
        "unknown_records": unknown_records,
        "unknown_shapes": {"shapes": unknown_shapes[0], "records": unknown_shapes[1]},
    }


def _short(sha):
    # Twelve characters identify a commit; forty look like a secret to the gate.
    return sha[:12] if sha else None
