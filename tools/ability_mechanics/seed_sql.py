"""Reading and writing the pg_dump-style seed SQL under ``db/resources/``.

Only what the generator needs: the literals of ``INSERT INTO t (cols)
VALUES (...)`` rows, with each literal's span so a single column can be
rewritten in place, and the file's own line ending.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Tuple

REPO_ROOT = Path(__file__).resolve().parents[2]

_BARE_LITERAL = re.compile(r"[^,)]+")


class InputError(Exception):
    """A seed file did not parse the way the tool expects (exit 2)."""


@dataclass
class Value:
    """One literal of a VALUES list: its decoded text and its span in the text."""

    text: Optional[str]  # None for NULL
    start: int
    end: int


def parse_values(s: str, i: int) -> Tuple[List[Value], int]:
    """Parse ``( v, v, ... )`` starting at ``s[i] == '('``; return the values
    and the offset just past the closing parenthesis."""
    if s[i] != "(":
        raise InputError(f"expected '(' at offset {i}")
    vals: List[Value] = []
    i += 1
    n = len(s)
    while i < n:
        c = s[i]
        if c in " \t\r\n,":
            i += 1
            continue
        if c == ")":
            return vals, i + 1
        if c == "'":
            j = i + 1
            buf = []
            while True:
                if j >= n:
                    raise InputError(f"unterminated string at offset {i}")
                if s[j] == "'":
                    if j + 1 < n and s[j + 1] == "'":
                        buf.append("'")
                        j += 2
                        continue
                    break
                buf.append(s[j])
                j += 1
            vals.append(Value("".join(buf), i, j + 1))
            i = j + 1
            continue
        m = _BARE_LITERAL.match(s, i)
        if not m:
            raise InputError(f"bad literal at offset {i}")
        tok = m.group(0).rstrip()
        vals.append(Value(None if tok.strip() == "NULL" else tok.strip(), i, i + len(tok)))
        i = m.end()
    raise InputError("unterminated VALUES list")


def sql_rows(text: str, table: str) -> List[Dict[str, Value]]:
    """Every ``INSERT INTO <table> (cols) VALUES (...)`` row of a seed text."""
    out = []
    for m in re.finditer(r"INSERT INTO %s \(([^)]*)\) VALUES " % re.escape(table), text):
        cols = [c.strip() for c in m.group(1).split(",")]
        vals, _ = parse_values(text, m.end())
        if len(vals) != len(cols):
            raise InputError(f"{table}: {len(cols)} columns but {len(vals)} values at offset {m.start()}")
        out.append(dict(zip(cols, vals)))
    return out


def sql_quote(s: Optional[str]) -> str:
    return "NULL" if s is None else "'" + s.replace("'", "''") + "'"


def read_text(path: Path) -> Tuple[str, str]:
    """The file's text with LF line ends, and the line end it was stored with.

    The seeds are stored CRLF; working in LF keeps the patterns simple, and
    ``write_text`` puts the original ending back so a run never rewrites a
    file's line endings.
    """
    raw = (REPO_ROOT / path).read_bytes().decode("utf-8")
    eol = "\r\n" if "\r\n" in raw else "\n"
    return raw.replace("\r\n", "\n"), eol


def write_text(path: Path, text: str, eol: str) -> None:
    (REPO_ROOT / path).write_bytes(text.replace("\n", eol).encode("utf-8"))
