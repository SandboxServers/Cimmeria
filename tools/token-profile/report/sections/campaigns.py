"""Cost per campaign: the sum over its PRs, plus packet work that had no PR of its own (D-TP7).

A campaign's PRs are those prs.campaign names (campaigns.json, attribution.md
§ Campaigns). Each PR carries only its own spend, so an integration PR is not
charged for the packets merged into it; those packets' requests are
'campaign' rows and are shown on their own line. Like the per-PR section, a
campaign's numbers are its whole attributed history; the window picks which
campaigns had any spend in it.
"""

from .cost import LABEL

NOTE = ("A campaign's cost is the sum of its PRs' spend plus the packet work that reached it through an"
        " integration branch without a PR of its own. An integration PR carries only its own spend (D-TP7).")


def build(db, sc, top=20):
    version = db.execute("SELECT value FROM meta WHERE key = 'schema_version'").fetchone()[0]
    if int(version) < 4:
        return {"layer": "cost per campaign", "label": LABEL, "note": NOTE, "available": False, "campaigns": []}
    active = {c for (c,) in db.execute(
        "SELECT DISTINCT COALESCE(a.campaign, p.campaign) FROM wreq w JOIN pr_attribution a USING (request_id)"
        " LEFT JOIN prs p ON p.pr_number = a.pr_number WHERE COALESCE(a.campaign, p.campaign) IS NOT NULL")}
    prs = {}
    for campaign, n, state, merged, usd in db.execute(
            "SELECT p.campaign, p.pr_number, p.state, p.merged_at, SUM(a.weight * COALESCE(c.usd, 0))"
            " FROM prs p JOIN pr_attribution a ON a.pr_number = p.pr_number JOIN rcost c USING (request_id)"
            " WHERE p.campaign IS NOT NULL GROUP BY p.pr_number"):
        if campaign in active:
            prs.setdefault(campaign, []).append({"pr": n, "state": sc.label(state, "pr_state"), "usd_est": usd or 0.0})
    packets = dict(db.execute(
        "SELECT a.campaign, SUM(a.weight * COALESCE(c.usd, 0)) FROM pr_attribution a JOIN rcost c USING (request_id)"
        " WHERE a.method = 'campaign' GROUP BY a.campaign"))
    out = []
    for campaign in active:
        rows = sorted(prs.get(campaign, []), key=lambda r: -r["usd_est"])
        pr_usd = sum(r["usd_est"] for r in rows)
        no_pr = packets.get(campaign) or 0.0
        out.append({"campaign": sc.label(campaign, "campaign"), "prs": len(rows),
                    "merged_prs": sum(1 for r in rows if r["state"] == "MERGED"),
                    "pr_usd_est": round(pr_usd, 4), "no_pr_usd_est": round(no_pr, 4),
                    "usd_est": round(pr_usd + no_pr, 4),
                    "top_prs": [{"pr": r["pr"], "usd_est": round(r["usd_est"], 4)} for r in rows[:10]]})
    out.sort(key=lambda r: -r["usd_est"])
    return {"layer": "cost per campaign", "label": LABEL, "note": NOTE, "available": True,
            "campaigns": out[:top], "campaigns_total": len(out)}
