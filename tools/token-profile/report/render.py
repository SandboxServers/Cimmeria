"""Render a report dict as Markdown. The JSON output is the dict itself."""

from .stats import PERCENTILES

DIST_HEAD = ["n"] + [f"p{p}" for p in PERCENTILES] + ["max", "mean"]


def num(x):
    if x is None:
        return "-"
    if isinstance(x, float) and abs(x) < 10 and x != int(x):
        return f"{x:,.2f}"
    return f"{round(x):,}"


def usd(x):
    return "-" if x is None else f"${x:,.2f}"


def pct(x):
    return "-" if x is None else f"{x * 100:.1f}%"


def table(head, rows, numeric=True):
    align = "---:" if numeric else "---"
    out = ["| " + " | ".join(head) + " |", "|" + "|".join("---" if i == 0 else align for i in range(len(head))) + "|"]
    for row in rows:
        out.append("| " + " | ".join("-" if c is None else str(c) for c in row) + " |")
    return "\n".join(out)


def dist_row(label, d, fmt=num):
    return [label, num(d["n"])] + [fmt(d[f"p{p}"]) for p in PERCENTILES] + [fmt(d["max"]), fmt(d["mean"])]


def dist_table(rows, fmt=num, first="series"):
    return table([first] + DIST_HEAD, [dist_row(label, d, fmt) for label, d in rows])


def stamp_block(s):
    w = s["window"]
    rows = [
        ("Window", f"{w['since'] or 'start'} to {w['until'] or 'now'}"),
        ("Requests in window", f"{num(w['requests'])}, {w['first_request'] or '-'} to {w['last_request'] or '-'}"),
        ("Profiler commit", s["profiler_commit"] or "-"),
        ("Report commit", s["report_commit"] or "-"),
        ("Price table", s["price_table"] or "-"),
        ("Claude Code", ", ".join(s["claude_code"]["all"]) or "-"),
        ("Models", ", ".join(s["models"]) or "-"),
        ("Schema version", s["schema_version"]),
        ("Ingest runs", f"{s['ingest_runs']}, last finished {s['last_ingest'] or '-'}"),
        ("Unknown records", f"{num(s['unknown_records'])} records; {s['unknown_shapes']['shapes']} unknown shapes"),
        ("Values rejected by the privacy filter", ", ".join(f"{k} {v}" for k, v in s["rejected"].items()) or "none"),
    ]
    return table(["Stamp", "Value"], rows, numeric=False)


def tokens_md(t):
    cols = ["input_tokens", "output_tokens", "thinking_tokens", "cache_read", "cache_write_5m", "cache_write_1h",
            "context_tokens"]
    head = ["requests", "input", "output", "thinking", "cache read", "cache write 5m", "cache write 1h", "context"]
    parts = [f"## Raw tokens\n\n{t['note']}\n",
             table(["", *head], [["all", num(t["totals"]["requests"])] + [num(t["totals"][c]) for c in cols]])]
    for dim, rows in t["by"].items():
        parts.append(f"\nBy {dim}:\n")
        parts.append(table([dim, *head], [[r["key"], num(r["requests"])] + [num(r[c]) for c in cols] for r in rows]))
    parts.append("\nPer request:\n")
    parts.append(dist_table(list(t["per_request"].items()), first="category"))
    return "\n".join(parts)


def cost_md(c):
    t = c["totals"]
    parts = [f"## Estimated USD\n\n**{c['label']}**\n",
             table(["", "total", "input", "output", "cache read", "cache write 5m", "cache write 1h"],
                   [["all", usd(t["usd"]), usd(t["usd_input"]), usd(t["usd_output"]), usd(t["usd_cache_read"]),
                     usd(t["usd_cache_write_5m"]), usd(t["usd_cache_write_1h"])]]),
             f"\nPriced requests: {num(t['priced_requests'])}. Unpriced: {num(t['unpriced_requests'])}"
             + (f" (models with no price: {', '.join(c['unpriced_models'])})" if c["unpriced_models"] else "")
             + f". Not priced at all: {num(c['not_priced']['web_search_requests'])} web searches,"
               f" {num(c['not_priced']['web_fetch_requests'])} web fetches."]
    for dim, rows in c["by"].items():
        parts.append(f"\nBy {dim}:\n")
        parts.append(table([dim, "requests", "est. USD", "share"],
                           [[r["key"], num(r["requests"]), usd(r["usd"]), pct(r["share"])] for r in rows]))
    parts.append("\nDistributions:\n")
    parts.append(dist_table([("per request", c["per_request"]), ("per transcript", c["per_transcript"])], usd))
    return "\n".join(parts)


def context_md(c):
    parts = [f"## Context pressure\n\n{c['note']}\n", "Context tokens per request:\n",
             dist_table(list(c["per_request"].items()), first="scope"),
             "\nPeak context per transcript:\n", dist_table(list(c["peak_per_transcript"].items()), first="scope"),
             "\nRequests per transcript:\n", dist_table(list(c["requests_per_transcript"].items()), first="scope"),
             "\nFirst request of each transcript (static context), by agent type:\n",
             dist_table(list(c["first_request_by_agent_type"].items()), first="agent type")]
    for scope, rows in c["cache_writes_by_idle_gap"].items():
        parts.append(f"\nCache writes by idle gap before the request, {scope}:\n")
        parts.append(table(["gap", "requests", "cache-write tokens", "share"],
                           [[r["gap"], num(r["requests"]), num(r["cache_write"]), pct(r["write_share"])]
                            for r in rows]))
    k = c["compactions"]
    parts.append(f"\n### Compactions\n\n{num(k['count'])} compactions over {num(k['transcripts_in_window'])}"
                 f" transcripts. By trigger: {', '.join(f'{a} {b}' for a, b in k['by_trigger'].items()) or 'none'}."
                 f" By scope: main {k['by_scope']['main']}, subagent {k['by_scope']['subagent']}.\n")
    parts.append(dist_table([(f, k[f]) for f in ("pre_tokens", "post_tokens", "dropped_tokens", "duration_ms")],
                            first="measure"))
    return "\n".join(parts)


def tools_md(t):
    tot = t["totals"]
    parts = [f"## Tool results and context exposure\n\n{t['note']}\n",
             f"{num(tot['calls'])} calls, {num(tot['result_chars'])} result characters,"
             f" {num(tot['exposure_chars'])} exposure characters, {num(tot['results_pending'])} results not seen.\n"]

    def rows(items, keys):
        return [[*(r[k] or "-" for k in keys), num(r["calls"]), num(r["errors"]), num(r["persisted"]),
                 num(r["result_chars"]["sum"]), pct(r["chars_share"]), num(r["result_chars"]["p50"]),
                 num(r["result_chars"]["p90"]), num(r["result_chars"]["max"]), num(r["exposure_chars"]),
                 pct(r["exposure_share"])] for r in items]

    tail = ["calls", "errors", "spilled", "result chars", "share", "p50", "p90", "max", "exposure", "share"]
    parts += ["By tool:\n", table(["tool", *tail], rows(t["by_tool"], ("tool",))),
              "\nBy command or path fingerprint, top by exposure:\n",
              table(["tool", "fingerprint", *tail], rows(t["by_fingerprint"], ("tool", "fingerprint")))]
    if t["by_mcp_server"]:
        parts += ["\nBy MCP server:\n", table(["server", *tail], rows(t["by_mcp_server"], ("server",)))]
    parts += ["\nResult characters per call:\n", dist_table([("all calls", t["per_call_result_chars"])])]
    return "\n".join(parts)


def prs_md(p):
    w = p["window_spend"]
    parts = [f"## Cost per merged PR\n\n**{p['label']}** A PR's spend is its whole attributed history;"
             " the window selects which PRs merged.\n",
             f"{num(p['merged_prs'])} PRs merged in the window. The top 10% of them hold {pct(p['top_10pct_share'])}"
             " of their spend.\n",
             table(["per PR"] + DIST_HEAD, [dist_row("est. USD", p["per_pr"]["usd_est"], usd),
                                            dist_row("requests", p["per_pr"]["requests"]),
                                            dist_row("peak context", p["per_pr"]["peak_context"]),
                                            dist_row("unattributed share", p["per_pr"]["unattributed_share"], pct)]),
             "\nSpend in the window by where it was attributed:\n",
             table(["attributed to", "est. USD", "share"],
                   [[k.replace("_", " "), usd(w[k]["usd"]), pct(w[k]["share"])]
                    for k in ("merged_in_window", "other_prs", "unattributed")]),
             "\nBy attribution method:\n",
             table(["method", "est. USD", "share"], [[m, usd(v["usd"]), pct(v["share"])]
                                                     for m, v in p["method_mix"].items()]),
             "\nMost expensive PRs:\n",
             table(["PR", "est. USD", "requests", "turns", "human prompts", "agents", "peak context", "method",
                    "confidence", "unattributed share"],
                   [[f"#{r['pr']}", usd(r["usd_est"]), num(r["requests"]), r["turns"], r["human_prompts"],
                     sum(a["count"] for a in r["agents"].values()), num(r["context"]["peak"]),
                     r["attribution"]["method"] or "-", f"{r['attribution']['confidence']:.2f}",
                     pct(r["attribution"]["unattributed_share"])] for r in p["top"]])]
    return "\n".join(parts)


def cache_md(c):
    rows = [[r["agent_type"], num(r["transcripts"]), num(r["requests"]),
             ", ".join(f"{k} {v}" for k, v in sorted(r["observed_policy"].items())),
             " / ".join(num(r["gaps"][b]) for b in r["gaps"]), usd(r["observed_usd"]), usd(r["sim_5m_usd"]),
             usd(r["sim_1h_usd"]), r["better_policy"], usd(r["saving_vs_other_usd"]),
             " to ".join(usd(v) for v in r["delta_range_usd"]),
             "-" if r["calibration"] is None else f"{r['calibration']:.2f}"] for r in c["by_agent_type"]]
    return "\n".join([f"## Cache-policy simulation\n\n{c['note']}\n",
                      table(["agent type", "transcripts", "requests", "observed TTL",
                             "gaps first / <=5m / 5m-1h / >1h", "observed", "replay 5m", "replay 1h", "better",
                             "difference", "1h - 5m range", "calibration"], rows),
                      f"\nRequests skipped for want of a price: {num(c['unpriced_requests'])}."])


def markdown(report):
    s = report["stamp"]
    head = (f"# Token profile report\n\nGenerated by `tools/token-profile/report`. Aggregates only: no commands,"
            f" transcript text, absolute paths or credentials. Estimated USD is list price, a plan-usage proxy on a"
            f" Max subscription, not a bill.\n\n{stamp_block(s)}\n")
    body = [tokens_md(report["tokens"]), cost_md(report["cost"]), context_md(report["context"]),
            tools_md(report["tools"]), prs_md(report["prs"]), cache_md(report["cache_policy"])]
    return head + "\n" + "\n\n".join(body) + "\n"
