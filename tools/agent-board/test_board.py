"""Unit tests for the agent-board CLI and campaign broker (no network)."""

import os
import sys
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE / "broker"))

import board  # noqa: E402
import broker  # noqa: E402


def tree():
    project = {"id": 6, "slug": "cimmeria", "name": "Cimmeria"}
    campaign = {"id": 14, "slug": "agent-board-rollout", "name": "Agent Board Rollout", "_parent_slug": "cimmeria"}
    other = {"id": 9, "slug": "openbc", "name": "OpenBC"}
    other_campaign = {"id": 15, "slug": "renderer", "name": "Renderer", "_parent_slug": "openbc"}
    return {
        "cimmeria": project, "cimmeria/agent-board-rollout": campaign,
        "openbc": other, "openbc/renderer": other_campaign,
        "questions": {"id": 12, "slug": "questions"}, "handoffs": {"id": 11, "slug": "handoffs"},
        "directives": {"id": 5, "slug": "directives"}, "decisions-log": {"id": 13, "slug": "decisions-log"},
    }


def ident(agent="main-session", operator="steven", project="cimmeria"):
    env = {"AGENT_BOARD_OPERATOR": operator, "AGENT_BOARD_PROJECT": project}
    with mock.patch.dict(os.environ, env):
        return board.Identity(agent)


class IdentityTests(unittest.TestCase):
    def test_project_from_https_and_ssh_remotes(self):
        self.assertEqual(board.project_from_remote("https://github.com/SandboxServers/Cimmeria.git"), "cimmeria")
        self.assertEqual(board.project_from_remote("git@github.com:SandboxServers/OpenBC.git"), "openbc")
        self.assertEqual(board.project_from_remote("https://github.com/SandboxServers/STBC-Reverse-Engineering.git"),
                         "stbc")
        self.assertEqual(board.project_from_remote("https://github.com/SandboxServers/MeridianConsole"), "meridian")
        self.assertIsNone(board.project_from_remote("https://github.com/someone/unrelated.git"))

    def test_account_and_secret_names_match_the_provisioned_layout(self):
        i = ident("combat-systems-advisor")
        self.assertEqual(i.username, "steven-claude-cimmeria-combat-systems-advisor")
        self.assertEqual(i.secret, "discourse-agent-cimmeria-combat-systems-advisor")
        d = ident("main-session", operator="derek", project="openbc")
        self.assertEqual(d.username, "derek-claude-openbc-main-session")
        self.assertEqual(d.secret, "discourse-agent-derek-openbc-main-session")

    def test_rejects_unknown_operator_and_bad_agent_names(self):
        with self.assertRaises(board.BoardError):
            ident(operator="mallory")
        with self.assertRaises(board.BoardError):
            ident(agent="../admin")


class CategoryRuleTests(unittest.TestCase):
    def test_default_is_the_project_category(self):
        self.assertEqual(board.resolve_post_category(ident(), tree(), None)["id"], 6)

    def test_campaign_by_bare_or_qualified_slug(self):
        self.assertEqual(board.resolve_post_category(ident(), tree(), "agent-board-rollout")["id"], 14)
        self.assertEqual(board.resolve_post_category(ident(), tree(), "cimmeria/agent-board-rollout")["id"], 14)

    def test_shared_categories_allowed(self):
        self.assertEqual(board.resolve_post_category(ident(), tree(), "questions")["id"], 12)
        self.assertEqual(board.resolve_post_category(ident(), tree(), "handoffs")["id"], 11)

    def test_other_projects_and_human_categories_refused(self):
        for slug in ("openbc", "openbc/renderer", "directives", "decisions-log"):
            with self.assertRaises(board.BoardError, msg=slug):
                board.resolve_post_category(ident(), tree(), slug)

    def test_unknown_campaign_lists_existing_ones(self):
        with self.assertRaises(board.BoardError) as cm:
            board.resolve_post_category(ident(), tree(), "nope")
        self.assertIn("agent-board-rollout", str(cm.exception))

    def test_header_names_project_campaign_and_agent(self):
        h = board.header_line(ident("documentation-writer"), tree()["cimmeria/agent-board-rollout"])
        self.assertIn("project: cimmeria", h)
        self.assertIn("campaign: agent-board-rollout", h)
        self.assertIn("agent: documentation-writer (Steven)", h)


class HookTests(unittest.TestCase):
    def test_hook_swallows_every_failure(self):
        def boom():
            raise RuntimeError("no network")
        with mock.patch("sys.stdout") as out:
            board.cmd_hook(None, boom)
        out.write.assert_not_called()


class BrokerRuleTests(unittest.TestCase):
    def test_only_main_session_accounts_match(self):
        self.assertTrue(broker.MAIN_SESSION_RE.match("steven-claude-cimmeria-main-session"))
        self.assertTrue(broker.MAIN_SESSION_RE.match("derek-claude-openbc-main-session"))
        for name in ("steven-claude-cimmeria-combat-systems-advisor", "steven", "campaign-broker",
                     "mallory-claude-cimmeria-main-session", "steven-claude-unknown-main-session"):
            self.assertIsNone(broker.MAIN_SESSION_RE.match(name), name)

    def test_campaign_name_validation(self):
        for ok in ("Agent Board Rollout", "Harset W2", "AB-01 self-cast (v2)"):
            self.assertTrue(broker.NAME_RE.match(ok), ok)
        for bad in ("", "x", "<script>", "a" * 60, " leading"):
            self.assertIsNone(broker.NAME_RE.match(bad), bad)

    def test_project_maps_agree_between_cli_and_broker(self):
        self.assertEqual(board.PROJECT_CATEGORY, broker.PROJECT_CATEGORIES)
        self.assertEqual(tuple(board.OPERATORS), broker.OPERATORS)

    def test_rate_limit(self):
        broker._rate.clear()
        with mock.patch.object(broker, "MAX_PER_HOUR", 2):
            self.assertTrue(broker.rate_ok("u"))
            self.assertTrue(broker.rate_ok("u"))
            self.assertFalse(broker.rate_ok("u"))
            self.assertTrue(broker.rate_ok("other"))


if __name__ == "__main__":
    unittest.main()
