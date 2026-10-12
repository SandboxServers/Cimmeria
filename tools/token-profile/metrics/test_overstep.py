"""Overstep rules O1 to O4 and the wasted-cost estimate: fixture requests, one negative per rule."""

import unittest
from datetime import datetime, timedelta, timezone

from metrics import overstep, registry

CFG = registry.load_config()
WINDOWS = {"lab-record": {"start": "2026-10-01T00:00:00Z", "end": None}}
PACKETS = {"cm-07": {"campaign": "campaign-metrics", "implementer": "packet-coder", "size": "M", "escalated": False}}
T0 = datetime(2026, 10, 11, 10, 0, 0, tzinfo=timezone.utc)


def _iso(dt):
    return dt.strftime("%Y-%m-%dT%H:%M:%SZ")


def _req(**kw):
    """A request dict: Opus 5.5, main session, one million input tokens, no calls, on 2026-10-11."""
    base = {"request_id": "r1", "session_id": "s1", "agent_id": None, "ts": _iso(T0), "model": "claude-opus-5-5",
            "input": 1_000_000, "output": 0, "cache_read": 0, "cache_write_5m": 0, "cache_write_1h": 0,
            "trigger_kind": "mixed", "worktree": None, "agent_type": None, "calls": []}
    base.update(kw)
    return base


def _call(tool, fingerprint=None):
    return {"tool_name": tool, "fingerprint": fingerprint}


def _detect(requests, cfg=CFG, packets=PACKETS):
    return overstep.detect(requests, packets, WINDOWS, cfg)


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
        for fingerprint in ("pwsh tools/build-lane/lane.ps1", "pwsh -File tools\\build-lane\\lane.ps1 cargo test",
                            "lane.ps1 cargo clippy"):
            with self.subTest(fingerprint=fingerprint):
                self.assertEqual(_detect([_req(calls=[_call("Bash", fingerprint)])])["hits"], {"O1": 1})

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
        packets = {"cm-07": dict(PACKETS["cm-07"], escalated=True)}
        self.assertEqual(_detect([self._sonnet()], packets=packets)["hits"], {})


class OpusSubagentTest(unittest.TestCase):
    def test_o4_is_counted_not_priced(self):
        sub = _req(agent_id="a2", agent_type="documentation-writer", worktree=".claude/worktrees/cm-07")
        result = _detect([sub])
        self.assertEqual(result["opus_subagents"], 1)
        self.assertEqual(result["hits"], {})


class RepriceTest(unittest.TestCase):
    def test_worktree_name_is_the_last_segment_lower_cased(self):
        self.assertEqual(overstep.worktree_name(".claude\\worktrees\\CM-07"), "cm-07")
        self.assertIsNone(overstep.worktree_name(None))

    def test_wasted_floors_at_zero(self):
        cheap = _req(model="claude-haiku-5-5", input=10_000)
        self.assertEqual(overstep._waste({"requests": [cheap], "follow_ups": []}, "sonnet", "2026-10-10")[1], 0.0)


if __name__ == "__main__":
    unittest.main()
