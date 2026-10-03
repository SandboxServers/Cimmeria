"""Run as `python tools/token-profile/report ...`; see cli.py."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from report.cli import main  # noqa: E402

sys.exit(main())
