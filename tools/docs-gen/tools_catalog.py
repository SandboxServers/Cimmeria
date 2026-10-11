"""Render the tools catalog: every script under tools/ and every crate binary, grouped by task.

The `gen:tools-catalog` block in tools/README.md is this module's output, so an agent
looking for "a UPK dumper" or "a pcap decoder" finds the one that already exists
instead of writing another. Each entry is a source path, how to run it, and its
summary: the first sentence of the file's leading docstring or comment (`//!` for a
Rust binary, `.SYNOPSIS` for PowerShell). A file with none shows NO_SUMMARY, so the
gap stays visible until someone writes one.

What counts as a tool, how each kind is run and which group it lands in are data,
in tools_catalog.toml beside this file. Files come from `git ls-files` (a walk of the
tree when the root is not a git checkout, as in the unit tests).
"""
from __future__ import annotations

import ast
import os
import pathlib
import re
import subprocess
import tomllib
from dataclasses import dataclass
from fnmatch import fnmatchcase
from posixpath import dirname

CONFIG = pathlib.Path(__file__).with_name("tools_catalog.toml")
NO_SUMMARY = "(no summary)"
OTHER = "Other"
#: Longest summary kept; longer first sentences are cut at a word boundary.
MAX_SUMMARY = 220
#: A first sentence shorter than this takes the next one with it.
MIN_SUMMARY = 40
WALK_SKIP = frozenset({".git", ".claude", "target", "node_modules", "__pycache__", "dist", "external"})
MAIN_GUARD_RE = re.compile(r"""^if\s+__name__\s*==\s*['"]__main__['"]\s*:""", re.M)
SENTENCE_END_RE = re.compile(r"(?<=[.!?])\s+(?=[A-Z`(\[])")
#: Paragraphs that are not a description: a usage line, a code fence, a doc keyword.
NOT_PROSE_RE = re.compile(r"^(usage\b|```|\.[A-Z]+\b)", re.I)
UNDERLINE_RE = re.compile(r"^(=+|-{3,})$")
LIST_ITEM_RE = re.compile(r"^([-*+]|\d+[.)])\s")


@dataclass
class Tool:
    path: str  # repo-relative source path; a package directory for a __main__.py
    run: str
    summary: str  # "" when the file has none


def load_config(path: pathlib.Path = CONFIG) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def list_files(root: pathlib.Path) -> list[str]:
    """Repo-relative POSIX paths of the files under tools/ and crates/."""
    try:
        out = subprocess.run(
            ["git", "-C", str(root), "ls-files", "-z", "--", "tools", "crates"],
            capture_output=True,
            check=True,
        ).stdout.decode("utf-8")
        files = [f for f in out.split("\0") if f and (root / f).is_file()]
        if files:
            return sorted(files)
    except (OSError, subprocess.CalledProcessError):
        pass
    found = []
    for top in ("tools", "crates"):
        for dirpath, dirnames, filenames in os.walk(root / top):
            dirnames[:] = sorted(d for d in dirnames if d not in WALK_SKIP)
            for f in filenames:
                found.append((pathlib.Path(dirpath) / f).relative_to(root).as_posix())
    return sorted(found)


# ── summaries ────────────────────────────────────────────────────────────────


def summarize(paragraphs: list[str]) -> str:
    """The first sentence of the first prose paragraph, as one table-safe line."""
    for para in paragraphs:
        para = " ".join(para.split())
        if para and not NOT_PROSE_RE.match(para):
            break
    else:
        return ""
    sentences = SENTENCE_END_RE.split(para, maxsplit=2)
    sentence = sentences[0]
    if len(sentence) < MIN_SUMMARY and len(sentences) > 1 and len(sentence) + len(sentences[1]) < MAX_SUMMARY:
        sentence += " " + sentences[1]  # "SUPERSEDED (NA21)." says too little alone
    if len(sentence) > MAX_SUMMARY:
        sentence = sentence[:MAX_SUMMARY].rsplit(" ", 1)[0].rstrip(",;:") + " …"
    if sentence.endswith(":"):
        sentence = sentence[:-1] + "."  # the list it introduced is not in the summary
    if sentence.count("`") % 2:
        sentence += "`"  # a cut code span would swallow the rest of the row
    return escape_cell(sentence)


def escape_cell(text: str) -> str:
    """Make text safe in a Markdown table cell.

    `|` would end the cell anywhere. `<` and `>` outside code spans would be read as
    HTML, so `lab calibrate <section>` would lose its placeholder on GitHub; inside a
    code span they are literal and stay as they are.
    """
    parts = text.split("`")
    for i in range(0, len(parts), 2):  # even parts are outside code spans
        parts[i] = parts[i].replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    return "`".join(parts).replace("|", "\\|")


def split_paragraphs(lines: list[str]) -> list[str]:
    """Blank-line separated paragraphs, each joined into one line.

    A list item starts a new paragraph, so a summary stops before the list. A
    title underlined with `===` or `---` is dropped when prose follows it.
    """
    paras: list[str] = []
    titles: list[str] = []
    cur: list[str] = []

    def flush() -> None:
        if cur:
            text = cur[0]
            for line in cur[1:]:
                # rejoin a word hyphenated across lines ("at-or-" + "after")
                text += line if re.search(r"[A-Za-z]-$", text) else " " + line
            paras.append(text)
            cur.clear()

    for line in lines:
        s = line.strip()
        if UNDERLINE_RE.match(s):
            if cur:
                titles.append(" ".join(cur))
                cur.clear()
            continue
        if not s:
            flush()
            continue
        if LIST_ITEM_RE.match(s):
            flush()
        cur.append(s)
    flush()
    return paras or titles


def leading_line_comments(lines: list[str], prefix: str, skip: tuple[str, ...] = ()) -> list[str]:
    """The comment lines before the first line of code, `prefix` stripped."""
    out: list[str] = []
    for line in lines:
        s = line.strip()
        if any(s.startswith(p) for p in skip):
            continue
        if not s:
            if out:
                out.append("")
            continue
        if not s.startswith(prefix):
            break
        out.append(s[len(prefix) :].strip())
    return out


def python_summary(text: str) -> str:
    try:
        doc = ast.get_docstring(ast.parse(text))
    except (SyntaxError, ValueError):
        doc = None
    if doc:
        return summarize(split_paragraphs(doc.splitlines()))
    lines = text.splitlines()
    return summarize(split_paragraphs(leading_line_comments(lines, "#", skip=("#!", "# -*-", "# noqa", "# type:"))))


def powershell_summary(text: str) -> str:
    lines = text.splitlines()
    i = 0
    while i < len(lines) and (not lines[i].strip() or lines[i].lstrip().lower().startswith(("#requires", "#!"))):
        i += 1
    if i < len(lines) and lines[i].lstrip().startswith("<#"):
        block: list[str] = []
        for line in lines[i:]:
            block.append(line.replace("<#", "", 1) if not block else line)
            if "#>" in line:
                block[-1] = block[-1].split("#>", 1)[0]
                break
        keyword = None
        sections: dict[str | None, list[str]] = {}
        for line in block:
            m = re.match(r"^\s*\.([A-Z]+)\b", line)
            if m:
                keyword = m.group(1)
                continue
            sections.setdefault(keyword, []).append(line)
        body = sections.get("SYNOPSIS") or sections.get(None) or sections.get("DESCRIPTION") or []
        return summarize(split_paragraphs(body))
    return summarize(split_paragraphs(leading_line_comments(lines[i:], "#")))


def shell_summary(text: str) -> str:
    lines = text.splitlines()
    return summarize(split_paragraphs(leading_line_comments(lines, "#", skip=("#!", "# shellcheck"))))


def java_summary(text: str) -> str:
    m = re.search(r"/\*\*?(.*?)\*/", text, re.S)
    if not m:
        return ""
    lines = [re.sub(r"^\s*\*\s?", "", l) for l in m.group(1).splitlines()]
    return summarize(split_paragraphs(lines))


def rust_summary(text: str) -> str:
    """First paragraph of the `//!` crate doc; a leading `//` block when there is none."""
    lines = text.splitlines()
    doc = leading_line_comments(lines, "//!", skip=("#![",))
    if doc:
        return summarize(split_paragraphs(doc))
    plain = leading_line_comments(lines, "//", skip=("#![",))
    return summarize(split_paragraphs([l for l in plain if not l.startswith("/")]))


SUMMARIZERS = {
    ".py": python_summary,
    ".ps1": powershell_summary,
    ".sh": shell_summary,
    ".java": java_summary,
    ".rs": rust_summary,
}


def read(root: pathlib.Path, rel: str) -> str:
    return (root / rel).read_bytes().decode("utf-8", errors="replace").replace("\r\n", "\n")


# ── discovery ────────────────────────────────────────────────────────────────


def is_python_entry(rel: str, text: str) -> bool:
    return dirname(rel) == "tools" or rel.endswith("/__main__.py") or bool(MAIN_GUARD_RE.search(text))


def script_tools(root: pathlib.Path, files: list[str], cfg: dict) -> list[Tool]:
    tools = []
    fileset = set(files)
    for rel in files:
        if not rel.startswith("tools/"):
            continue
        ext = pathlib.PurePosixPath(rel).suffix
        if ext not in cfg["run"] or any(fnmatchcase(rel, p) for p in cfg.get("exclude", ())):
            continue
        text = read(root, rel)
        if ext == ".py" and not is_python_entry(rel, text):
            continue
        summary = SUMMARIZERS[ext](text)
        path = rel
        if rel.endswith("/__main__.py"):
            # A package run as a directory: its __init__.py says what it is, and its
            # __main__.py usually only says how to run it.
            path = dirname(rel) + "/"
            init = f"{dirname(rel)}/__init__.py"
            if init in fileset:
                summary = python_summary(read(root, init)) or summary
        template = next(
            (o["run"] for o in cfg.get("run_override", ()) if fnmatchcase(rel, o["pattern"])),
            cfg["run"][ext],
        )
        run = template.format(path=rel, dir=dirname(rel), stem=pathlib.PurePosixPath(rel).stem)
        tools.append(Tool(path, run, summary))
    return tools


def _workspace_manifest(manifest: str, manifests: dict[str, dict]) -> str | None:
    """The nearest enclosing non-root workspace's Cargo.toml, or None for the root workspace."""
    d = dirname(manifest)
    while d:
        candidate = f"{d}/Cargo.toml"
        if "workspace" in manifests.get(candidate, {}):
            return candidate
        d = dirname(d)
    return None


def crate_bins(root: pathlib.Path, files: list[str]) -> list[Tool]:
    """Every binary target of every package under crates/ and tools/, as cargo infers them."""
    fileset = set(files)
    manifests = {
        rel: tomllib.loads(read(root, rel))
        for rel in files
        if rel.endswith("/Cargo.toml") and rel.startswith(("crates/", "tools/"))
    }
    tools = []
    for manifest, data in sorted(manifests.items()):
        package = data.get("package")
        if not package:
            continue
        pkg = package["name"]
        base = dirname(manifest)
        bins: dict[str, str] = {}  # source path -> bin name
        for b in data.get("bin", ()):
            name = b["name"]
            path = b.get("path")
            if path is None:
                for guess in (f"src/bin/{name}.rs", f"src/bin/{name}/main.rs", "src/main.rs"):
                    if f"{base}/{guess}" in fileset:
                        path = guess
                        break
            if path:
                bins[f"{base}/{path}"] = name
        if package.get("autobins", True):
            main = f"{base}/src/main.rs"
            if main in fileset and main not in bins and pkg not in bins.values():
                bins[main] = pkg
            prefix = f"{base}/src/bin/"
            for rel in files:
                if not rel.startswith(prefix) or rel in bins:
                    continue
                rest = rel[len(prefix) :]
                if "/" not in rest and rest.endswith(".rs"):
                    bins[rel] = rest[:-3]
                elif rest.count("/") == 1 and rest.endswith("/main.rs"):
                    bins[rel] = rest.split("/")[0]
        workspace = _workspace_manifest(manifest, manifests)
        for path, name in sorted(bins.items()):
            if path not in fileset:
                continue
            run = f"cargo run -p {pkg} --bin {name} --"
            if workspace:
                run = f"cargo run --manifest-path {workspace} -p {pkg} --bin {name} --"
            tools.append(Tool(path, run, rust_summary(read(root, path))))
    return tools


# ── rendering ────────────────────────────────────────────────────────────────


def group_of(path: str, groups: list[dict]) -> str:
    for g in groups:
        if any(fnmatchcase(path, p) for p in g["patterns"]):
            return g["title"]
    return OTHER


def collect(root: pathlib.Path, cfg: dict, files: list[str] | None = None) -> list[Tool]:
    files = list_files(root) if files is None else files
    return script_tools(root, files, cfg) + crate_bins(root, files)


def render(root: pathlib.Path, cfg: dict | None = None, files: list[str] | None = None) -> str:
    cfg = load_config() if cfg is None else cfg
    tools = collect(root, cfg, files)
    groups = cfg.get("group", [])
    titles = [g["title"] for g in groups] + [OTHER]
    by_group: dict[str, list[Tool]] = {t: [] for t in titles}
    for tool in tools:
        by_group[group_of(tool.path, groups)].append(tool)

    scripts = sum(1 for t in tools if not t.run.startswith("cargo "))
    missing = sum(1 for t in tools if not t.summary)
    out = [
        f"{scripts} scripts and {len(tools) - scripts} crate binaries; "
        f"{missing} without a summary line, marked {NO_SUMMARY}.",
        "",
        "| Group | Tools |",
        "|---|---:|",
    ]
    out += [f"| {t} | {len(by_group[t])} |" for t in titles if by_group[t]]
    for title in titles:
        entries = sorted(by_group[title], key=lambda t: t.path)
        if not entries:
            continue
        out += ["", f"### {title}", "", "| Tool | Run | What it does |", "|---|---|---|"]
        for t in entries:
            out.append(f"| `{t.path}` | `{t.run}` | {t.summary or NO_SUMMARY} |")
    return "\n".join(out)
