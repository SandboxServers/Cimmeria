"""Builds the report files as text (JSON data and Markdown reports) and writes them behind the privacy gate."""

import json
import re
from pathlib import Path

from report.scrub import Scrubber

from .assemble import assemble
from .compare import summary_doc
from .md_campaign import render_campaign
from .md_summary import render_summary

SCHEMA = "cimmeria-campaign-metrics/1"
SAFE_NAME = re.compile(r"[a-z0-9][a-z0-9._-]*")
RESERVED_STEMS = {"con", "prn", "aux", "nul", *(f"com{i}" for i in range(1, 10)), *(f"lpt{i}" for i in range(1, 10))}
TAKEN_NAMES = {"summary", "metrics"}


def _json(doc):
    return json.dumps(doc, indent=1, sort_keys=True) + "\n"


def _check_name(name):
    """Raise ValueError unless a campaign name is safe to use as a file name; the name is not echoed."""
    stem = name.split(".", 1)[0]
    if not SAFE_NAME.fullmatch(name) or ".." in name or name.endswith(".") or name in TAKEN_NAMES or stem in RESERVED_STEMS:
        raise ValueError("a campaign name is not a safe file name")


def build_outputs(inputs, self_cost=None):
    """Return {relative path: text} for every report file; nothing is written.

    `self_cost` is the own-session cost from `reporting.self_cost`, or None when the summary has no own-cost section.
    A cost with no sessions counts as no own-cost section.
    """
    assembled = assemble(inputs)
    records = assembled["campaigns"]
    meta = assembled["meta"]
    for name in records:
        _check_name(name)
    summary = summary_doc(records)
    outputs = {}
    for name, record in records.items():
        outputs[f"data/{name}.json"] = _json(record)
        outputs[f"reports/{name}.md"] = render_campaign(record)
    outputs["data/summary.json"] = _json({**summary, "meta": meta})
    outputs["data/metrics.json"] = _json({"schema": SCHEMA, "stamp": meta["stamp"], "campaigns": records, "summary": summary})
    self_cost = self_cost if self_cost and self_cost.get("sessions") else None
    outputs["reports/summary.md"] = render_summary(records, summary, meta, self_cost=self_cost)
    return outputs


def write_outputs(outputs, out_dir, scrubber=None):
    """Write `outputs` under `out_dir` and return the sorted relative paths written.

    Every text passes the privacy gate before the first file is created: a `PrivacyError` leaves `out_dir` untouched.
    Markdown is written with CRLF line ends, JSON with LF; all files are UTF-8 without a BOM.
    """
    scrubber = scrubber or Scrubber()
    out_dir = Path(out_dir)
    for path, text in outputs.items():
        scrubber.assert_clean(text, path)
        if not (out_dir / path).resolve().is_relative_to(out_dir.resolve()):
            raise ValueError("a report path is outside the output folder")
    for path, text in outputs.items():
        target = out_dir / path
        target.parent.mkdir(parents=True, exist_ok=True)
        if path.endswith(".md"):
            text = text.replace("\r\n", "\n").replace("\n", "\r\n")
        target.write_bytes(text.encode("utf-8"))
    return sorted(outputs)
