#!/usr/bin/env python3
"""Turn designer effect text into effect NVP rows (ability-mechanics D-AB06).

The 2009 effect rows state their numbers only in ``effect_desc`` ("+10%
Health", "Heals 35% of target's Focus pool", "-200F / -20H"). This tool
parses that text for the effects of player-reachable abilities and writes:

* one generated block per family into
  ``db/resources/Effects/Seed/effect_nvps.sql``, between
  ``-- ability-mechanics generated <family> begin`` / ``end`` markers, with
  ``nvp_id`` in the family's reserved range (``families.NVP_RANGES``);
* the family's ``script_name`` on each generated effect's row in
  ``db/resources/Effects/Seed/effects.sql``. That file loads after
  ``effect_nvps.sql`` in ``db/database.sql``, so an ``UPDATE`` inside the
  block would match no row.

Every generated row is RECONSTRUCTION: its value is read from the effect's
own text, never recovered from 2009 server data.

Ownership rules, shared by every family:

* An effect with a hand-authored NVP (a row outside every generated block)
  of one of the family's NVP names is left alone and reported.
* An effect whose ``script_name`` is set, and was not set by this family's
  committed block, is left alone and reported. A script the family set
  earlier and no longer generates is cleared back to NULL.
* Rows outside the markers are never rewritten.

Usage (from the repo root, stock Python 3):

    python tools/ability_mechanics/effect_nvps_from_desc.py            # regenerate
    python tools/ability_mechanics/effect_nvps_from_desc.py --check    # exit 1 on drift
    python tools/ability_mechanics/effect_nvps_from_desc.py --report   # what parsed, what did not

Exit codes: 0 ok, 1 drift (``--check``), 2 input or validation failure.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Set, Tuple

sys.path.insert(0, str(Path(__file__).resolve().parent))

from corpus import BEGIN, END, NVPS_SQL, EFFECTS_SQL, Corpus, Effect, find_blocks, load_corpus  # noqa: E402
from families import FAMILIES, NVP_RANGES  # noqa: E402
from family import Family, Generated, Rejected, show  # noqa: E402
from seed_sql import InputError, sql_quote, sql_rows, write_text  # noqa: E402

TOOL = "tools/ability_mechanics/effect_nvps_from_desc.py"


@dataclass
class FamilyResult:
    family: Family
    generated: List[Generated]
    rejected: List[Rejected]
    hand_authored: List[Tuple[Effect, str]]  # (effect, how the parser compares)


def run_family(family: Family, corpus: Corpus, claimed: Dict[int, str]) -> FamilyResult:
    """Classify every reachable candidate of one family.

    ``claimed`` maps effect id to the family that binds its script in this
    run; a second family cannot bind the same effect.
    """
    owned_before = corpus.owned_before(family.name)
    result = FamilyResult(family, [], [], [])
    for e in corpus.reachable_effects():
        if not family.is_candidate(e, corpus):
            continue
        outcome = family.parse(e, corpus)
        hand_rows = corpus.hand_rows(e.effect_id, family.nvp_names)
        if hand_rows:
            if isinstance(outcome, Generated):
                same = sorted(outcome.nvps) == sorted(hand_rows)
                note = "parser agrees" if same else f"parser would write {outcome.nvps}, hand row has {hand_rows}"
            else:
                note = f"parser rejects: {outcome.reason}"
            result.hand_authored.append((e, note))
            continue
        if isinstance(outcome, Rejected):
            result.rejected.append(outcome)
            continue
        if outcome.script is not None:
            ours = e.effect_id in owned_before and e.script_name in family.scripts
            if e.script_name is not None and not ours:
                result.rejected.append(Rejected(e, f"script_name is already '{e.script_name}' (hand-authored)"))
                continue
            if e.effect_id in claimed:
                result.rejected.append(Rejected(e, f"script already bound by the {claimed[e.effect_id]} family"))
                continue
            claimed[e.effect_id] = family.name
        result.generated.append(outcome)
    return result


def nvp_ids(text: str) -> List[int]:
    return [int(r["nvp_id"].text) for r in sql_rows(text, "effect_nvps")]


def used_outside(nvps_text: str, family: str) -> Set[int]:
    """Every ``nvp_id`` in the file outside the family's own block: hand rows
    (including generated rows someone moved out of the markers) and other
    families' blocks. The family's allocation skips all of them."""
    b = find_blocks(nvps_text).get(family)
    outside = nvps_text if b is None else nvps_text[: b.start] + nvps_text[b.end :]
    return set(nvp_ids(outside))


def check_unique_ids(nvps_text: str) -> None:
    """Backstop before anything is written or compared: a duplicate
    ``nvp_id`` would fail the seed load on the primary key."""
    seen: Set[int] = set()
    dups = sorted({i for i in nvp_ids(nvps_text) if i in seen or seen.add(i)})
    if dups:
        raise InputError(f"{NVPS_SQL}: duplicate nvp_id {dups}")


def render_block(result: FamilyResult, used: Set[int]) -> str:
    fam = result.family
    lo, hi = NVP_RANGES[fam.name]
    lines = [
        BEGIN.format(family=fam.name),
        f"-- GENERATED by {TOOL} (family '{fam.name}', nvp_id {lo}-{hi}).",
        "-- Do not edit by hand: change the parser and regenerate. Rows outside these",
        "-- markers are hand-authored and the tool never touches them.",
        "-- RECONSTRUCTION: the 2009 rows shipped no NVP for these effects. Each value",
        "-- is read from the effect's own effect_desc, quoted below (\\n = line break);",
        "-- it is not recovered server data. The same run sets each effect's",
        "-- script_name in effects.sql.",
    ]
    nvp_id = lo
    for g in result.generated:
        e = g.effect
        lines.append(f"-- RECONSTRUCTION {e.effect_id} {e.name} (ability {e.ability_id}), effect_desc {show(e.desc)}")
        bind = f" -> {g.script}" if g.script else ""
        rows = ", ".join(f"{n} {v}" for n, v in g.nvps)
        lines.append(f'--   from "{g.source}"{bind}, {rows}')
        for note in g.notes:
            lines.append(f"--   note: {note}")
        for n, v in g.nvps:
            while nvp_id in used:
                nvp_id += 1
            if nvp_id > hi:
                raise InputError(f"family '{fam.name}' ran out of nvp_ids ({lo}-{hi})")
            lines.append(
                "INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES "
                f"({nvp_id}, {e.effect_id}, {sql_quote(n)}, {sql_quote(v)});"
            )
            nvp_id += 1
    lines.append(END.format(family=fam.name))
    return "\n".join(lines) + "\n"


def place_block(nvps_text: str, family: str, block: str) -> str:
    """Replace the family's block, or add it before the pg_dump trailer (the
    sequence ``setval``), after every row."""
    existing = find_blocks(nvps_text).get(family)
    if existing:
        return nvps_text[: existing.start] + block + nvps_text[existing.end :]
    trailers = list(re.finditer(r"^--\n-- TOC entry", nvps_text, re.M))
    if not trailers:
        raise InputError(f"{NVPS_SQL}: no trailer to add the '{family}' block before")
    at = trailers[-1].start()
    return nvps_text[:at] + block + "\n" + nvps_text[at:]


def script_edits(corpus: Corpus, result: FamilyResult) -> Dict[int, Optional[str]]:
    """``script_name`` per effect after this family: what it binds, and NULL
    for what it bound before and no longer generates.

    An effect with a hand-authored row of the family's NVPs is handed over,
    not dropped: its binding stays, because the hand row needs the same
    script to do anything. Ownership comes from ``corpus.hand_rows`` alone,
    so it holds even when the effect stopped being a candidate (its text
    changed) or reachable."""
    fam = result.family
    want: Dict[int, Optional[str]] = {}
    for eid in corpus.owned_before(fam.name):
        if corpus.hand_rows(eid, fam.nvp_names):
            continue
        e = corpus.effects.get(eid)
        if e is not None and e.script_name in fam.scripts:
            want[eid] = None
    for g in result.generated:
        if g.script is not None:
            want[g.effect.effect_id] = g.script
    return want


def apply_scripts(corpus: Corpus, want: Dict[int, Optional[str]]) -> str:
    text = corpus.effects_text
    for eid in sorted(want, key=lambda i: corpus.effects[i].script_span[0], reverse=True):
        e = corpus.effects[eid]
        if e.script_name != want[eid]:
            s, t = e.script_span
            text = text[:s] + sql_quote(want[eid]) + text[t:]
    return text


def generate(
    families: Sequence[str], corpus: Optional[Corpus] = None
) -> Tuple[Corpus, List[FamilyResult], str, str]:
    """Run the families; return the corpus, the results, and the new
    ``effect_nvps.sql`` and ``effects.sql`` texts (LF line ends)."""
    corpus = corpus or load_corpus()
    claimed: Dict[int, str] = {}
    results: List[FamilyResult] = []
    nvps_text = corpus.nvps_text
    clears: Dict[int, Optional[str]] = {}
    binds: Dict[int, Optional[str]] = {}
    # Once per family: a repeated `--family heal` would otherwise see its own
    # claims from the first pass and write an empty block.
    for name in dict.fromkeys(families):
        result = run_family(FAMILIES[name], corpus, claimed)
        results.append(result)
        block = render_block(result, used_outside(nvps_text, name))
        nvps_text = place_block(nvps_text, name, block)
        for eid, script in script_edits(corpus, result).items():
            (binds if script is not None else clears)[eid] = script
    check_unique_ids(nvps_text)
    # A bind from any family beats another family's clear of the same effect.
    return corpus, results, nvps_text, apply_scripts(corpus, {**clears, **binds})


def print_report(results: List[FamilyResult], out=sys.stdout) -> None:
    for r in results:
        print(f"## family {r.family.name}", file=out)
        print(f"generated: {len(r.generated)} effects", file=out)
        for g in r.generated:
            rows = ", ".join(f"{n} {v}" for n, v in g.nvps)
            print(f"  {g.effect.effect_id:>5} {g.script or '-':<11} {rows:<22} {show(g.effect.desc)}", file=out)
            for note in g.notes:
                print(f"        note: {note}", file=out)
        print(f"hand-authored, left alone: {len(r.hand_authored)}", file=out)
        for e, note in r.hand_authored:
            print(f"  {e.effect_id:>5} {show(e.desc)} ({note})", file=out)
        print(f"unparsed: {len(r.rejected)}", file=out)
        for x in r.rejected:
            e = x.effect
            print(f"  {e.effect_id:>5} (ability {e.ability_id}) {show(e.desc)}: {x.reason}", file=out)
        print(file=out)


def main(argv: Optional[Sequence[str]] = None) -> int:
    ap = argparse.ArgumentParser(description=(__doc__ or "").split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="exit 1 if the committed seed differs from the output")
    ap.add_argument("--report", action="store_true", help="print generated, hand-authored and unparsed effects")
    ap.add_argument(
        "--family",
        action="append",
        choices=sorted(FAMILIES),
        help="limit to one family (repeatable); default every family",
    )
    args = ap.parse_args(argv)
    try:
        corpus, results, nvps_text, effects_text = generate(args.family or list(FAMILIES))
    except InputError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    if args.report:
        print_report(results)
    drift = [p for p, new, old in ((NVPS_SQL, nvps_text, corpus.nvps_text), (EFFECTS_SQL, effects_text, corpus.effects_text)) if new != old]
    if args.check:
        for p in drift:
            print(f"drift: {p.as_posix()} differs from what {TOOL} would write", file=sys.stderr)
        if drift:
            print(f"fix: python {TOOL}", file=sys.stderr)
            return 1
        print(f"ok: {sum(len(r.generated) for r in results)} generated effects match the seed")
        return 0
    if NVPS_SQL in drift:
        write_text(NVPS_SQL, nvps_text, corpus.nvps_eol)
    if EFFECTS_SQL in drift:
        write_text(EFFECTS_SQL, effects_text, corpus.effects_eol)
    for p in drift:
        print(f"wrote {p.as_posix()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
