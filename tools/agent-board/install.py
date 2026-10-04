#!/usr/bin/env python3
"""Install the agent-board CLI and MCP server for this machine (once per operator).

    python tools/agent-board/install.py --operator <steven|derek> [--connect IP] [--no-mcp]

What it does:
  * copies board.py to ~/.agent-board/ and writes a `board` shell shim beside it
  * records your operator name (whose agent accounts you use)
  * registers the `agent-board` MCP server at Claude Code user scope, so every
    board project repo gets it without a committed .mcp.json

No API key is written anywhere: `board` reads it from Key Vault on each run.
Re-run after pulling a newer board.py.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import stat
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import board  # noqa: E402

SHIM = """#!/bin/sh
# agent-board CLI shim (installed by tools/agent-board/install.py)
for py in python3 python; do
  if command -v "$py" >/dev/null 2>&1 && "$py" -c "import sys; sys.exit(sys.version_info < (3, 9))" 2>/dev/null; then
    exec "$py" "$(dirname "$0")/board.py" "$@"
  fi
done
echo "board: Python 3.9+ not found" >&2
exit 1
"""


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--operator", choices=sorted(board.OPERATORS),
                    help="whose agent accounts this machine uses; required on first install")
    ap.add_argument("--connect", metavar="IP", help="connect to this address instead of public DNS "
                                                    "(e.g. the board host's LAN address); '' clears it")
    ap.add_argument("--no-mcp", action="store_true", help="do not register the MCP server")
    args = ap.parse_args()

    home = board.CONFIG_DIR
    home.mkdir(parents=True, exist_ok=True)
    shutil.copy2(HERE / "board.py", home / "board.py")
    shim = home / "board"
    shim.write_text(SHIM, encoding="utf-8", newline="\n")
    shim.chmod(shim.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

    cfg_path = home / "config.json"
    cfg = board.load_config()
    cfg["operator"] = args.operator or cfg.get("operator")
    if not cfg["operator"]:
        raise SystemExit("first install needs --operator (" + " or ".join(sorted(board.OPERATORS)) + ")")
    if args.connect is not None:
        cfg["connect_address"] = args.connect or None
    cfg_path.write_text(json.dumps(cfg, indent=2), encoding="utf-8")
    print(f"installed {home / 'board.py'} for operator {cfg['operator']}"
          + (f", connecting via {cfg['connect_address']}" if cfg.get("connect_address") else ""))

    if not args.no_mcp:
        claude = shutil.which("claude")
        if not claude:
            print("claude CLI not found; skipped MCP registration")
        else:
            subprocess.run([claude, "mcp", "remove", "--scope", "user", "agent-board"],
                           capture_output=True, text=True)
            r = subprocess.run([claude, "mcp", "add", "--scope", "user", "agent-board", "--",
                                sys.executable, str(home / "board.py"), "mcp"], capture_output=True, text=True)
            print("registered MCP server agent-board (user scope)" if r.returncode == 0
                  else f"MCP registration failed: {r.stderr.strip()}")

    print(f"Add {home} to PATH, or call {shim} directly. Try: board whoami")


if __name__ == "__main__":
    main()
