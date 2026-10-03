"""Compare a database's pr_attribution with ground truth, weighted by estimated USD and by request count.

For each labelled request the attribution's weight is split four ways:
`correct` (on the true PR), `wrong` (on another PR), `campaign` (charged to
a campaign, not a PR) and `unattributed`. Precision is correct over correct
plus wrong; recall is correct over everything labelled.
"""

PARTS = ("correct", "wrong", "campaign", "unattributed")


def _usd(db):
    return dict(db.execute("SELECT request_id, COALESCE(usd, 0) FROM rcost"))


def score(db, truth, usd=None):
    """The scores of db's attribution against truth ({request_id: pr})."""
    usd = usd if usd is not None else _usd(db)
    rows = {}
    for rid, pr, method, w in db.execute("SELECT request_id, pr_number, method, weight FROM pr_attribution"):
        if rid in truth:
            rows.setdefault(rid, []).append((pr, method, w))
    acc = {"usd": {p: 0.0 for p in PARTS}, "requests": {p: 0.0 for p in PARTS}}
    wrong_by_method, missing = {}, 0
    for rid, pr_true in truth.items():
        got = rows.get(rid)
        if not got:
            missing += 1
            continue
        u = usd.get(rid, 0.0)
        for pr, method, w in got:
            part = ("unattributed" if method == "unattributed" else "campaign" if method == "campaign"
                    else "correct" if pr == pr_true else "wrong")
            acc["usd"][part] += w * u
            acc["requests"][part] += w
            if part == "wrong":
                wrong_by_method[method] = wrong_by_method.get(method, 0.0) + w * u
    out = {"labelled_requests": len(truth) - missing, "requests_missing": missing}
    for unit in ("usd", "requests"):
        a = acc[unit]
        total = sum(a.values())
        placed = a["correct"] + a["wrong"]
        out[unit] = {"total": total, **{f"{p}_share": a[p] / total if total else 0.0 for p in PARTS},
                     "precision": a["correct"] / placed if placed else 0.0,
                     "recall": a["correct"] / total if total else 0.0}
    total_usd = out["usd"]["total"]
    out["wrong_usd_share_by_method"] = {m: v / total_usd if total_usd else 0.0
                                        for m, v in sorted(wrong_by_method.items(), key=lambda kv: -kv[1])}
    return out


def overall(db, usd=None):
    """Attribution coverage over every request: the USD share each method places."""
    usd = usd if usd is not None else _usd(db)
    by = {}
    for rid, method, w in db.execute("SELECT request_id, method, weight FROM pr_attribution"):
        by[method] = by.get(method, 0.0) + w * usd.get(rid, 0.0)
    total = sum(by.values())
    return {"usd": total, "method_share": {m: v / total if total else 0.0
                                           for m, v in sorted(by.items(), key=lambda kv: -kv[1])}}
