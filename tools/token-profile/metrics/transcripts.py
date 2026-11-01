"""Reads the harness transcripts for the metrics subcommands: subagent dispatches, their briefs, requests and returns.

The shared definitions of the harness-tuning contract (C1): the report-path and return-line grammar, the dispatch
record, and the checks that a dispatch wrote its report or stopped on the context cap. Transcripts are found through
ingest.store.discover, so the window and the fork order match the profiler's.
"""

import json
import re
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path

from ingest.store import discover

RP = re.compile(r"[^\s`'\"()<>]*\.claude[\\/]reports[\\/][^\\/\s`'\"()<>]+[\\/][^\\/\s`'\"()<>]+\.md")
LINE = re.compile(r"^(?P<id>\S+)\s+(?P<status>done|blocked|failed)\s+(?P<sha>[0-9a-f]{7,40}|-)\s+"
                  r"(?P<report>\S+\.md|-)(?P<extra>(?:\s+[a-z_]+=\S+)*)\s*$")
SHORT_CHARS = 200
CAP_MARKER = "Context cap reached"  # the user-level Haiku cap's deny text (D-HT3); never changed by this campaign
WRITE_VERBS = ("WriteAllText", "Set-Content", "Out-File", "Add-Content", ">")
REPORT_TOOLS = ("Write", "Edit", "MultiEdit")
SHELL_TOOLS = ("PowerShell", "Bash")


@dataclass
class Request:
    """One API request: the max over the streamed lines that share its message id."""

    message_id: str
    input: int
    cache_read: int
    cache_create: int
    output: int

    @property
    def context(self) -> int:
        """The prompt size of the request: input + cache_read + cache_create."""
        return self.input + self.cache_read + self.cache_create


@dataclass
class ToolUse:
    """A tool_use block, with the index of the request that made it in `Dispatch.requests`."""

    id: str
    name: str
    input: dict
    request_index: int


@dataclass
class Dispatch:
    """One subagent transcript: who ran it, its brief and final text, its requests, tool uses and results."""

    session_id: str
    agent_id: str
    agent_type: str | None
    first_ts: str | None
    brief: str
    final_text: str
    requests: list[Request]
    tool_uses: list[ToolUse]
    results: dict[str, str]  # tool_use_id -> result text


def records(path) -> Iterator[dict]:
    """The JSON objects of a JSONL file, in order. A bad line is skipped; undecodable bytes are replaced."""
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            try:
                rec = json.loads(line)
            except ValueError:
                continue
            if isinstance(rec, dict):
                yield rec


def text_of(content) -> str:
    """A message content as text: the string itself, or the joined "text" fields of a list of blocks."""
    if isinstance(content, str):
        return content
    if not isinstance(content, list):
        return ""
    return "\n".join(b["text"] for b in content if isinstance(b, dict) and isinstance(b.get("text"), str))


def norm_path(p: str) -> str:
    """A path as a brief or a tool input writes it: quotes stripped, backslashes to slashes, lower case."""
    return p.strip("'\"").replace("\\", "/").lower()


def report_suffix(p: str) -> str:
    """The normalised path from '.claude/reports/' on, or "" when the path has no such part."""
    n = norm_path(p)
    i = n.find(".claude/reports/")
    return n[i:] if i >= 0 else ""


def _count(v) -> int:
    """A token count from a usage field; anything that is not an int counts as 0."""
    return v if isinstance(v, int) and not isinstance(v, bool) else 0


def _merge_usage(req: Request, usage) -> None:
    """Raise each field of `req` to the max of one streamed line's usage."""
    if not isinstance(usage, dict):
        return
    req.input = max(req.input, _count(usage.get("input_tokens")))
    req.cache_read = max(req.cache_read, _count(usage.get("cache_read_input_tokens")))
    req.cache_create = max(req.cache_create, _count(usage.get("cache_creation_input_tokens")))
    req.output = max(req.output, _count(usage.get("output_tokens")))


def _agent_type(meta: Path) -> str | None:
    """The agentType of an agent-<id>.meta.json, or None when the file is missing or unreadable."""
    try:
        data = json.loads(meta.read_text(encoding="utf-8", errors="replace"))
    except (OSError, ValueError):
        return None
    value = data.get("agentType") if isinstance(data, dict) else None
    return value if isinstance(value, str) else None


def read_dispatch(path, session_id, agent_id) -> Dispatch:
    """The dispatch in one subagent transcript. Its agent_type comes from the .meta.json beside it."""
    path = Path(path)
    requests: list[Request] = []
    index: dict[str, int] = {}
    tool_uses: list[ToolUse] = []
    seen: set[str] = set()
    results: dict[str, str] = {}
    brief = final_text = ""
    first_ts = None
    for n, rec in enumerate(records(path)):
        if n == 0:
            first_ts = rec.get("timestamp")
        msg = rec.get("message") if isinstance(rec.get("message"), dict) else {}
        content = msg.get("content")
        kind = rec.get("type")
        if kind == "user":
            if not brief and text_of(content):
                brief = text_of(content)
            for b in content if isinstance(content, list) else ():
                if isinstance(b, dict) and b.get("type") == "tool_result" and isinstance(b.get("tool_use_id"), str):
                    results[b["tool_use_id"]] = text_of(b.get("content"))
        elif kind == "assistant":
            if text_of(content):
                final_text = text_of(content)
            mid = msg.get("id") if isinstance(msg.get("id"), str) else f"line:{n}"
            if mid not in index:
                index[mid] = len(requests)
                requests.append(Request(mid, 0, 0, 0, 0))
            _merge_usage(requests[index[mid]], msg.get("usage"))
            for b in content if isinstance(content, list) else ():
                if isinstance(b, dict) and b.get("type") == "tool_use" and isinstance(b.get("id"), str) \
                        and b["id"] not in seen:
                    seen.add(b["id"])
                    inp = b.get("input") if isinstance(b.get("input"), dict) else {}
                    tool_uses.append(ToolUse(b["id"], str(b.get("name") or ""), inp, index[mid]))
    return Dispatch(session_id, agent_id, _agent_type(path.with_name(path.stem + ".meta.json")), first_ts,
                    brief, final_text, requests, tool_uses, results)


def iter_dispatches(projects_root, prefix, since=None, until=None) -> Iterator[Dispatch]:
    """The subagent dispatches under the project folders that start with `prefix`, whose first_ts is in
    [since, until). Dates are compared as strings, so `since` and `until` are YYYY-MM-DD."""
    for kind, path, _project, session_id, agent_id in discover(projects_root, prefix):
        if kind != "subagent":
            continue
        d = read_dispatch(path, session_id, agent_id)
        ts = d.first_ts or ""
        if (since is not None and ts < since) or (until is not None and ts >= until):
            continue
        yield d


def iter_main_transcripts(projects_root, prefix) -> Iterator[tuple[str, Path]]:
    """(session_id, path) for each main session transcript under the project folders that start with `prefix`."""
    for kind, path, _project, session_id, _agent in discover(projects_root, prefix):
        if kind == "main":
            yield session_id, Path(path)


def brief_report_paths(brief: str) -> list[str]:
    """The .claude/reports/<dir>/<file>.md paths a brief names, in order."""
    return [m.group(0) for m in RP.finditer(brief)]


def _unwrap(line: str) -> str:
    """A stripped line, without one pair of wrapping backticks."""
    s = line.strip()
    if len(s) >= 2 and s[0] == s[-1] == "`":
        s = s[1:-1].strip()
    return s


def return_line(final_text: str) -> tuple[str, re.Match | None]:
    """The kind of a dispatch's return line and its LINE match: strict, line_first, buried, prose, short or empty.
    The match is None unless a line matches (strict, line_first, buried)."""
    text = (final_text or "").strip()
    if not text:
        return "empty", None
    lines = [s for s in (_unwrap(line) for line in text.splitlines()) if s]
    matches = [(i, LINE.match(s)) for i, s in enumerate(lines)]
    hit = next(((i, m) for i, m in matches if m), None)
    if hit is None:
        return ("prose" if len(text) >= SHORT_CHARS else "short"), None
    i, m = hit
    if i > 0:
        return "buried", m
    return ("strict" if len(lines) == 1 else "line_first"), m


def cap_denied(d: Dispatch) -> list[ToolUse]:
    """The tool uses whose result holds the user-level cap's deny text."""
    return [t for t in d.tool_uses if CAP_MARKER in d.results.get(t.id, "")]


def is_context_stop(d: Dispatch) -> bool:
    """True when the dispatch stopped on the context: a blocked return that names the context, or a cap denial."""
    if cap_denied(d):
        return True
    _, m = return_line(d.final_text)
    if m is not None:
        blocked = m.group("status") == "blocked"
        has_reason = re.search(r"\sreason=context(?!\S)", m.group("extra") or "") is not None
    else:
        blocked = re.search(r"\bblocked\b", d.final_text, re.IGNORECASE) is not None
        has_reason = False
    return blocked and (has_reason or "context" in d.final_text.lower())


def report_written(d: Dispatch, paths: list[str]) -> bool:
    """True when the dispatch wrote one of the brief's report paths: with Write, Edit or MultiEdit, or with a
    shell command that holds a write verb and the file's name."""
    suffixes = {report_suffix(p) for p in paths} - {""}
    names = {Path(norm_path(p)).name for p in paths}
    for t in d.tool_uses:
        if t.name in REPORT_TOOLS:
            target = t.input.get("file_path")
            if isinstance(target, str) and report_suffix(target) in suffixes:
                return True
        elif t.name in SHELL_TOOLS:
            cmd = t.input.get("command")
            if isinstance(cmd, str) and any(v in cmd for v in WRITE_VERBS) and any(n in cmd.lower() for n in names):
                return True
    return False


def report_writer_types(repo) -> set[str]:
    """The agent types under <repo>/.claude/agents that write a report: a "## Reporting" section and a model
    that does not contain haiku. An agent with no file is never counted."""
    out: set[str] = set()
    for f in sorted((Path(repo) / ".claude" / "agents").glob("*.md")):
        lines = f.read_text(encoding="utf-8", errors="replace").splitlines()
        if not lines or lines[0].strip() != "---":
            continue
        end = next((i for i in range(1, len(lines)) if lines[i].strip() == "---"), None)
        if end is None:
            continue
        model = next((ln.strip().split(":", 1)[1].strip().lower() for ln in lines[1:end]
                      if ln.strip().startswith("model:")), "")
        if "## Reporting" in "\n".join(lines[end + 1:]) and "haiku" not in model:
            out.add(f.stem)
    return out
