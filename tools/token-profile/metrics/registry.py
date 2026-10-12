"""The campaign registry and regime config (CM-01): which campaigns exist, and which regime a time falls in.

Tagging a PR reuses ingest/campaigns.py; nothing here re-reads the branch rules.
"""

import json
from datetime import datetime, timezone
from pathlib import Path

from ingest.campaigns import CONFIG as CAMPAIGNS
from ingest.campaigns import Campaigns

HERE = Path(__file__).resolve().parent
CONFIG = HERE / "config.json"


def load_config(path=None):
    """The metrics config.json: window, regime switch, role map, accepted statuses. No prices."""
    return json.loads(Path(path or CONFIG).read_text(encoding="utf-8"))


def load_campaigns(path=None):
    """The ingest's campaigns.json as it is, `_doc` included."""
    return json.loads(Path(path or CAMPAIGNS).read_text(encoding="utf-8"))


def regime_of(ts_iso, config):
    """"haiku" from the regime switch instant on, "old" before it. A timestamp without a zone is taken as UTC."""
    ts = datetime.fromisoformat(ts_iso)
    if ts.tzinfo is None:
        ts = ts.replace(tzinfo=timezone.utc)
    return "haiku" if ts >= datetime.fromisoformat(config["regime_switch"]) else "old"


def campaign_of_pr(pr, campaigns):
    """The campaign of a PR row: its head branch, then a tracking issue in its title, then its base branch.
    `campaigns` is a Campaigns or the campaigns.json dict."""
    rules = campaigns if isinstance(campaigns, Campaigns) else Campaigns(campaigns)
    return (rules.of_branch(pr.get("head_branch")) or rules.of_title(pr.get("title"))
            or rules.of_branch(pr.get("base_branch")))


def problems(campaigns, repo):
    """What is wrong with the campaign rules: a campaign with no docs/analysis/<name>/ folder, a docs/analysis/
    folder with neither a campaign nor an `_untagged` entry, or two prefixes where one contains the other (the
    first match would then depend on the order). Names only, no prefixes."""
    rules = {name: rule for name, rule in campaigns.items() if not name.startswith("_")}
    analysis = Path(repo) / "docs" / "analysis"
    found = [f"no folder for {name}" for name in sorted(rules) if not (analysis / name).is_dir()]
    if analysis.is_dir():
        known = set(rules) | set(campaigns.get("_untagged", {}))
        found += [f"no campaign entry for {folder.name}" for folder in sorted(analysis.iterdir())
                  if folder.is_dir() and folder.name not in known]
    owned = [(prefix, name) for name, rule in rules.items() for prefix in rule.get("prefixes", ())]
    for i, (prefix, name) in enumerate(owned):
        for other, rival in owned[i + 1:]:
            if rival != name and (prefix.startswith(other) or other.startswith(prefix)):
                found.append(f"prefix of {name} overlaps a prefix of {rival}")
    return found
