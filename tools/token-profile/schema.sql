-- Token profiler data contract, schema version 3.
--
-- The ingest (TP-01a, tools/token-profile/ingest/) writes these tables from
-- Claude Code transcripts; the reports (TP-01b, tools/token-profile/report/)
-- read them and nothing else. Neither packet changes this file without
-- bumping meta.schema_version and saying why in docs/analysis/token-usage/.
--
-- The database is local (stdlib sqlite3) and never committed. It may hold
-- absolute paths and session ids; only scrubbed aggregates leave it.
-- Transcript shapes this schema is built from: transcript-format.md.
-- How requests map to PRs: attribution.md.
--
-- Version 2 (TP-01a): ingest_files.parse_state, tool_calls.task_id and
-- tool_calls.pr_ref, prs.closed_at. All additive; see attribution.md and
-- transcript-format.md for what fills them.
--
-- Version 3 (TP-05): cost_states.process_start. A cost-state total covers
-- only the Claude Code process that wrote it, which started at
-- process_start; a resumed session's earlier requests are not in it. See
-- README.md § Reconciliation.
--
-- Token columns are raw counts. thinking_tokens is a SUBSET of
-- output_tokens and is never added on top of it.

PRAGMA foreign_keys = ON;

CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
INSERT INTO meta (key, value) VALUES ('schema_version', '3');

-- One row per ingest run, for version-stamping every report.
CREATE TABLE profiler_runs (
    run_id            INTEGER PRIMARY KEY,
    started_at        TEXT NOT NULL,          -- ISO-8601 UTC
    finished_at       TEXT,
    profiler_commit   TEXT NOT NULL,          -- git rev of tools/token-profile
    price_table       TEXT NOT NULL,          -- price_tables.version used
    files_scanned     INTEGER NOT NULL DEFAULT 0,
    records_read      INTEGER NOT NULL DEFAULT 0,
    unknown_records   INTEGER NOT NULL DEFAULT 0,
    status            TEXT NOT NULL CHECK (status IN ('running', 'ok', 'failed'))
);

-- Incremental ingest: where each transcript file was read up to. A file
-- whose size shrank, or whose first-line hash changed, is re-read in full.
CREATE TABLE ingest_files (
    path              TEXT PRIMARY KEY,       -- absolute, local only
    project_dir       TEXT NOT NULL,          -- ~/.claude/projects/<dir> name
    kind              TEXT NOT NULL CHECK (kind IN ('main', 'subagent', 'meta')),
    size_bytes        INTEGER NOT NULL,
    mtime             TEXT NOT NULL,
    head_sha1         TEXT NOT NULL,          -- sha1 of the first line
    byte_offset       INTEGER NOT NULL,       -- next unread byte
    parse_state       TEXT NOT NULL DEFAULT '{}',  -- JSON: the classifier state at byte_offset, so an
                                              -- appended file resumes mid-turn (open trigger, retry flag, ...)
    last_run_id       INTEGER NOT NULL REFERENCES profiler_runs (run_id)
);

-- Per-model prices in USD per million tokens. Versioned: a report names
-- the version it used, and an old report can be recomputed under a new one.
CREATE TABLE price_tables (
    version           TEXT NOT NULL,          -- e.g. '2026-10-03'
    model             TEXT NOT NULL,          -- API model id, e.g. 'claude-opus-5-5'
    input             REAL NOT NULL,
    output            REAL NOT NULL,
    cache_read        REAL NOT NULL,
    cache_write_5m    REAL NOT NULL,
    cache_write_1h    REAL NOT NULL,
    source            TEXT NOT NULL,          -- URL of the pricing page
    PRIMARY KEY (version, model)
);

-- One row per top-level session (a <session-id>.jsonl file).
CREATE TABLE sessions (
    session_id        TEXT PRIMARY KEY,
    project_dir       TEXT NOT NULL,
    first_ts          TEXT NOT NULL,
    last_ts           TEXT NOT NULL,
    entrypoint        TEXT,                   -- 'cli', 'sdk-ts', ...
    cc_version_first  TEXT,
    cc_version_last   TEXT,
    worktree          TEXT,                   -- .claude/worktrees/<name>, else NULL
    is_coordinator    INTEGER NOT NULL DEFAULT 0 CHECK (is_coordinator IN (0, 1)),
    forked_from       TEXT                    -- fork-context-ref.parentSessionId
);

-- One row per subagent or teammate (subagents/agent-<id>.jsonl plus its
-- .meta.json). Main-session requests have agent_id NULL instead.
CREATE TABLE agents (
    agent_id          TEXT PRIMARY KEY,
    session_id        TEXT NOT NULL REFERENCES sessions (session_id),
    agent_type        TEXT,                   -- meta.agentType
    custom_agent_type TEXT,                   -- meta.customAgentType (.claude/agents/<x>.md)
    name              TEXT,                   -- meta.name (addressable teammate name)
    model_alias       TEXT,                   -- meta.model ('opus', 'inherit', ...)
    task_kind         TEXT,                   -- meta.taskKind
    spawn_depth       INTEGER,
    request_shape     TEXT,                   -- meta.requestShape ('background', ...)
    worktree          TEXT,
    first_ts          TEXT,
    last_ts           TEXT,
    meta_missing      INTEGER NOT NULL DEFAULT 0 CHECK (meta_missing IN (0, 1))
);

-- What a request was spent on. Every request belongs to exactly one trigger.
-- Precedence and buckets are in transcript-format.md § Trigger classification.
-- trigger_id is the uuid of the turn-starting user record or queued_command
-- attachment, or '<request_id>:retry' for a retry, which has no record of its own.
CREATE TABLE triggers (
    trigger_id        TEXT PRIMARY KEY,
    session_id        TEXT NOT NULL REFERENCES sessions (session_id),
    agent_id          TEXT REFERENCES agents (agent_id),
    ts                TEXT NOT NULL,
    kind              TEXT NOT NULL CHECK (kind IN (
                          'human_prompt',
                          'background_completion',
                          'monitor_event',
                          'idle_notification',
                          'teammate_message',
                          'agent_message',
                          'cross_session_message',
                          'scheduled_task',
                          'local_command',
                          'subagent_prompt',
                          'compact_summary',
                          'auxiliary',
                          'retry',
                          'mixed',
                          'unknown')),
    origin_kind       TEXT,                   -- raw origin.kind, if present
    source_ref        TEXT,                   -- task id, teammate id or peer name; never message text
    rule              TEXT NOT NULL           -- precedence rule that matched, e.g. 'R3'
);

-- One row per API request, deduplicated by requestId keeping the FINAL
-- record (the first is a streaming partial with a short output count).
CREATE TABLE requests (
    request_id        TEXT PRIMARY KEY,
    session_id        TEXT NOT NULL REFERENCES sessions (session_id),
    agent_id          TEXT REFERENCES agents (agent_id),
    trigger_id        TEXT NOT NULL REFERENCES triggers (trigger_id),
    message_uuid      TEXT NOT NULL,          -- uuid of the final record kept
    records_seen      INTEGER NOT NULL CHECK (records_seen >= 1),
    ts                TEXT NOT NULL,
    model             TEXT NOT NULL,
    cc_version        TEXT,
    git_branch        TEXT,
    cwd_worktree      TEXT,
    input_tokens      INTEGER NOT NULL,
    output_tokens     INTEGER NOT NULL,
    thinking_tokens   INTEGER NOT NULL DEFAULT 0,
    cache_read        INTEGER NOT NULL,
    cache_write_5m    INTEGER NOT NULL,
    cache_write_1h    INTEGER NOT NULL,
    web_search        INTEGER NOT NULL DEFAULT 0,
    web_fetch         INTEGER NOT NULL DEFAULT 0,
    context_tokens    INTEGER NOT NULL,       -- input + cache_read + cache_write_5m + cache_write_1h
    prev_gap_s        REAL,                   -- seconds since the previous request in the same transcript
    is_api_error      INTEGER NOT NULL DEFAULT 0 CHECK (is_api_error IN (0, 1)),
    CHECK (thinking_tokens <= output_tokens),
    CHECK (context_tokens = input_tokens + cache_read + cache_write_5m + cache_write_1h)
);
CREATE INDEX requests_session_ts ON requests (session_id, ts);
CREATE INDEX requests_agent ON requests (agent_id);

-- One row per tool_use block, joined to its tool_result.
CREATE TABLE tool_calls (
    tool_use_id       TEXT PRIMARY KEY,
    request_id        TEXT NOT NULL REFERENCES requests (request_id),
    session_id        TEXT NOT NULL REFERENCES sessions (session_id),
    agent_id          TEXT REFERENCES agents (agent_id),
    ts                TEXT NOT NULL,
    tool_name         TEXT NOT NULL,          -- 'Bash', 'Read', 'mcp__ghidra__decompile_function', ...
    mcp_server        TEXT,
    fingerprint       TEXT,                   -- scrubbed: command head or repo-relative path, see transcript-format.md
    task_id           TEXT,                   -- agent id (Agent/Task) or background task id (Bash) the call started
    pr_ref            INTEGER,                -- the one PR the call names: `gh pr create|merge|checks|view N`,
                                              -- or a single #N in a background command's description
    result_chars      INTEGER,                -- NULL until the result is seen
    result_is_error   INTEGER CHECK (result_is_error IN (0, 1)),
    result_persisted  INTEGER NOT NULL DEFAULT 0 CHECK (result_persisted IN (0, 1)),  -- spilled to tool-results/
    later_requests    INTEGER,                -- requests in the same context after the result, until compaction
    exposure_chars    INTEGER                 -- result_chars * later_requests ("context exposure", not cost)
);
CREATE INDEX tool_calls_request ON tool_calls (request_id);
CREATE INDEX tool_calls_task ON tool_calls (task_id);

-- compact_boundary records.
CREATE TABLE compactions (
    boundary_uuid     TEXT PRIMARY KEY,
    session_id        TEXT NOT NULL REFERENCES sessions (session_id),
    agent_id          TEXT REFERENCES agents (agent_id),
    ts                TEXT NOT NULL,
    trigger           TEXT NOT NULL CHECK (trigger IN ('auto', 'manual', 'unknown')),
    pre_tokens        INTEGER,
    post_tokens       INTEGER,
    dropped_tokens    INTEGER,
    duration_ms       INTEGER,
    request_before    TEXT REFERENCES requests (request_id),
    request_after     TEXT REFERENCES requests (request_id)
);

-- pr-link records: the session created or touched this PR at this time.
CREATE TABLE pr_links (
    session_id        TEXT NOT NULL REFERENCES sessions (session_id),
    pr_number         INTEGER NOT NULL,
    repository        TEXT NOT NULL,
    ts                TEXT NOT NULL,
    PRIMARY KEY (session_id, pr_number, ts)
);

-- The LAST cost-state record of each session: Claude Code's own running
-- total, a reconciliation source besides OTel. It covers every request of
-- the process that wrote it (from process_start on), subagents included.
CREATE TABLE cost_states (
    session_id        TEXT PRIMARY KEY REFERENCES sessions (session_id),
    observed_ts       TEXT,
    total_cost_usd    REAL NOT NULL,
    has_unknown_cost  INTEGER NOT NULL CHECK (has_unknown_cost IN (0, 1)),
    lines_added       INTEGER,
    lines_removed     INTEGER,
    model_usage_json  TEXT NOT NULL,          -- modelUsage verbatim (numbers only)
    process_start     TEXT                    -- startTime as ISO-8601 UTC, NULL when absent (version 3)
);

-- PRs, from `gh pr list --state all --json ...`.
CREATE TABLE prs (
    pr_number         INTEGER PRIMARY KEY,
    head_branch       TEXT NOT NULL,
    head_sha          TEXT,
    merge_sha         TEXT,
    created_at        TEXT NOT NULL,
    merged_at         TEXT,
    closed_at         TEXT,                   -- set for MERGED and CLOSED; ends the branch-match window
    state             TEXT NOT NULL CHECK (state IN ('MERGED', 'CLOSED', 'OPEN')),
    additions         INTEGER,
    deletions         INTEGER,
    changed_files     INTEGER
);

-- Commits seen at the head of a branch, so ancestry attribution still works
-- after rm-worktree.sh has deleted a merged packet branch. Filled from three
-- sources, in attribution.md § A3: a ref snapshot on every ingest run, the
-- build lane's job log (worktree + commit per build), and the
-- "Merge branch '<name>'" subjects of merge commits on main.
CREATE TABLE branch_heads (
    branch            TEXT NOT NULL,
    commit_sha        TEXT NOT NULL,
    observed_at       TEXT NOT NULL,
    source            TEXT NOT NULL CHECK (source IN ('ref-snapshot', 'lane-log', 'merge-subject')),
    PRIMARY KEY (branch, commit_sha, source)
);

-- Request-to-PR attribution. For every request, the weights of its rows sum
-- to 1; the unattributed share is a row with pr_number NULL. Methods and
-- their precedence: attribution.md. attribution_imbalance must be empty at
-- the end of every ingest run, or the run fails.
CREATE TABLE pr_attribution (
    request_id        TEXT NOT NULL REFERENCES requests (request_id),
    pr_number         INTEGER REFERENCES prs (pr_number),
    method            TEXT NOT NULL CHECK (method IN (
                          'branch', 'ancestry', 'parent-session',
                          'trigger', 'pr-link', 'split', 'unattributed')),
    weight            REAL NOT NULL CHECK (weight > 0 AND weight <= 1),
    confidence        REAL NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    CHECK ((method = 'unattributed') = (pr_number IS NULL))
);
CREATE UNIQUE INDEX pr_attribution_key ON pr_attribution (request_id, COALESCE(pr_number, -1));

-- Requests whose attribution is missing or whose weights do not sum to 1.
CREATE VIEW attribution_imbalance AS
SELECT r.request_id, COALESCE(SUM(a.weight), 0) AS total_weight
FROM requests r
LEFT JOIN pr_attribution a ON a.request_id = r.request_id
GROUP BY r.request_id
HAVING ABS(COALESCE(SUM(a.weight), 0) - 1.0) > 1e-6;

-- Records whose shape the ingest does not recognise. A run with rows here
-- exits non-zero unless --allow-unknown is passed; they are never dropped silently.
CREATE TABLE unknown_shapes (
    shape             TEXT NOT NULL,          -- 'type', 'type/subtype' or 'attachment/<type>'
    cc_version        TEXT NOT NULL DEFAULT '',
    first_path        TEXT NOT NULL,
    first_line        INTEGER NOT NULL,
    count             INTEGER NOT NULL,
    PRIMARY KEY (shape, cc_version)
);
