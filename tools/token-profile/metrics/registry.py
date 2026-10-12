"""The campaign registry and regime config (CM-01): which campaigns exist, and which regime a time falls in.

Tagging a PR reuses ingest/campaigns.py; nothing here re-reads the branch rules.
"""

import json
from datetime import datetime
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
    """"haiku" from the regime switch instant on, "old" before it."""
    return "haiku" if datetime.fromisoformat(ts_iso) >= datetime.fromisoformat(config["regime_switch"]) else "old"


def campaign_of_pr(pr, campaigns):
    """The campaign of a PR row: its head branch, then a tracking issue in its title, then its base branch."""
    rules = Campaigns(campaigns)
    return (rules.of_branch(pr.get("head_branch")) or rules.of_title(pr.get("title"))
            or rules.of_branch(pr.get("base_branch")))


def problems(campaigns, repo):
    """What is wrong with the campaign rules: a campaign with no docs/analysis/<name>/ folder, or two prefixes
    where one contains the other (the first match would then depend on the order). Names only, no prefixes."""
    rules = {name: rule for name, rule in campaigns.items() if not name.startswith("_")}
    found = [f"no folder for {name}" for name in sorted(rules)
             if not (Path(repo) / "docs" / "analysis" / name).is_dir()]
    owned = [(prefix, name) for name, rule in rules.items() for prefix in rule.get("prefixes", ())]
    for i, (prefix, name) in enumerate(owned):
        for other, rival in owned[i + 1:]:
            if rival != name and (prefix.startswith(other) or other.startswith(prefix)):
                found.append(f"prefix of {name} overlaps a prefix of {rival}")
    return found
