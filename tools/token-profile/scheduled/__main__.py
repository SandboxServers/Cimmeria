"""Run as `python tools/token-profile/scheduled ...`; see jobs.py."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from scheduled.jobs import main  # noqa: E402

sys.exit(main())
