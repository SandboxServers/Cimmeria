"""Builds the report files as text (JSON data and Markdown reports) and writes them behind the privacy gate."""

import json

from .assemble import assemble
from .compare import summary_doc
from .md_campaign import render_campaign
from .md_summary import render_summary

SCHEMA = "cimmeria-campaign-metrics/1"


def _json(doc):
    return json.dumps(doc, indent=1, sort_keys=True) + "\n"


def build_outputs(inputs, self_cost=None):
    """Return {relative path: text} for every report file; nothing is written.

    `self_cost` is the own-session cost from `reporting.self_cost`, or None when the summary has no own-cost section.
    """
    assembled = assemble(inputs)
    records = assembled["campaigns"]
    meta = assembled["meta"]
    summary = summary_doc(records)
    outputs = {}
    for name, record in records.items():
        outputs[f"data/{name}.json"] = _json(record)
        outputs[f"reports/{name}.md"] = render_campaign(record)
    outputs["data/summary.json"] = _json({**summary, "meta": meta})
    outputs["data/metrics.json"] = _json({"schema": SCHEMA, "stamp": meta["stamp"], "campaigns": records, "summary": summary})
    outputs["reports/summary.md"] = render_summary(records, summary, meta, self_cost=self_cost)
    return outputs
