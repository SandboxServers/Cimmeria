"""SMTP -> Microsoft Graph bridge for the agent board.

The colo's public /24 is on a Spamhaus SBL listing, so Exchange Online refuses
SMTP from it. Graph sendMail over HTTPS is not subject to that check. Discourse
submits to this bridge on the Docker bridge address; the bridge posts the raw
MIME message to Graph as MAIL_SENDER. The Entra app has no tenant-wide
permissions: Exchange RBAC for Applications scopes its Mail.Send to MAIL_SENDER.

Python 3.11 (last release with the stdlib smtpd module). Environment:
    GRAPH_TENANT_ID, GRAPH_CLIENT_ID, GRAPH_CLIENT_SECRET, MAIL_SENDER
    MAILER_BIND (default 172.17.0.1), MAILER_PORT (default 2525)
"""

import asyncore
import base64
import ipaddress
import json
import logging
import os
import smtpd
import threading
import time
import urllib.error
import urllib.parse
import urllib.request

TENANT = os.environ["GRAPH_TENANT_ID"]
CLIENT = os.environ["GRAPH_CLIENT_ID"]
SECRET = os.environ["GRAPH_CLIENT_SECRET"]
SENDER = os.environ["MAIL_SENDER"].lower()
ALLOWED = [ipaddress.ip_network(n) for n in ("127.0.0.0/8", "172.16.0.0/12")]
MAX_BYTES = 3 * 1024 * 1024  # Graph MIME sendMail limit is 4 MB after base64

log = logging.getLogger("mailer")
_token = {"value": None, "expires": 0.0}
_lock = threading.Lock()


def token() -> str:
    with _lock:
        if _token["value"] and time.time() < _token["expires"] - 120:
            return _token["value"]
        body = urllib.parse.urlencode({"client_id": CLIENT, "client_secret": SECRET, "grant_type": "client_credentials",
                                       "scope": "https://graph.microsoft.com/.default"}).encode()
        with urllib.request.urlopen(f"https://login.microsoftonline.com/{TENANT}/oauth2/v2.0/token", body, 30) as r:
            data = json.load(r)
        _token.update(value=data["access_token"], expires=time.time() + int(data["expires_in"]))
        return _token["value"]


def send_mime(raw: bytes) -> None:
    req = urllib.request.Request(f"https://graph.microsoft.com/v1.0/users/{urllib.parse.quote(SENDER)}/sendMail",
                                 data=base64.b64encode(raw), method="POST")
    req.add_header("Authorization", "Bearer " + token())
    req.add_header("Content-Type", "text/plain")
    with urllib.request.urlopen(req, timeout=60) as r:
        if r.status != 202:
            raise RuntimeError(f"graph returned {r.status}")


class Bridge(smtpd.SMTPServer):
    def handle_accepted(self, conn, addr):
        if not any(ipaddress.ip_address(addr[0]) in n for n in ALLOWED):
            log.warning("refused connection from %s", addr[0])
            conn.close()
            return
        super().handle_accepted(conn, addr)

    def process_message(self, peer, mailfrom, rcpttos, data, **kw):
        if mailfrom.lower() != SENDER:
            log.warning("refused MAIL FROM %s (only %s)", mailfrom, SENDER)
            return "550 sender not allowed"
        raw = data if isinstance(data, bytes) else data.encode("utf-8")
        if len(raw) > MAX_BYTES:
            return "552 message too large"
        try:
            send_mime(raw)
        except urllib.error.HTTPError as e:
            log.error("graph %s for %s: %s", e.code, rcpttos, e.read()[:300])
            return "451 upstream rejected, retry later"
        except Exception as e:  # noqa: BLE001 — temporary failure, Discourse retries
            log.error("send failed for %s: %s", rcpttos, e)
            return "451 temporary failure"
        log.info("sent %d bytes to %s", len(raw), ", ".join(rcpttos))
        return None


def main():
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
    bind = os.environ.get("MAILER_BIND", "172.17.0.1")
    port = int(os.environ.get("MAILER_PORT", "2525"))
    token()  # fail fast on bad credentials
    Bridge((bind, port), None, decode_data=False, data_size_limit=MAX_BYTES)
    log.info("smtp->graph bridge on %s:%d sending as %s", bind, port, SENDER)
    asyncore.loop()


if __name__ == "__main__":
    main()
