"""Versioned per-model list prices, in USD per million tokens.

A new price list is a new version: add a dict under a new key, never edit an
old one, so a report made under an old version can be recomputed exactly.
Cache writes are 1.25x input for the 5-minute TTL and 2x for the 1-hour TTL
on every model. Cache reads differ by model: 0.05x on Opus 5.5, 0.025x on
Fable 5.1, 0.1x elsewhere. There is no long-context premium on any of them.
"""

import re

SOURCE = "https://platform.claude.com/docs/en/about-claude/pricing"


def _row(inp, out, read):
    return {"input": inp, "output": out, "cache_read": read,
            "cache_write_5m": inp * 1.25, "cache_write_1h": inp * 2.0}


PRICE_TABLES = {
    "2026-10-03": {
        "claude-fable-5-1": _row(10.0, 50.0, 0.25),
        "claude-fable-5": _row(10.0, 50.0, 1.0),
        "claude-opus-5-5": _row(4.0, 20.0, 0.20),
        "claude-opus-5": _row(5.0, 25.0, 0.50),
        "claude-opus-4-8": _row(5.0, 25.0, 0.50),
        "claude-opus-4-7": _row(5.0, 25.0, 0.50),
        "claude-opus-4-6": _row(5.0, 25.0, 0.50),
        "claude-sonnet-5-5": _row(2.0, 10.0, 0.20),
        "claude-sonnet-5": _row(2.0, 10.0, 0.20),
        "claude-sonnet-4-6": _row(3.0, 15.0, 0.30),
        "claude-haiku-4-5": _row(1.0, 5.0, 0.10),
    },
}
CURRENT = "2026-10-03"

TOKEN_COLUMNS = ("input", "output", "cache_read", "cache_write_5m", "cache_write_1h")


def normalize_model(model):
    """'claude-haiku-4-5-20251001' and 'claude-opus-5[1m]' to their table keys."""
    model = re.sub(r"\[[^\]]*\]$", "", model or "")
    return re.sub(r"-\d{8}$", "", model)


def price_for(model, version=CURRENT):
    return PRICE_TABLES[version].get(normalize_model(model))


def estimate_usd(model, tokens, version=CURRENT):
    """List-price estimate for one request; None when the model has no price.

    tokens maps the TOKEN_COLUMNS names to counts. Thinking tokens are part of
    output and are not passed separately.
    """
    row = price_for(model, version)
    if row is None:
        return None
    return sum(tokens.get(k, 0) * row[k] for k in TOKEN_COLUMNS) / 1e6


def store(db, version=CURRENT):
    db.executemany(
        "INSERT OR IGNORE INTO price_tables (version, model, input, output, cache_read, cache_write_5m,"
        " cache_write_1h, source) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        [(version, model, *(row[k] for k in TOKEN_COLUMNS), SOURCE)
         for model, row in PRICE_TABLES[version].items()])
