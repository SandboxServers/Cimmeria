#!/usr/bin/env python3
"""Ability telemetry coverage matrix (ability-mechanics AB-C7).

Every ability-related entity method, crossed with the four places a cast
is accounted for: the server's receipt row, the server's wire-send row, the
client's send hook and the client's receive hook. The plan is
``docs/analysis/ability-mechanics/lab-uat-and-telemetry.md`` (Part 2,
AB-C7, and the Acceptance's "Coverage" line).

Inputs, all read, none written except the matrix:

* the four ``docs/protocol/*-dispatch-table.md`` files: every method of
  :data:`ABILITY_METHODS` must be in its table at its index, and every other
  ability-looking row in the tables must be in :data:`NOT_IN_SET` with a
  reason, so a new ability method in a table forces a decision here;
* the code tables each side keeps for this script (``SOURCES``): the const
  arrays are scanned for their ``index`` / ``name`` pairs, so a hook or row
  that is not in its table does not count;
* the client's declaration of what it must hook (``CLIENT_DECLARED``), which
  must equal this set minus the client exceptions; the Rust test
  ``ability_trace::coverage`` then proves every declared method resolves
  through the real hook tables.

Every cell of a method that travels in that column's direction is either
filled or listed in :data:`EXCEPTIONS` with a reason. An exception for a
cell that has since been filled is stale and fails the check too.

Usage (from the repo root, stock Python 3):

    python tools/telemetry-coverage/abilities.py           # write the matrix
    python tools/telemetry-coverage/abilities.py --check   # exit 1 on drift or a gap

Exit codes: 0 ok, 1 drift or an unexplained gap (``--check``), 2 bad input.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Set, Tuple

ROOT = Path(__file__).resolve().parents[2]
TOOL = "tools/telemetry-coverage/abilities.py"
MATRIX = ROOT / "docs/analysis/ability-mechanics/telemetry-coverage.md"

CELL_TABLE = "docs/protocol/cell-method-dispatch-table.md"
CLIENT_TABLE = "docs/protocol/client-method-dispatch-table.md"
BASE_TABLE = "docs/protocol/sgwplayer-base-method-dispatch-table.md"
MESSAGE_TABLE = "docs/protocol/message-dispatch-table.md"
TABLES = (CELL_TABLE, CLIENT_TABLE, BASE_TABLE, MESSAGE_TABLE)

C2S = "client_to_server"
S2C = "server_to_client"

SERVER_RECV = "server_recv"
SERVER_SEND = "server_send"
CLIENT_SEND = "client_send"
CLIENT_RECV = "client_recv"
COLUMNS = (SERVER_RECV, SERVER_SEND, CLIENT_SEND, CLIENT_RECV)
COLUMN_TITLES = {
    SERVER_RECV: "Server recv row",
    SERVER_SEND: "Server send row",
    CLIENT_SEND: "Client send hook",
    CLIENT_RECV: "Client recv hook",
}
# Which columns a direction travels through.
TRAVELS = {
    C2S: (CLIENT_SEND, SERVER_RECV),
    S2C: (SERVER_SEND, CLIENT_RECV),
}


@dataclass(frozen=True)
class Method:
    name: str
    index: int
    direction: str
    table: str
    note: str = ""


# The ability method set (plan, Part 2, AB-C1 and AB-C3, and AB-N0's
# gmDebug* list). Indices are flat: cell-method indices for client-to-server
# methods, SGWPlayer client-method indices for server-to-client ones.
#
# Deliberately not in the set (each also listed in NOT_IN_SET below):
# gmDebug 173-175 are the minigame debug commands, not ability ones; the
# AB-N2 grant commands (136, 153, 154, 158) change what a player knows
# rather than cast; setAutoCycle (83) arms the auto-attack loop, whose casts
# arrive as ordinary useAbility receipts on the server.
ABILITY_METHODS: Tuple[Method, ...] = (
    # Client to server (cell methods).
    Method("toggleCombatDebug", 2, C2S, CELL_TABLE),
    Method("toggleCombatVerboseDebug", 3, C2S, CELL_TABLE),
    Method("confirmationResponse", 4, C2S, CELL_TABLE),
    Method("useAbility", 68, C2S, CELL_TABLE),
    Method("useAbilityOnGroundTarget", 69, C2S, CELL_TABLE),
    Method("resetMyAbilities", 72, C2S, CELL_TABLE, "sent as Event_NetOut_RespecAbility"),
    Method("trainAbility", 77, C2S, CELL_TABLE),
    Method("petInvokeAbility", 88, C2S, CELL_TABLE),
    Method("petAbilityToggle", 89, C2S, CELL_TABLE),
    Method("gmDebugAbility", 169, C2S, CELL_TABLE, "SGWGmPlayer only"),
    Method("gmDebugCombat", 170, C2S, CELL_TABLE, "SGWGmPlayer only"),
    Method("gmDebugCombatVerbose", 171, C2S, CELL_TABLE, "SGWGmPlayer only"),
    Method("gmDebugHeal", 172, C2S, CELL_TABLE, "SGWGmPlayer only"),
    Method("gmDebugAbilityOnMob", 176, C2S, CELL_TABLE, "SGWGmPlayer only"),
    # Server to client (client methods).
    Method("onSequence", 1, S2C, CLIENT_TABLE, "includes Ability_Interrupt (event 1002)"),
    Method("onTimerUpdate", 12, S2C, CLIENT_TABLE),
    Method("onEffectResults", 14, S2C, CLIENT_TABLE, "effect id = cast_id"),
    Method("onStateFieldUpdate", 19, S2C, CLIENT_TABLE),
    Method("onStatUpdate", 20, S2C, CLIENT_TABLE),
    Method("onStatBaseUpdate", 21, S2C, CLIENT_TABLE),
    Method("onPlayerCommunication", 28, S2C, CLIENT_TABLE, "feedback channel only"),
    Method("onKnownAbilitiesUpdate", 101, S2C, CLIENT_TABLE),
    Method("onErrorCode", 121, S2C, CLIENT_TABLE),
    Method("onAbilityTreeInfo", 141, S2C, CLIENT_TABLE),
)

# Ability-looking dispatch-table rows that are not in the set, and why.
NOT_IN_SET: Dict[str, str] = {
    "onEffectUserData": "client method 13: effect user data for the UI, sent by no ability path today",
    "giveAbility": "client method 118: bound to no client handler (client-method table, handler bindings)",
    "onPetAbilityList": "SGWPet client method 29: the pet's ability list, a pet-campaign message, not a cast",
    "gmGiveAbility": "GM 136 (AB-N2): changes what a player knows, not a cast",
    "gmResetAbilities": "GM 153 (AB-N2): changes what a player knows, not a cast",
    "gmGiveAllAbilities": "GM 154 (AB-N2): changes what a player knows, not a cast",
    "gmSetMobAbilitySet": "GM 158 (AB-N2): changes a mob's ability set, not a cast",
    "listAbilities": "GM 123: lists what a player knows into the feedback channel, not a cast",
    "loadAbility": "GM 197: content reload",
    "loadAbilitySet": "GM 199: content reload",
    "gmDebugStartMinigame": "GM 173: minigame debug, not ability debug",
    "gmDebugSpectateMinigame": "GM 174: minigame debug, not ability debug",
    "gmDebugJoinMinigame": "GM 175: minigame debug, not ability debug",
    "toggleHealDebug": (
        "cell 6: no client event is bound to it (findings/native-combat-debug.md); "
        "gmDebugHeal (172) is the heal-debug method the client sends"
    ),
    "toggleCombatLOS": "GM 217: an LOS toggle, not a cast",
}

# Rows of the tables that name ability-ish methods: what NOT_IN_SET guards.
ABILITY_LOOKING = re.compile(r"(?i)abilit|effect|combatdebug|healdebug|combatlos")

# Where each column's code table lives: (path, const name). A missing file
# is an empty column (and every cell of it then needs an exception).
SOURCES: Dict[str, Tuple[str, str]] = {
    SERVER_RECV: ("crates/cell/src/cell/dispatch/ability_receipt.rs", "ABILITY_RECEIPTS"),
    SERVER_SEND: ("crates/cell-combat/src/cell/abilities/wire_ledger/coverage.rs", "LEDGER_METHODS"),
    CLIENT_SEND: ("crates/client-telemetry/src/hooks/ability_trace/decode.rs", "ALLOWLIST"),
    CLIENT_RECV: ("crates/client-telemetry/src/hooks/ability_trace/recv_methods.rs", "METHODS"),
}

# What each column's table entry means, for the matrix cell.
FILLED_TEXT = {
    SERVER_RECV: "receipt row",
    SERVER_SEND: "`abilities.wire` row",
    CLIENT_SEND: "`client.ability.sent`",
    CLIENT_RECV: "`client.ability.recv`",
}

# The client's declaration of what it must hook (Rust test
# `ability_trace::coverage` proves each one resolves in the hook tables).
CLIENT_DECLARED: Dict[str, Tuple[str, str]] = {
    CLIENT_SEND: ("crates/client-telemetry/src/hooks/ability_trace/coverage.rs", "CLIENT_SENDS"),
    CLIENT_RECV: ("crates/client-telemetry/src/hooks/ability_trace/coverage.rs", "CLIENT_RECVS"),
}

# Cells of a travelling direction that stay empty on purpose.
EXCEPTIONS: Dict[Tuple[str, str], str] = {
    ("toggleCombatDebug", CLIENT_SEND): (
        "the stock client has no event bound to it and cannot send it "
        "(findings/native-combat-debug.md); the server still logs a crafted call"
    ),
    ("toggleCombatVerboseDebug", CLIENT_SEND): (
        "the stock client has no event bound to it and cannot send it "
        "(findings/native-combat-debug.md); the server still logs a crafted call"
    ),
    ("onStatBaseUpdate", SERVER_SEND): (
        "sent only by world entry and the respawn resync as a full base-stat burst, "
        "never by a cast; a cast moves current values (onStatUpdate)"
    ),
    ("onKnownAbilitiesUpdate", SERVER_SEND): (
        "sent by grants, training, respec and world entry, not by a cast; "
        "those paths log their own grant rows"
    ),
    ("onAbilityTreeInfo", SERVER_SEND): (
        "sent by world entry and the respawn resync, not by a cast"
    ),
}


class InputError(Exception):
    pass


# --- dispatch tables -------------------------------------------------------

@dataclass(frozen=True)
class TableRow:
    table: str
    section: str
    index: int
    name: str


_IDENT = re.compile(r"^`?([A-Za-z_][A-Za-z0-9_]*)`?(?:\(|$|\s)")


def parse_table(path: str) -> List[TableRow]:
    """The method rows of the dispatch table at ``path``."""
    return parse_table_text((ROOT / path).read_text(encoding="utf-8"), path)


def parse_table_text(text: str, path: str) -> List[TableRow]:
    """Every markdown table row that starts with a numeric index and names a
    method in a later cell (plain, backticked, or ``name(args)``)."""
    rows: List[TableRow] = []
    section = ""
    for line in text.splitlines():
        if line.startswith("#"):
            section = line.lstrip("#").strip()
            continue
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 2 or not cells[0].isdigit():
            continue
        for cell in cells[1:]:
            if re.fullmatch(r"0x[0-9A-Fa-f]+.*", cell):
                continue  # a wire byte column
            m = _IDENT.match(cell)
            if m:
                rows.append(TableRow(path, section, int(cells[0]), m.group(1)))
                break
    return rows


# --- code tables -----------------------------------------------------------

def const_block(text: str, const: str) -> Optional[str]:
    """The body of ``const NAME ... = &[`` up to its closing ``];`` at the
    start of a line."""
    m = re.search(r"const\s+" + re.escape(const) + r"\s*:[^=]*=\s*&\[", text)
    if not m:
        return None
    end = re.compile(r"^\];", re.M).search(text, m.end())
    if not end:
        raise InputError(f"const {const}: no closing `];`")
    return text[m.end():end.start()]


def scan_const(path: str, const: str) -> Optional[Dict[str, int]]:
    """``{name: index}`` for a const table of the file at ``path``, or
    ``None`` when the file or the const does not exist."""
    p = ROOT / path
    if not p.exists():
        return None
    return scan_const_text(p.read_text(encoding="utf-8"), const, path)


def scan_const_text(text: str, const: str, path: str = "<text>") -> Optional[Dict[str, int]]:
    """``{name: index}`` for a const table in ``text``, or ``None`` when the
    const does not exist. Understands ``generic(N, "name")``-style calls,
    ``(N, "name")`` tuples, and struct literals with an ``index`` /
    ``cell_index`` field and a ``name`` / ``method`` field, in either order;
    an index may be a ``u16`` const of the same file."""
    body = const_block(text, const)
    if body is None:
        return None
    consts = {
        m.group(1): int(m.group(2))
        for m in re.finditer(r"const\s+([A-Z][A-Z0-9_]*)\s*:\s*u16\s*=\s*(\d+)\s*;", text)
    }
    token = re.compile(
        r"(?P<call>(?:\b[a-z_]+)?\(\s*(?P<cidx>\d+)\s*,\s*\"(?P<cname>\w+)\"\s*\))"
        r"|(?:\b(?:cell_)?index\s*:\s*(?P<idx>\w+))"
        r"|(?:\b(?:name|method)\s*:\s*\"(?P<name>\w+)\")"
    )
    out: Dict[str, int] = {}
    pend_idx: Optional[int] = None
    pend_name: Optional[str] = None
    for m in token.finditer(body):
        if m.group("call"):
            out[m.group("cname")] = int(m.group("cidx"))
            continue
        if m.group("idx") is not None:
            raw = m.group("idx")
            if raw.isdigit():
                pend_idx = int(raw)
            elif raw in consts:
                pend_idx = consts[raw]
            else:
                raise InputError(f"{path} {const}: unknown index const {raw}")
        else:
            pend_name = m.group("name")
        if pend_idx is not None and pend_name is not None:
            out[pend_name] = pend_idx
            pend_idx = pend_name = None
    return out


# --- the matrix ------------------------------------------------------------

@dataclass
class Cell:
    state: str  # "filled", "exception", "missing", "n/a"
    text: str


@dataclass
class Result:
    rows: List[Tuple[Method, List[TableRow], Dict[str, Cell]]]
    problems: List[str]
    gaps: List[Tuple[str, str, str]]  # (method, column, reason)


def build() -> Result:
    problems: List[str] = []
    table_rows: List[TableRow] = []
    for t in TABLES:
        if not (ROOT / t).exists():
            raise InputError(f"missing dispatch table {t}")
        table_rows += parse_table(t)

    names = {m.name for m in ABILITY_METHODS}
    if len(names) != len(ABILITY_METHODS):
        problems.append("ABILITY_METHODS lists a method twice")

    # Every ability-looking row of the tables is in the set or explained.
    for r in table_rows:
        if ABILITY_LOOKING.search(r.name) and r.name not in names and r.name not in NOT_IN_SET:
            problems.append(
                f"{r.table} ({r.section}) row {r.index} `{r.name}` looks ability-related: "
                f"add it to ABILITY_METHODS or NOT_IN_SET with a reason"
            )
    for n in sorted(NOT_IN_SET):
        if n in names:
            problems.append(f"`{n}` is in both ABILITY_METHODS and NOT_IN_SET")
        elif not any(r.name == n for r in table_rows):
            problems.append(f"NOT_IN_SET `{n}` is in no dispatch table (stale)")

    scanned: Dict[str, Optional[Dict[str, int]]] = {
        col: scan_const(*src) for col, src in SOURCES.items()
    }

    for (name, col), _ in EXCEPTIONS.items():
        if name not in names:
            problems.append(f"exception for unknown method `{name}`")
        elif col not in TRAVELS[next(m for m in ABILITY_METHODS if m.name == name).direction]:
            problems.append(f"exception `{name}` / {col}: the method does not travel through that column")

    rows = []
    gaps = []
    for m in ABILITY_METHODS:
        found = [r for r in table_rows if r.name == m.name]
        if not any(r.table == m.table and r.index == m.index for r in found):
            problems.append(
                f"`{m.name}` is not in {m.table} at index {m.index} "
                f"(found: {[(r.table, r.index) for r in found] or 'nowhere'})"
            )
        cells: Dict[str, Cell] = {}
        for col in COLUMNS:
            if col not in TRAVELS[m.direction]:
                cells[col] = Cell("n/a", "")
                continue
            table = scanned[col] or {}
            exc = EXCEPTIONS.get((m.name, col))
            if m.name in table:
                if table[m.name] != m.index:
                    problems.append(
                        f"{SOURCES[col][0]} {SOURCES[col][1]}: `{m.name}` at index "
                        f"{table[m.name]}, the dispatch table says {m.index}"
                    )
                if exc:
                    problems.append(f"stale exception: `{m.name}` / {col} is filled now")
                cells[col] = Cell("filled", FILLED_TEXT[col])
            elif exc:
                cells[col] = Cell("exception", "none: " + exc)
                gaps.append((m.name, col, exc))
            else:
                cells[col] = Cell("missing", "MISSING")
                problems.append(
                    f"`{m.name}` has no {COLUMN_TITLES[col].lower()} "
                    f"({SOURCES[col][0]} {SOURCES[col][1]}) and no exception"
                )
        rows.append((m, found, cells))

    # Code-table entries that are not in the set are fine (a hook may cover
    # more than the set), but the client's declaration must equal the set
    # minus its exceptions.
    for col, (path, const) in CLIENT_DECLARED.items():
        declared = scan_const(path, const)
        if declared is None:
            if (ROOT / SOURCES[col][0]).exists():
                problems.append(f"{path} {const} is missing")
            continue
        want = {
            m.name: m.index
            for m in ABILITY_METHODS
            if col in TRAVELS[m.direction] and (m.name, col) not in EXCEPTIONS
        }
        if declared != want:
            extra = sorted(set(declared.items()) - set(want.items()))
            lack = sorted(set(want.items()) - set(declared.items()))
            problems.append(f"{path} {const} differs from the set: extra {extra}, missing {lack}")

    return Result(rows, problems, gaps)


def render(res: Result) -> str:
    out: List[str] = []
    w = out.append
    w("# Ability Telemetry Coverage")
    w("")
    w("> Type: reference (generated). Audience: the ability-mechanics coordinator and packet workers.")
    w(f"> Generated by `{TOOL}`; do not edit by hand. Regenerate with "
      f"`python {TOOL}`, and check with `python {TOOL} --check` (CI runs it). "
      "Plan: [AB-C7](lab-uat-and-telemetry.md).")
    w("")
    w("Every ability-related entity method, and whether each side accounts for it where it travels. "
      "A client-to-server method needs a client send hook and a server receipt row; a server-to-client "
      "method needs a server send row and a client receive hook. `-` is a direction the method does not "
      "travel. An empty cell that travels is an exception listed in the script, with its reason.")
    w("")
    w("| Method | Index | Direction | Dispatch table | " + " | ".join(COLUMN_TITLES[c] for c in COLUMNS) + " | Notes |")
    w("|---|---:|---|---|" + "---|" * len(COLUMNS) + "---|")
    for m, found, cells in res.rows:
        tables = ", ".join(sorted({f"{Path(r.table).stem} {r.index}" for r in found})) or "none"
        vals = []
        for c in COLUMNS:
            cell = cells[c]
            vals.append({"n/a": "-", "filled": cell.text, "exception": "exception", "missing": "**MISSING**"}[cell.state])
        direction = "client to server" if m.direction == C2S else "server to client"
        w(f"| `{m.name}` | {m.index} | {direction} | {tables} | " + " | ".join(vals) + f" | {m.note} |")
    w("")
    w("## Exceptions")
    w("")
    if res.gaps:
        w("| Method | Column | Why the cell is empty |")
        w("|---|---|---|")
        for name, col, reason in res.gaps:
            w(f"| `{name}` | {COLUMN_TITLES[col]} | {reason} |")
    else:
        w("None.")
    w("")
    w("## Sources")
    w("")
    w("| Column | Code table |")
    w("|---|---|")
    for c in COLUMNS:
        path, const = SOURCES[c]
        w(f"| {COLUMN_TITLES[c]} | `{path}` `{const}` |")
    w("")
    w("## Ability-looking methods not in the set")
    w("")
    w("| Method | Why |")
    w("|---|---|")
    for n in sorted(NOT_IN_SET):
        w(f"| `{n}` | {NOT_IN_SET[n]} |")
    w("")
    return "\r\n".join(out)


def main(argv: Optional[Sequence[str]] = None) -> int:
    ap = argparse.ArgumentParser(description=(__doc__ or "").split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="exit 1 on drift or an unexplained gap")
    args = ap.parse_args(argv)
    try:
        res = build()
    except InputError as e:
        print(f"error: {e}", file=sys.stderr)
        return 2
    text = render(res)
    for p in res.problems:
        print(f"coverage: {p}", file=sys.stderr)
    if args.check:
        current = MATRIX.read_bytes().decode("utf-8") if MATRIX.exists() else ""
        drift = current.replace("\r\n", "\n") != text.replace("\r\n", "\n")
        if drift:
            print(f"coverage: {MATRIX.relative_to(ROOT)} is out of date; run `python {TOOL}`", file=sys.stderr)
        return 1 if drift or res.problems else 0
    MATRIX.write_bytes(text.encode("utf-8"))
    print(f"wrote {MATRIX.relative_to(ROOT)} ({len(res.rows)} methods, {len(res.gaps)} exceptions)")
    return 1 if res.problems else 0


if __name__ == "__main__":
    sys.exit(main())
