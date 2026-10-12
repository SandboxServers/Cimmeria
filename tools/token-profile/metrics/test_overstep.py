"""Overstep rules O1 to O4 and the wasted-cost estimate: fixture requests, one negative per rule."""

import sqlite3
import unittest
from datetime import datetime, timedelta, timezone
from unittest import mock

from metrics import overstep, registry

CFG = registry.load_config()
PACKETS = [("campaign-metrics", {"id": "CM-07", "implementer": "packet-coder", "size": "M", "escalated": False})]
T0 = datetime(2026, 10, 11, 10, 0, 0, tzinfo=timezone.utc)
LAB_ROW = {"pr_number": None, "weight": 1.0, "campaign": "lab-record", "pr_campaign": None}


def _iso(dt):
    return dt.strftime("%Y-%m-%dT%H:%M:%SZ")


def _req(**kw):
    """A request dict: Opus 5.5, main session, one million input tokens, no calls, on 2026-10-11, in lab-record."""
    base = {"request_id": "r1", "session_id": "s1", "agent_id": None, "ts": _iso(T0), "model": "claude-opus-5-5",
            "input": 1_000_000, "output": 0, "cache_read": 0, "cache_write_5m": 0, "cache_write_1h": 0,
            "trigger_kind": "mixed", "worktree": None, "agent_type": None, "calls": [], "rows": [LAB_ROW]}
    base.update(kw)
    return base


def _call(tool, fingerprint=None):
    return {"tool_name": tool, "fingerprint": fingerprint}


def _detect(requests, cfg=CFG, packets=PACKETS, pr_map=None):
    return overstep.detect(requests, packets, cfg, pr_map)


class OpusCodingTest(unittest.TestCase):
    def test_o1_wasted_usd_to_the_cent(self):
        # Opus 5.5 prices 1M input at $4.00; Haiku 5.5 at $0.10, with the long-prompt card (5x) above 100K: $0.50.
        result = _detect([_req(calls=[_call("Edit", "crates/gameserver/src/lib.rs")])])
        rule = result["by_campaign"]["lab-record"]["rules"]["O1"]
        self.assertEqual(round(rule["usd"], 2), 4.00)
        self.assertEqual(round(rule["wasted_usd"], 2), 3.50)
        self.assertEqual(result["hits"], {"O1": 1})

    def test_o1_follow_up_carries_cache_cost_only(self):
        first = _req(input=0, calls=[_call("Edit", "crates/gameserver/src/lib.rs")])
        follow = _req(request_id="r2", ts=_iso(T0 + timedelta(seconds=30)), input=0, cache_read=100_000)
        rule = _detect([first, follow])["by_campaign"]["lab-record"]["rules"]["O1"]
        self.assertEqual(round(rule["usd"], 2), 0.02)

    def test_o1_run_of_a_build_lane(self):
        result = _detect([_req(calls=[_call("Bash", "cargo test")])])
        self.assertEqual(result["hits"], {"O1": 1})

    def test_o1_run_of_the_powershell_lane(self):
        result = _detect([_req(calls=[_call("PowerShell", "pwsh tools/build-lane/lane.ps1")])])
        self.assertEqual(result["hits"], {"O1": 1})

    def test_o1_negative_other_pwsh_command_is_not_a_build_run(self):
        self.assertEqual(_detect([_req(calls=[_call("PowerShell", "pwsh")])])["hits"], {})

    def test_o1_negative_docs_edit_is_allowed_even_when_code_path_matches(self):
        # docs/ is an allowed coordinator path; it is made a code path here so only the exclusion keeps it out.
        cfg = dict(CFG, overstep=dict(CFG["overstep"], code_paths=CFG["overstep"]["code_paths"] + ["docs/"]))
        result = _detect([_req(calls=[_call("Edit", "docs/analysis/x/README.md")])], cfg=cfg)
        self.assertEqual(result["hits"], {})

    def test_o1_negative_subagent_is_not_coordinator_work(self):
        result = _detect([_req(agent_id="a1", agent_type="rust-gameserver-dev",
                               calls=[_call("Edit", "crates/gameserver/src/lib.rs")])])
        self.assertEqual(result["hits"], {})

    def test_before_regime_switch_is_ignored(self):
        early = _req(ts="2026-10-09T10:00:00Z", calls=[_call("Edit", "crates/gameserver/src/lib.rs")])
        self.assertEqual(_detect([early])["hits"], {})


class ResearchTest(unittest.TestCase):
    def test_o2_research_tool_call(self):
        result = _detect([_req(model="claude-opus-5", calls=[_call("mcp__ghidra__decompile_function")])])
        self.assertEqual(result["hits"], {"O2": 1})

    def test_o2_read_burst_of_25_code_reads_in_10_minutes(self):
        reqs = [_req(request_id=f"r{i}", ts=_iso(T0 + timedelta(seconds=20 * i)),
                     calls=[_call("Read", "crates/gameserver/src/lib.rs")]) for i in range(25)]
        result = _detect(reqs)
        self.assertEqual(result["hits"], {"O2": 1})
        self.assertEqual(result["by_campaign"]["lab-record"]["rules"]["O2"]["count"], 1)

    def test_o2_negative_24_reads_are_no_burst(self):
        reqs = [_req(request_id=f"r{i}", ts=_iso(T0 + timedelta(seconds=20 * i)),
                     calls=[_call("Read", "crates/gameserver/src/lib.rs")]) for i in range(24)]
        self.assertEqual(_detect(reqs)["hits"], {})

    def test_o2_negative_coordinator_reads_a_ledger(self):
        reqs = [_req(request_id=f"r{i}", ts=_iso(T0 + timedelta(seconds=20 * i)),
                     calls=[_call("Read", "docs/analysis/campaign-metrics/README.md")]) for i in range(30)]
        self.assertEqual(_detect(reqs)["hits"], {})


class SonnetImplementingTest(unittest.TestCase):
    def _sonnet(self, **kw):
        base = {"model": "claude-sonnet-5-5", "agent_id": "a1", "agent_type": "rust-gameserver-dev",
                "worktree": ".claude/worktrees/cm-07", "calls": [_call("Edit", "crates/gameserver/src/lib.rs")]}
        base.update(kw)
        return _req(**base)

    def test_o3_sonnet_edits_a_packet_coders_small_packet(self):
        self.assertEqual(_detect([self._sonnet()])["hits"], {"O3": 1})

    def test_o3_negative_review_fixer_edit_is_expected(self):
        fixer = self._sonnet(agent_type="packet-review-fixer")
        self.assertEqual(_detect([fixer])["hits"], {})

    def test_o3_negative_escalated_packet(self):
        packets = [("campaign-metrics", dict(PACKETS[0][1], escalated=True))]
        self.assertEqual(_detect([self._sonnet()], packets=packets)["hits"], {})

    def test_o3_sonnet_main_session_edit_on_a_packet_coders_packet(self):
        req = _req(model="claude-sonnet-5-5", worktree="lr03", calls=[_call("Edit", "crates/gameserver/src/lib.rs")])
        self.assertEqual(_detect([req], packets=[("lab-record", _pkt("LR-03"))])["hits"], {"O3": 1})


class ImplementingTest(unittest.TestCase):
    """O3 (Sonnet) and O3b (Opus) subagents editing a packet-coder's packet in worktree lr03 (packet LR-03)."""

    def _subagent(self, model, **kw):
        base = {"model": model, "agent_id": "a1", "agent_type": "rust-gameserver-dev", "worktree": "lr03",
                "calls": [_call("Edit", "crates/gameserver/src/lib.rs")]}
        base.update(kw)
        return _req(**base)

    def test_o3_and_o3b_sonnet_and_opus_subagents_both_hit(self):
        packets = [("lab-record", _pkt("LR-03"))]
        reqs = [self._subagent("claude-sonnet-5-5"), self._subagent("claude-opus-5-5", request_id="r2")]
        result = _detect(reqs, packets=packets)
        self.assertEqual(result["hits"], {"O3": 1, "O3b": 1})
        self.assertEqual(sorted(item["family"] for item in result["by_campaign"]["lab-record"]["items"]),
                         ["opus", "sonnet"])

    def test_o3b_is_repriced_at_the_haiku_tier_and_carries_a_caveat(self):
        result = _detect([self._subagent("claude-opus-5-5")], packets=[("lab-record", _pkt("LR-03"))])
        rule = result["by_campaign"]["lab-record"]["rules"]["O3b"]
        self.assertEqual(round(rule["usd"], 2), 4.00)
        self.assertEqual(round(rule["wasted_usd"], 2), 3.50)
        self.assertIn("upper bound", result["caveats"]["O3b"])

    def test_o3_and_o3b_negative_escalated_packet(self):
        packets = [("lab-record", dict(_pkt("LR-03"), escalated=True))]
        reqs = [self._subagent("claude-sonnet-5-5"), self._subagent("claude-opus-5-5", request_id="r2")]
        self.assertEqual(_detect(reqs, packets=packets)["hits"], {})

    def test_o3b_negative_reviewer_edit_is_expected(self):
        reviewer = self._subagent("claude-opus-5-5", agent_type="packet-reviewer")
        self.assertEqual(_detect([reviewer], packets=[("lab-record", _pkt("LR-03"))])["hits"], {})


class ClaimTest(unittest.TestCase):
    CODE = [_call("Edit", "crates/gameserver/src/lib.rs")]

    def test_three_consecutive_o1_requests_price_each_request_once(self):
        # r1 to r3 edit code; f1 to f3 are plain follow-ups between them, each reading 100K from the cache.
        stream = []
        for i in range(1, 4):
            stream.append(_req(request_id=f"r{i}", ts=_iso(T0 + timedelta(seconds=20 * i)), calls=self.CODE))
            stream.append(_req(request_id=f"f{i}", ts=_iso(T0 + timedelta(seconds=20 * i + 10)), input=0, cache_read=100_000))
        result = _detect(stream)
        self.assertEqual(result["hits"], {"O1": 3})
        rule = result["by_campaign"]["lab-record"]["rules"]["O1"]
        self.assertAlmostEqual(rule["usd"], sum(overstep.actual_usd(r) for r in stream))

    def test_a_request_matching_o1_and_o2_counts_once_under_o1(self):
        req = _req(calls=self.CODE + [_call("mcp__ghidra__decompile_function")])
        self.assertEqual(_detect([req])["hits"], {"O1": 1})


class OpusSubagentTest(unittest.TestCase):
    def test_o4_is_counted_not_priced(self):
        sub = _req(agent_id="a2", agent_type="documentation-writer", worktree=".claude/worktrees/cm-07")
        result = _detect([sub])
        self.assertEqual(result["opus_subagents"], 1)
        self.assertEqual(result["hits"], {})

    def test_opus_subagents_count_distinct_agents(self):
        subs = [_req(request_id=f"r{i}", agent_id=agent, agent_type="general-purpose", worktree=".claude/worktrees/cm-07")
                for i, agent in enumerate(["a1", "a1", "a1", "a2", "a2"])]
        result = _detect(subs)
        self.assertEqual(result["requests"], 5)
        self.assertEqual(result["opus_subagents"], 2)


def _pkt(packet_id):
    return {"id": packet_id, "implementer": "packet-coder", "size": "S", "escalated": False}


class PacketForTest(unittest.TestCase):
    def test_a_dashless_worktree_name_matches_the_packet(self):
        found = overstep.packet_for({"worktree": "lr03"}, [("lab-record", _pkt("LR-03"))])
        self.assertEqual(found, ("lab-record", _pkt("LR-03")))

    def test_an_id_held_by_two_campaigns_matches_nothing(self):
        packets = [("lab-a", _pkt("LX-01")), ("lab-b", _pkt("LX-01"))]
        self.assertIsNone(overstep.packet_for({"worktree": "lx-01"}, packets))

    def test_requests_of_one_session_match_the_packet_once(self):
        reqs = [_req(request_id=f"r{i}", worktree="lr03", agent_name="lr03-dev") for i in range(50)]
        with mock.patch("metrics.overstep.packet_of_session", return_value="LR-03") as matcher:
            result = _detect(reqs, packets=[("lab-record", _pkt("LR-03"))])
        self.assertEqual(matcher.call_count, 1)
        self.assertEqual(result["requests"], 50)


class RequestSqlTest(unittest.TestCase):
    def test_a_named_agent_takes_its_custom_agent_type(self):
        db = sqlite3.connect(":memory:")
        db.row_factory = sqlite3.Row
        db.executescript("""
            CREATE TABLE sessions (session_id TEXT, worktree TEXT);
            CREATE TABLE triggers (trigger_id INTEGER, kind TEXT);
            CREATE TABLE agents (agent_id TEXT, agent_type TEXT, custom_agent_type TEXT, name TEXT, worktree TEXT);
            CREATE TABLE requests (request_id TEXT, session_id TEXT, agent_id TEXT, ts TEXT, model TEXT,
                input_tokens INTEGER, output_tokens INTEGER, cache_read INTEGER, cache_write_5m INTEGER,
                cache_write_1h INTEGER, trigger_id INTEGER, cwd_worktree TEXT);
            INSERT INTO sessions VALUES ('s1', NULL);
            INSERT INTO triggers VALUES (1, 'mixed');
            INSERT INTO agents VALUES ('a1', 'lr07-dev', 'rust-gameserver-dev', 'lr07-dev', '.claude/worktrees/lr07');
            INSERT INTO requests VALUES ('r1', 's1', 'a1', '2026-10-11T10:00:00Z', 'claude-sonnet-5-5', 1, 0, 0, 0, 0, 1,
                NULL);
        """)
        row = db.execute(overstep.REQUEST_SQL).fetchone()
        self.assertEqual(row["agent_type"], "rust-gameserver-dev")
        self.assertEqual(row["agent_name"], "lr07-dev")
        self.assertEqual(row["worktree"], ".claude/worktrees/lr07")


class WindowTest(unittest.TestCase):
    def test_a_fractional_second_at_the_regime_switch_is_included(self):
        # As text "...54.500Z" sorts before the switch "...54Z", so a string compare dropped it.
        req = _req(ts="2026-10-10T20:02:54.500Z", calls=[_call("Edit", "crates/gameserver/src/lib.rs")])
        self.assertEqual(_detect([req])["requests"], 1)

    def test_fixer_requests_are_counted_per_worktree_for_fixer_heavy(self):
        fixer = _req(agent_id="a3", agent_type="packet-review-fixer", worktree=".claude/worktrees/cm-07")
        result = _detect([fixer, _req(request_id="r3", worktree=".claude/worktrees/cm-07")])
        self.assertEqual(result["fixer_requests"], {"cm-07": 1})


class AttributionTest(unittest.TestCase):
    CODE = [_call("Edit", "crates/gameserver/src/lib.rs")]

    def test_a_pr_attribution_row_names_the_campaign_over_overlapping_open_windows(self):
        window = {"start": "2026-10-01T00:00:00Z", "end": None}
        ledger = {"campaigns": {"camp-a": {"window": window, "packets": []},
                                "camp-b": {"window": window, "packets": []}}}
        packets, _, pr_map = overstep.ledger_inputs(ledger)
        row = {"pr_number": None, "weight": 1.0, "campaign": "camp-b", "pr_campaign": None}
        result = _detect([_req(calls=self.CODE, rows=[row])], packets=packets, pr_map=pr_map)
        self.assertEqual(list(result["by_campaign"]), ["camp-b"])

    def test_a_pr_placeholder_row_takes_its_pr_map_campaign(self):
        row = {"pr_number": 9, "weight": 1.0, "campaign": None, "pr_campaign": "pr-9"}
        self.assertEqual(overstep._campaign_of_row(row, {9: "x"}, None), "x")

    def test_a_pr_placeholder_with_no_map_entry_and_no_packet_is_unattributed(self):
        row = {"pr_number": 9, "weight": 1.0, "campaign": None, "pr_campaign": "pr-9"}
        self.assertEqual(overstep._campaign_of_row(row, {}, None), "unattributed")

    def test_a_request_with_no_attribution_and_no_packet_is_unattributed(self):
        result = _detect([_req(calls=self.CODE, rows=[], worktree=None)])
        self.assertEqual(list(result["by_campaign"]), ["unattributed"])

    def test_a_request_split_evenly_gives_each_campaign_half_the_waste(self):
        rows = [{"pr_number": None, "weight": 0.5, "campaign": "camp-a", "pr_campaign": None},
                {"pr_number": None, "weight": 0.5, "campaign": "camp-b", "pr_campaign": None}]
        whole = _detect([_req(calls=self.CODE)])["by_campaign"]["lab-record"]["rules"]["O1"]["wasted_usd"]
        by_campaign = _detect([_req(calls=self.CODE, rows=rows)])["by_campaign"]
        for name in ("camp-a", "camp-b"):
            self.assertAlmostEqual(by_campaign[name]["rules"]["O1"]["wasted_usd"], whole / 2)

    def test_campaign_usd_sums_to_the_total_of_the_hits(self):
        split = [{"pr_number": None, "weight": 0.25, "campaign": "camp-a", "pr_campaign": None},
                 {"pr_number": None, "weight": 0.75, "campaign": None, "pr_campaign": "camp-b"}]
        pr_tagged = [{"pr_number": 7, "weight": 1.0, "campaign": None, "pr_campaign": "camp-c"}]
        reqs = [_req(request_id=f"r{i}", session_id=f"s{i}", calls=self.CODE, rows=rows)
                for i, rows in enumerate([split, [], pr_tagged])]
        result = _detect(reqs)
        usd = sum(row["usd"] for stats in result["by_campaign"].values() for row in stats["rules"].values())
        self.assertAlmostEqual(usd, sum(overstep.actual_usd(r) for r in reqs))
        self.assertEqual(set(result["by_campaign"]), {"camp-a", "camp-b", "camp-c", "unattributed"})


class RepriceTest(unittest.TestCase):
    def test_worktree_name_is_the_last_segment_lower_cased(self):
        self.assertEqual(overstep.worktree_name(".claude\\worktrees\\CM-07"), "cm-07")
        self.assertIsNone(overstep.worktree_name(None))

    def test_wasted_floors_at_zero(self):
        cheap = _req(model="claude-haiku-5-5", input=10_000)
        self.assertEqual(overstep._waste({"requests": [cheap], "follow_ups": []}, "sonnet", "2026-10-10")[1], 0.0)


if __name__ == "__main__":
    unittest.main()
