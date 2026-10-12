"""The whole campaign Markdown report: one campaign record in, one Markdown document out."""

from . import md_campaign_quality as quality
from .md_campaign_cost import accepted_md, cost_md, first_pass_md, prs_md, retries_md, stamp_md
from .md_common import section_md

SECTIONS = (
    ("Stamp", stamp_md),
    ("Cost", cost_md),
    ("PRs", prs_md),
    ("Accepted", accepted_md),
    ("First pass", first_pass_md),
    ("Retries", retries_md),
    ("CI", quality.ci_md),
    ("Escaped defects", quality.defects_md),
    ("Escalation", quality.escalation_md),
    ("Stop context", quality.stop_context_md),
    ("Cycle time", quality.cycle_md),
    ("Overstep", quality.overstep_md),
)


def render_campaign(record):
    """Render one campaign: its name, its regime, then the twelve sections in report order."""
    body = "\n".join(section_md(title, render(record).rstrip("\n")) for title, render in SECTIONS)
    return f"# {record['campaign']}\n\n> Regime: {record['regime']}\n\n" + body.rstrip("\n") + "\n"
