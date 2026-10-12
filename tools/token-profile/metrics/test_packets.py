"""The packet join (CM-06): session-to-packet matching, stop reasons, the join of a fixture database and the packets command.

The fixture campaign `lab-record` has three packets: LR-01 (A: one coder, one reviewer, no fixes), LR-02 (B: a coder
that stalls in the background, a coder that exhausts its context, then a Sonnet general-purpose agent in the same
worktree) and LR-03 (C: one coder, one reviewer, one fixer).
"""

import contextlib
import io
import json
import shutil
import sqlite3
import tempfile
import unittest
from pathlib import Path

from metrics import packets, registry
from metrics.cli import main

HERE = Path(__file__).resolve().parent
SCHEMA = HERE.parent / "schema.sql"
CAMPAIGN = "lab-record"
CONFIG = registry.load_config()


def _row(packet_id, status="Integrated", prs=(), escalated=False):
    """A ledger packet row as build_ledger writes it."""
    return {"id": packet_id, "title": "", "status": status, "status_raw": status, "prs": list(prs),
            "implementer": "haiku", "implementer_note": None, "escalated": escalated, "retry_reason": None,
            "size": "", "wave": "", "depends": "", "override": False}


def _ledger():
    """The ledger document of the fixture campaign."""
    rows = [_row("LR-01", prs=[101]), _row("LR-02", prs=[102]), _row("LR-03", prs=[103])]
    return {"schema": "cimmeria-campaign-ledger/1",
            "campaigns": {CAMPAIGN: {"packets": rows,
                                     "window": {"start": "2026-10-01T00:00:00Z", "end": "2026-10-03T00:00:00Z"}}}}


def _github():
    """The github.json document of the fixture PRs: 101 has no reviewer fix, 102 and 103 have commits on record."""
    return {"prs": [
        {"number": 101, "mergedAt": "2026-10-01T12:00:00Z",
         "commits": [{"sha": "a1", "committed": "2026-10-01T08:00:00Z"}]},
        {"number": 102, "mergedAt": "2026-10-02T12:00:00Z",
         "commits": [{"sha": "b1", "committed": "2026-10-01T09:00:00Z"}]},
        {"number": 103, "mergedAt": "2026-10-01T16:00:00Z",
         "commits": [{"sha": "c1", "committed": "2026-10-01T08:00:00Z"},
                     {"sha": "c2", "committed": "2026-10-01T11:30:00Z"}]}]}


WORKTREE = ".claude/worktrees/{}"


def _agent(agent_id, worktree, first_ts, agent_type="general-purpose", custom=None, name=None, model="haiku"):
    """An agents row; the session is the agent's own."""
    return {"agent_id": agent_id, "session_id": "s-" + agent_id, "agent_type": agent_type,
            "custom_agent_type": custom, "name": name, "model_alias": model, "worktree": worktree,
            "first_ts": first_ts}


AGENTS = [
    _agent("a-coder", WORKTREE.format("lr-01"), "2026-10-01T06:00:00Z", "packet-coder", "packet-coder"),
    _agent("a-reviewer", WORKTREE.format("lr-01"), "2026-10-01T10:00:00Z", "packet-reviewer", "packet-reviewer"),
    _agent("b-coder-1", WORKTREE.format("lr-02"), "2026-10-01T06:00:00Z", "packet-coder", "packet-coder"),
    _agent("b-coder-2", WORKTREE.format("lr-02"), "2026-10-01T07:30:00Z", "packet-coder", "packet-coder"),
    _agent("b-sonnet", WORKTREE.format("lr-02"), "2026-10-01T08:30:00Z", model="sonnet"),
    _agent("c-coder", WORKTREE.format("lr-03"), "2026-10-01T06:00:00Z", "packet-coder", "packet-coder"),
    _agent("c-reviewer", WORKTREE.format("lr-03"), "2026-10-01T09:00:00Z", "packet-reviewer", "packet-reviewer"),
    _agent("c-fixer", WORKTREE.format("lr-03"), "2026-10-01T11:00:00Z", "packet-review-fixer", "packet-review-fixer"),
    _agent("scratch", WORKTREE.format("scratch"), "2026-10-01T12:00:00Z", "lab-driver", "lab-driver"),
]


def _stop(agent_id, **fields):
    """An agent_stops row: a clean end_turn at 40k context unless `fields` says otherwise."""
    row = {"agent_id": agent_id, "session_id": "s-" + agent_id, "last_ts": "2026-10-01T06:00:00Z", "requests": 10,
           "last_stop_reason": "end_turn", "last_context_tokens": 40_000, "last_tool_name": None,
           "last_tool_background": 0, "last_result_is_error": None, "ended_after_result": 0,
           "ended_in_api_error": 0, "api_error_kind": None}
    row.update(fields)
    return row


STOPS = [
    _stop("a-coder"),
    _stop("a-reviewer"),
    _stop("b-coder-1", last_stop_reason="tool_use", last_context_tokens=96_000, last_tool_name="Task",
          last_tool_background=1, ended_after_result=0),
    _stop("b-coder-2", last_context_tokens=91_000),
    _stop("c-coder"),
    _stop("c-reviewer"),
    _stop("c-fixer"),
]


BRANCHES = [
    {"worktree": WORKTREE.format("lr-02"), "branch": "lab-record/LR-09-old-slug", "observed_at": "2026-09-30T00:00:00Z",
     "source": "lane-log"},
    {"worktree": WORKTREE.format("lr-02"), "branch": "lab-record/LR-02-stall-slug", "observed_at": "2026-10-01T00:00:00Z",
     "source": "lane-log"},
]


def _insert(db, table, rows):
    """Insert each dict as a row of `table`, its keys as the columns."""
    for row in rows:
        columns = ", ".join(row)
        marks = ", ".join(f":{key}" for key in row)
        db.execute(f"INSERT INTO {table} ({columns}) VALUES ({marks})", row)


def _build_db(path):
    """A profiler database at `path` with the schema and the fixture rows."""
    db = sqlite3.connect(path)
    db.executescript(SCHEMA.read_text(encoding="utf-8"))
    _insert(db, "sessions", [{"session_id": a["session_id"], "project_dir": "repo", "first_ts": a["first_ts"],
                              "last_ts": a["first_ts"]} for a in AGENTS])
    _insert(db, "agents", AGENTS)
    _insert(db, "agent_stops", STOPS)
    _insert(db, "worktree_branches", BRANCHES)
    db.commit()
    db.close()


class FixtureTest(unittest.TestCase):
    """A temporary profiler database, ledger and github file for the fixture campaign."""

    @classmethod
    def setUpClass(cls):
        cls.dir = Path(tempfile.mkdtemp())
        cls.db_path = cls.dir / "profile.sqlite"
        _build_db(cls.db_path)
        cls.db = packets.load_db(cls.db_path)

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.dir, ignore_errors=True)

    def join(self, agents=None, github=None):
        """The packet join of the fixture campaign, with `agents` replacing the database's when given."""
        db = dict(self.db, agents=agents) if agents is not None else self.db
        return packets.join_campaigns(_ledger(), db, github or _github(), CONFIG)


class SessionMatchTest(unittest.TestCase):
    """packet_of_session: worktree, branch hint, then name; a name that is not one packet matches none."""

    ROWS = [{"id": "LR-1"}, {"id": "LR-10"}]

    def test_worktree_name_matches(self):
        agent = _agent("x", WORKTREE.format("lr-01"), None)
        self.assertEqual(packets.packet_of_session(agent, agent["worktree"], None, _row_ids("LR-01", "LR-02"),
                                                   CAMPAIGN), "LR-01")

    def test_packet_id_end_is_bounded(self):
        """`lr-1` does not match inside `lr-10`, and `lr-10` does not match `lr-1`."""
        agent = _agent("x", None, None)
        for worktree, expected in (("lr-10", "LR-10"), ("lr-1", "LR-1"), ("lr-100", None)):
            with self.subTest(worktree=worktree):
                self.assertEqual(packets.packet_of_session(agent, worktree, None, self.ROWS, CAMPAIGN), expected)

    def test_shorter_id_yields_to_the_longer_one_it_prefixes(self):
        """`tp-05b` names TP-05b, not TP-05; `tp-05` names TP-05; `tp05b-fix` names TP-05b."""
        agent = _agent("x", None, None)
        rows = _row_ids("TP-05", "TP-05b")
        for worktree, expected in (("tp-05b", "TP-05b"), ("tp-05", "TP-05"), ("tp05b-fix", "TP-05b")):
            with self.subTest(worktree=worktree):
                self.assertEqual(packets.packet_of_session(agent, worktree, None, rows, CAMPAIGN), expected)

    def test_branch_hint_strips_the_campaign_prefix(self):
        agent = _agent("x", None, None)
        self.assertEqual(packets.packet_of_session(agent, None, "lab-record/LR-02-stall-slug", _row_ids("LR-02"),
                                                   CAMPAIGN), "LR-02")

    def test_name_is_the_fallback(self):
        agent = _agent("x", WORKTREE.format("scratch"), None, name="LR-03 reviewer")
        self.assertEqual(packets.packet_of_session(agent, agent["worktree"], None, _row_ids("LR-03"), CAMPAIGN),
                         "LR-03")

    def test_name_comes_before_worktree(self):
        agent = _agent("x", WORKTREE.format("lr-02"), None, name="LR-01 reviewer")
        self.assertEqual(packets.packet_of_session(agent, agent["worktree"], None, _row_ids("LR-01", "LR-02"),
                                                   CAMPAIGN), "LR-01")

    def test_worktree_and_branch_are_separate_steps(self):
        """The worktree names LR-01 and the branch LR-02: the worktree step matches, the texts are not pooled."""
        agent = _agent("x", WORKTREE.format("lr-01"), None)
        self.assertEqual(packets.packet_of_session(agent, agent["worktree"], "lab-record/LR-02-slug",
                                                   _row_ids("LR-01", "LR-02"), CAMPAIGN), "LR-01")

    def test_name_that_names_two_packets_matches_neither(self):
        agent = _agent("x", None, None, name="LR-01 and LR-02")
        self.assertIsNone(packets.packet_of_session(agent, None, None, _row_ids("LR-01", "LR-02"), CAMPAIGN))


def _row_ids(*ids):
    """Ledger rows with only the IDs, which is all packet_of_session reads."""
    return [{"id": packet_id} for packet_id in ids]


class StopReasonTest(unittest.TestCase):
    """stop_class and stop_reason follow the Contract's by_stop rule and the retry reasons."""

    def test_stop_class(self):
        self.assertEqual(packets.stop_class(None), "no_record")
        self.assertEqual(packets.stop_class(_stop("x", ended_in_api_error=1, api_error_kind="rate_limit")), "error")
        self.assertEqual(packets.stop_class(_stop("x", last_stop_reason="tool_use")), "tool_use")

    def test_background_stall_is_checked_before_context(self):
        stop = _stop("x", last_stop_reason="tool_use", last_context_tokens=96_000, last_tool_background=1)
        self.assertEqual(packets.stop_reason(stop), "stalled-background")

    def test_context_at_cutoff_is_exhausted(self):
        self.assertEqual(packets.stop_reason(_stop("x", last_context_tokens=90_000)), "context-exhausted")

    def test_refused_tool_call_is_permission_denied(self):
        stop = _stop("x", last_stop_reason="tool_use", last_result_is_error=1)
        self.assertEqual(packets.stop_reason(stop), "permission-denied")

    def test_api_error_and_missing_record_are_other(self):
        self.assertEqual(packets.stop_reason(_stop("x", ended_in_api_error=1)), "other")
        self.assertEqual(packets.stop_reason(None), "other")


class JoinTest(FixtureTest):
    """The join of the fixture campaign: the three packets, the sessions and the stop context."""

    def test_packet_a_is_a_first_pass(self):
        record = _record(self.join(), "LR-01")
        self.assertTrue(record["accepted"])
        self.assertEqual((record["implementer_sessions"], record["reviewer_sessions"], record["fixer_sessions"]),
                         (1, 1, 0))
        self.assertEqual(record["fix_commits"], 0)
        self.assertIs(record["first_pass"], True)
        self.assertEqual(record["cycle_hours"], 6.0)

    def test_packet_b_escalates_after_a_stall_and_an_exhausted_context(self):
        record = _record(self.join(), "LR-02")
        self.assertTrue(record["escalated"])
        self.assertEqual(record["escalation_to"], "general-purpose")
        self.assertEqual(record["retries"], ["stalled-background"])
        self.assertEqual(record["why"], "context-exhausted")
        self.assertIs(record["first_pass"], False)
        self.assertEqual(record["cycle_hours"], 30.0)

    def test_packet_c_has_a_fix_and_is_not_a_first_pass(self):
        record = _record(self.join(), "LR-03")
        self.assertEqual(record["fixer_sessions"], 1)
        self.assertEqual(record["fix_commits"], 1)
        self.assertIs(record["first_pass"], False)
        self.assertEqual(record["cycle_hours"], 10.0)

    def test_campaign_rates_and_escalations(self):
        campaign = self.join()["campaigns"][CAMPAIGN]
        self.assertEqual(campaign["first_pass"]["rate"], 0.33)
        self.assertEqual(campaign["first_pass"]["eligible"], 3)
        self.assertEqual(campaign["retries"]["by_reason"]["stalled-background"], 1)
        self.assertEqual(campaign["escalation"]["escalated"], 1)
        self.assertEqual(campaign["escalation"]["items"], [{"id": "LR-02", "to": "general-purpose",
                                                            "why": "context-exhausted"}])
        self.assertEqual(campaign["cycle_time"]["campaign_hours"], 48.0)

    def test_stop_context(self):
        stop_context = self.join()["campaigns"][CAMPAIGN]["stop_context"]
        self.assertEqual(stop_context["by_stop"], {"end_turn": 6, "tool_use": 1, "error": 0, "no_record": 1})
        self.assertEqual(stop_context["context_at_stop"]["max"], 96_000)
        self.assertEqual(stop_context["share_above_90k"], 0.29)

    def test_sessions_count_the_unmapped_scratch_agent(self):
        sessions = self.join()["sessions"]
        self.assertEqual((sessions["total"], sessions["mapped"], sessions["unmapped"]), (8, 7, 1))
        self.assertEqual(sessions["unmapped_share"], 0.12)

    def test_load_db_takes_the_latest_branch_of_a_worktree(self):
        by_id = {a["agent_id"]: a for a in self.db["agents"]}
        self.assertEqual(by_id["b-coder-1"]["branch_hint"], "lab-record/LR-02-stall-slug")

    def test_worktree_revert_leaves_every_session_unmapped(self):
        """Without the worktree and branch, only the name could map a session, and no fixture name does."""
        stripped = [dict(a, worktree=None, branch_hint=None) for a in self.db["agents"]]
        sessions = self.join(agents=stripped)["sessions"]
        self.assertEqual((sessions["mapped"], sessions["unmapped"]), (0, 8))

    def test_escalated_packet_without_a_coder_keeps_its_retry_reason(self):
        ledger_doc = _ledger()
        ledger_doc["campaigns"][CAMPAIGN]["packets"][1]["escalated"] = True
        ledger_doc["campaigns"][CAMPAIGN]["packets"][1]["retry_reason"] = "ci-failure"
        record = _record(packets.join_campaigns(ledger_doc, {"agents": [], "stops": {}}, _github(), CONFIG), "LR-02")
        self.assertEqual((record["implementer_sessions"], record["why"]), (0, "ci-failure"))

    def test_packet_without_a_coder_is_judged_on_its_commits(self):
        agents = [_agent("dom", WORKTREE.format("lr-01"), "2026-10-01T06:00:00Z", "rust-gameserver-dev",
                         "rust-gameserver-dev"), AGENTS[1]]
        github = _github()
        self.assertIs(_record(self.join(agents=agents, github=github), "LR-01")["first_pass"], True)
        github["prs"][0]["commits"].append({"sha": "a2", "committed": "2026-10-01T11:00:00Z"})
        self.assertIs(_record(self.join(agents=agents, github=github), "LR-01")["first_pass"], False)

    def test_domain_implementer_at_opus_escalates(self):
        agents = [_agent("a-coder", WORKTREE.format("lr-01"), "2026-10-01T06:00:00Z", "packet-coder", "packet-coder"),
                  _agent("dom", WORKTREE.format("lr-01"), "2026-10-01T07:00:00Z", "rust-gameserver-dev",
                         "rust-gameserver-dev", model=None)]
        agents[1]["request_model"] = "claude-opus-5-5"
        record = _record(self.join(agents=agents), "LR-01")
        self.assertTrue(record["escalated"])
        self.assertEqual(record["escalation_to"], "implementer-domain")

    def test_escalation_skips_an_unmapped_sonnet_agent_started_after_merge(self):
        late = _agent("late-sonnet", WORKTREE.format("lr-01"), "2026-10-01T13:00:00Z", name="lr-02-sonnet",
                      model="claude-sonnet-5")
        record = _record(self.join(agents=[AGENTS[0], late]), "LR-01")
        self.assertFalse(record["escalated"])

    def test_escalation_takes_a_sonnet_agent_mapped_to_the_packet_after_merge(self):
        late = _agent("late-sonnet", WORKTREE.format("lr-01"), "2026-10-01T13:00:00Z", model="claude-sonnet-5")
        record = _record(self.join(agents=[AGENTS[0], late]), "LR-01")
        self.assertEqual(record["escalation_to"], "general-purpose")

    def test_negative_cycle_time_is_none(self):
        github = _github()
        github["prs"][0]["mergedAt"] = "2026-09-30T00:00:00Z"
        self.assertIsNone(_record(self.join(github=github), "LR-01")["cycle_hours"])

    def test_pr_without_commits_has_no_first_pass(self):
        github = _github()
        del github["prs"][0]["commits"]
        record = _record(self.join(github=github), "LR-01")
        self.assertIsNone(record["fix_commits"])
        self.assertIsNone(record["first_pass"])
        self.assertEqual(self.join(github=github)["campaigns"][CAMPAIGN]["first_pass"]["eligible"], 2)


class PacketsCommandTest(FixtureTest):
    """The packets subcommand writes packets.json and reports a missing input as status=error."""

    def test_writes_packets_json(self):
        out = self.dir / "out"
        ledger_path, github_path = self.dir / "ledger.json", self.dir / "github.json"
        ledger_path.write_text(json.dumps(_ledger()), encoding="utf-8")
        github_path.write_text(json.dumps(_github()), encoding="utf-8")
        argv = ["packets", "--db", str(self.db_path), "--ledger", str(ledger_path), "--github", str(github_path),
                "--out", str(out)]
        with contextlib.redirect_stdout(io.StringIO()) as printed:
            self.assertEqual(main(argv), 0)
        written = json.loads((out / "packets.json").read_text(encoding="utf-8"))
        self.assertEqual(written["schema"], packets.SCHEMA)
        self.assertIn('"packets": {"lab-record": 3}', printed.getvalue())

    def test_missing_ledger_is_status_error(self):
        argv = ["packets", "--db", str(self.db_path), "--ledger", str(self.dir / "missing.json"),
                "--github", str(self.dir / "missing.json"), "--out", str(self.dir / "out")]
        with contextlib.redirect_stderr(io.StringIO()) as err:
            self.assertEqual(main(argv), 2)
        self.assertIn("status=error FileNotFoundError", err.getvalue())


class MissingCommitsTest(unittest.TestCase):
    """A campaign whose PRs are unknown: the join returns, with no first pass and no PR cycle time."""

    def test_no_pr_facts_gives_null_first_pass(self):
        ledger_doc = _ledger()
        ledger_doc["campaigns"][CAMPAIGN]["packets"][0]["prs"] = []
        db = {"agents": [a for a in AGENTS if a["worktree"] == WORKTREE.format("lr-01")],
              "stops": {s["agent_id"]: s for s in STOPS}}
        record = _record(packets.join_campaigns(ledger_doc, db, {"prs": []}, CONFIG), "LR-01")
        self.assertIsNone(record["first_pass"])
        self.assertIsNone(record["cycle_hours"])


class LoadDbTest(unittest.TestCase):
    """load_db: a NULL worktree takes the one its tool calls ran in; the branch hint is the one observed by the agent's
    last record, else the worktree's latest branch."""

    def setUp(self):
        self.dir = Path(tempfile.mkdtemp())
        self.path = self.dir / "profile.sqlite"
        db = sqlite3.connect(self.path)
        db.executescript(SCHEMA.read_text(encoding="utf-8"))
        db.execute("PRAGMA foreign_keys = OFF")  # the tool call's request is not needed here
        agents = [_agent("n-1", None, "2026-10-01T06:00:00Z"),
                  _agent("b-1", WORKTREE.format("lr-02"), "2026-10-01T06:00:00Z"),
                  _agent("b-2", WORKTREE.format("lr-02"), "2026-10-01T06:00:00Z")]
        _insert(db, "sessions", [{"session_id": "s-" + a["agent_id"], "project_dir": "repo",
                                  "first_ts": a["first_ts"], "last_ts": a["first_ts"]} for a in agents])
        _insert(db, "agents", [dict(a, session_id="s-" + a["agent_id"]) for a in agents])
        db.execute("UPDATE agents SET last_ts = '2026-10-01T09:00:00Z' WHERE agent_id = 'b-1'")
        db.execute("UPDATE agents SET last_ts = '2026-09-01T00:00:00Z' WHERE agent_id = 'b-2'")
        _insert(db, "tool_calls", [{"tool_use_id": "t-1", "request_id": "r-1", "session_id": "s-n-1",
                                    "agent_id": "n-1", "ts": "2026-10-01T06:10:00Z", "tool_name": "Read",
                                    "worktree": WORKTREE.format("lr-01")}])
        _insert(db, "worktree_branches", [
            {"worktree": WORKTREE.format("lr-02"), "branch": "lab-record/LR-09-old-slug",
             "observed_at": "2026-09-30T00:00:00Z", "source": "lane-log"},
            {"worktree": WORKTREE.format("lr-02"), "branch": "lab-record/LR-03-new-slug",
             "observed_at": "2026-10-02T00:00:00Z", "source": "lane-log"}])
        db.commit()
        db.close()
        self.db = packets.load_db(self.path)

    def tearDown(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def test_null_worktree_takes_the_tool_call_worktree(self):
        by_id = {a["agent_id"]: a for a in self.db["agents"]}
        self.assertEqual(by_id["n-1"]["worktree"], WORKTREE.format("lr-01"))
        only_n1 = dict(self.db, agents=[dict(by_id["n-1"], agent_type="lab-driver")])
        self.assertEqual(packets.join_campaigns(_ledger(), only_n1, _github(), CONFIG)["sessions"]["mapped"], 1)

    def test_branch_hint_is_the_branch_observed_by_the_last_record(self):
        by_id = {a["agent_id"]: a for a in self.db["agents"]}
        self.assertEqual(by_id["b-1"]["branch_hint"], "lab-record/LR-09-old-slug")
        self.assertEqual(by_id["b-2"]["branch_hint"], "lab-record/LR-03-new-slug")


class CampaignClaimTest(unittest.TestCase):
    """Two campaigns with the same packet ID: a session belongs to the campaign whose window opened latest before it."""

    CAMPAIGNS = {
        "alpha": {"packets": [_row("LX-01")], "window": {"start": "2026-10-01T00:00:00Z", "end": None}},
        "beta": {"packets": [_row("LX-01")], "window": {"start": "2026-10-05T00:00:00Z", "end": None}}}

    def test_session_after_the_later_window_start_maps_only_to_that_campaign(self):
        late = _agent("late", WORKTREE.format("lx-01"), "2026-10-06T06:00:00Z")
        self.assertEqual(packets._claim(late, self.CAMPAIGNS), ("beta", "LX-01"))

    def test_session_between_the_window_starts_maps_to_the_earlier_campaign(self):
        early = _agent("early", WORKTREE.format("lx-01"), "2026-10-02T06:00:00Z")
        self.assertEqual(packets._claim(early, self.CAMPAIGNS), ("alpha", "LX-01"))

    def test_session_before_every_window_maps_to_none(self):
        before = _agent("before", WORKTREE.format("lx-01"), "2026-09-30T06:00:00Z")
        self.assertIsNone(packets._claim(before, self.CAMPAIGNS))

    def test_branch_hint_claims_its_campaign_whatever_the_window(self):
        early = dict(_agent("early", None, "2026-10-02T06:00:00Z"), branch_hint="beta/LX-01-slug")
        self.assertEqual(packets._claim(early, self.CAMPAIGNS), ("beta", "LX-01"))

    def test_each_session_is_mapped_once(self):
        agents = [_agent("early", WORKTREE.format("lx-01"), "2026-10-02T06:00:00Z", "lab-driver", "lab-driver"),
                  _agent("late", WORKTREE.format("lx-01"), "2026-10-06T06:00:00Z", "lab-driver", "lab-driver"),
                  _agent("before", WORKTREE.format("lx-01"), "2026-09-30T06:00:00Z", "lab-driver", "lab-driver")]
        ledger_doc = {"campaigns": self.CAMPAIGNS}
        result = packets.join_campaigns(ledger_doc, {"agents": agents, "stops": {}}, {"prs": []}, CONFIG)
        self.assertEqual((result["sessions"]["total"], result["sessions"]["mapped"], result["sessions"]["unmapped"]),
                         (3, 2, 1))


def _record(result, packet_id):
    """The record of `packet_id` in the fixture campaign of a join result."""
    return next(r for r in result["campaigns"][CAMPAIGN]["packets"] if r["id"] == packet_id)


if __name__ == "__main__":
    unittest.main()
