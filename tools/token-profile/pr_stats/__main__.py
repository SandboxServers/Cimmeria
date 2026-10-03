"""Run as `python tools/token-profile/pr_stats ...`; see cli.py."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from pr_stats.cli import main  # noqa: E402

sys.exit(main())
