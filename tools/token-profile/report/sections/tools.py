"""Tool results and context exposure, by tool and by command or path fingerprint.

Exposure is result_chars x later requests: how often a result was re-read
before compaction or the end of the transcript. It ranks offenders; it is
never converted to tokens or dollars.
"""

from ..stats import distribution, share

NOTE = ("Context exposure = result characters x requests that re-read them. "
        "A ranking of what fills context, not a cost.")


def build(db, sc, top=25):
    rows = db.execute("SELECT tool_name, mcp_server, fingerprint, result_chars, result_is_error,"
                      " result_persisted, exposure_chars FROM wtool").fetchall()
    total_chars = sum(r["result_chars"] or 0 for r in rows)
    total_exposure = sum(r["exposure_chars"] or 0 for r in rows)

    by_tool, by_fp, by_server = {}, {}, {}
    for r in rows:
        tool = sc.label(r["tool_name"], "tool_name")
        fp = sc.fingerprint(r["tool_name"], r["fingerprint"])
        for groups, key in ((by_tool, tool), (by_fp, (tool, fp)),
                            (by_server, sc.label(r["mcp_server"], "mcp_server") if r["mcp_server"] else None)):
            if key is None:
                continue
            g = groups.setdefault(key, {"calls": 0, "errors": 0, "persisted": 0, "chars": [], "exposure": 0})
            g["calls"] += 1
            g["errors"] += r["result_is_error"] or 0
            g["persisted"] += r["result_persisted"] or 0
            g["chars"].append(r["result_chars"])
            g["exposure"] += r["exposure_chars"] or 0

    def rows_of(groups, key_fields, limit=None):
        out = []
        for key, g in sorted(groups.items(), key=lambda kv: (-kv[1]["exposure"], -sum(c or 0 for c in kv[1]["chars"]))):
            dist = distribution(g["chars"])
            out.append({**dict(zip(key_fields, key if isinstance(key, tuple) else (key,))),
                        "calls": g["calls"], "errors": g["errors"], "persisted": g["persisted"],
                        "result_chars": dist, "chars_share": share(dist["sum"], total_chars),
                        "exposure_chars": g["exposure"], "exposure_share": share(g["exposure"], total_exposure)})
        return out[:limit] if limit else out

    return {
        "layer": "tool results and exposure",
        "note": NOTE,
        "totals": {"calls": len(rows), "result_chars": total_chars, "exposure_chars": total_exposure,
                   "results_pending": sum(1 for r in rows if r["result_chars"] is None)},
        "by_tool": rows_of(by_tool, ("tool",)),
        "by_fingerprint": rows_of(by_fp, ("tool", "fingerprint"), top),
        "by_mcp_server": rows_of(by_server, ("server",)),
        "per_call_result_chars": distribution([r["result_chars"] for r in rows]),
    }
