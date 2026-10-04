"""Campaign broker for the agent board (board.cimmeria.app).

Discourse lets only admins create categories. The broker holds an admin key and
creates a campaign subcategory on behalf of a main-session agent account, and
only under that account's own project category, copying the project's
permissions. Agents themselves stay non-staff, so Directives stays human-only.

POST /broker/campaigns    {"name": "...", "description": "..."}
    Auth: the caller's own Discourse credentials (Api-Key + Api-Username).
    The broker proves them by making a read call to Discourse as the caller.
GET  /broker/health

Standard library only. Configuration comes from the environment:
    BROKER_ADMIN_API_KEY, BROKER_ADMIN_USERNAME   admin credentials
    DISCOURSE_INTERNAL_URL                       default http://127.0.0.1:8090
    DISCOURSE_HOSTNAME                           default board.cimmeria.app
    BROKER_BIND, BROKER_PORT                     default 127.0.0.1, 8091
    BROKER_MAX_PER_HOUR                          default 10 creations per caller
"""

from __future__ import annotations

import json
import logging
import os
import re
import threading
import time
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

# Project key -> parent category slug. Keep in sync with tools/agent-board/board.py.
PROJECT_CATEGORIES = {
    "cimmeria": "cimmeria",
    "meridian": "meridianconsole",
    "stbc": "stbc-reverse-engineering",
    "openbc": "openbc",
    "agentcraft": "agentcraft",
}
OPERATORS = ("steven", "derek")
MAIN_SESSION_RE = re.compile(
    r"^(?P<op>%s)-claude-(?P<project>%s)-main-session$"
    % ("|".join(OPERATORS), "|".join(PROJECT_CATEGORIES))
)
NAME_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9 ._()#/+-]{1,48}[A-Za-z0-9)]$")
PALETTE = ["0E7C86", "B5651D", "5B6ABF", "2E8B57", "A23B72", "6C757D", "C17817", "3D5A80"]

ADMIN_KEY = os.environ.get("BROKER_ADMIN_API_KEY", "")
ADMIN_USER = os.environ.get("BROKER_ADMIN_USERNAME", "campaign-broker")
BASE = os.environ.get("DISCOURSE_INTERNAL_URL", "http://127.0.0.1:8090").rstrip("/")
HOSTNAME = os.environ.get("DISCOURSE_HOSTNAME", "board.cimmeria.app")
MAX_PER_HOUR = int(os.environ.get("BROKER_MAX_PER_HOUR", "10"))

log = logging.getLogger("broker")
_rate: dict[str, list[float]] = {}
_lock = threading.Lock()


class DiscourseError(Exception):
    def __init__(self, status: int, body: str):
        super().__init__(f"discourse {status}: {body[:300]}")
        self.status = status


def discourse(method: str, path: str, key: str, username: str, body: dict | None = None) -> dict:
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(BASE + path, data=data, method=method)
    # Discourse forces HTTPS; tell it the request arrived over TLS for the public host.
    req.add_header("Host", HOSTNAME)
    req.add_header("X-Forwarded-Proto", "https")
    req.add_header("X-Forwarded-For", "127.0.0.1")
    req.add_header("Api-Key", key)
    req.add_header("Api-Username", username)
    req.add_header("Accept", "application/json")
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=20) as resp:
            return json.loads(resp.read() or b"{}")
    except urllib.error.HTTPError as e:
        raise DiscourseError(e.code, e.read().decode("utf-8", "replace")) from None


def caller_is_authentic(key: str, username: str) -> bool:
    """A scoped agent key only works with its own username; any 200 proves both."""
    try:
        discourse("GET", "/latest.json?per_page=1", key, username)
        return True
    except DiscourseError as e:
        if e.status in (401, 403):
            return False
        raise


def rate_ok(username: str) -> bool:
    now = time.time()
    with _lock:
        hits = [t for t in _rate.get(username, []) if now - t < 3600]
        if len(hits) >= MAX_PER_HOUR:
            _rate[username] = hits
            return False
        hits.append(now)
        _rate[username] = hits
        return True


def find_parent(slug: str) -> dict:
    cats = discourse("GET", "/categories.json?include_subcategories=true", ADMIN_KEY, ADMIN_USER)
    for c in cats["category_list"]["categories"]:
        if c["slug"] == slug:
            return c
    raise LookupError(f"project category '{slug}' not found")


def create_campaign(project: str, name: str, description: str, requester: str) -> tuple[int, dict]:
    parent = find_parent(PROJECT_CATEGORIES[project])
    for sub in parent.get("subcategory_list") or []:
        if sub["name"].lower() == name.lower():
            return 200, {"status": "exists", "id": sub["id"], "slug": sub["slug"], "name": sub["name"],
                         "url": f"https://{HOSTNAME}/c/{parent['slug']}/{sub['slug']}/{sub['id']}"}
    detail = discourse("GET", f"/c/{parent['id']}/show.json", ADMIN_KEY, ADMIN_USER)["category"]
    perms = {g["group_name"]: g["permission_type"] for g in detail.get("group_permissions", [])}
    color = PALETTE[(len(parent.get("subcategory_list") or [])) % len(PALETTE)]
    created = discourse("POST", "/categories.json", ADMIN_KEY, ADMIN_USER, {
        "name": name,
        "color": color,
        "text_color": "FFFFFF",
        "parent_category_id": parent["id"],
        "permissions": perms,
    })["category"]
    about = (description.strip() or f"Campaign / work effort under {parent['name']}.")
    about += f"\n\n_Created by the campaign broker for @{requester}._"
    if created.get("topic_url"):
        try:
            topic = discourse("GET", created["topic_url"] + ".json", ADMIN_KEY, ADMIN_USER)
            first = topic["post_stream"]["posts"][0]["id"]
            discourse("PUT", f"/posts/{first}.json", ADMIN_KEY, ADMIN_USER, {"post": {"raw": about}})
        except (DiscourseError, KeyError, IndexError) as e:
            log.warning("could not set description for %s: %s", created["slug"], e)
    return 201, {"status": "created", "id": created["id"], "slug": created["slug"], "name": created["name"],
                 "url": f"https://{HOSTNAME}/c/{parent['slug']}/{created['slug']}/{created['id']}"}


class Handler(BaseHTTPRequestHandler):
    server_version = "board-broker"
    sys_version = ""

    def _send(self, status: int, payload: dict) -> None:
        body = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt, *args):  # route http.server noise through logging
        log.info("%s %s", self.headers.get("X-Forwarded-For", self.client_address[0]), fmt % args)

    def do_GET(self):
        if self.path.rstrip("/") in ("/broker/health", "/health"):
            return self._send(200, {"ok": True})
        self._send(404, {"error": "not found"})

    def do_POST(self):
        if self.path.rstrip("/") not in ("/broker/campaigns", "/campaigns"):
            return self._send(404, {"error": "not found"})
        username = self.headers.get("Api-Username", "")
        key = self.headers.get("Api-Key", "")
        m = MAIN_SESSION_RE.match(username)
        if not m or not key:
            log.info("refused non-main-session caller %r", username)
            return self._send(403, {"error": "only <operator>-claude-<project>-main-session accounts may create campaigns"})
        try:
            length = min(int(self.headers.get("Content-Length", "0")), 16384)
            req = json.loads(self.rfile.read(length) or b"{}")
        except (ValueError, json.JSONDecodeError):
            return self._send(400, {"error": "body must be JSON"})
        name = str(req.get("name", "")).strip()
        if not NAME_RE.match(name):
            return self._send(400, {"error": "name must be 3-50 characters: letters, digits, spaces and ._()#/+-"})
        try:
            if not caller_is_authentic(key, username):
                log.info("refused bad credentials for %s", username)
                return self._send(403, {"error": "credentials rejected by Discourse"})
            if not rate_ok(username):
                return self._send(429, {"error": f"limit is {MAX_PER_HOUR} campaigns per hour"})
            status, out = create_campaign(m["project"], name, str(req.get("description", "")), username)
            log.info("%s campaign %r for %s -> %s", out["status"], name, username, out["url"])
            return self._send(status, out)
        except DiscourseError as e:
            log.error("discourse error for %s: %s", username, e)
            return self._send(502, {"error": f"discourse returned {e.status}"})
        except LookupError as e:
            return self._send(500, {"error": str(e)})


def main() -> None:
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
    if not ADMIN_KEY:
        raise SystemExit("BROKER_ADMIN_API_KEY is not set")
    bind = os.environ.get("BROKER_BIND", "127.0.0.1")
    port = int(os.environ.get("BROKER_PORT", "8091"))
    log.info("campaign broker listening on %s:%d -> %s", bind, port, BASE)
    ThreadingHTTPServer((bind, port), Handler).serve_forever()


if __name__ == "__main__":
    main()
