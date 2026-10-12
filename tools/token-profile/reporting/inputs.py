"""Loads the five campaign metrics input files (cost, ledger, github, packets, overstep) from one folder."""

import json
from pathlib import Path

FILES = ("cost", "ledger", "github", "packets", "overstep")


def load_inputs(folder):
    """Return the parsed input files keyed by name, plus `missing`: the names of files not found (those load as {})."""
    folder = Path(folder)
    inputs = {}
    missing = []
    for name in FILES:
        path = folder / f"{name}.json"
        if not path.exists():
            missing.append(name)
            inputs[name] = {}
            continue
        try:
            inputs[name] = json.loads(path.read_text(encoding="utf-8-sig"))
        except json.JSONDecodeError as exc:
            raise ValueError(f"{name}.json is not valid JSON") from exc
    inputs["missing"] = missing
    return inputs
