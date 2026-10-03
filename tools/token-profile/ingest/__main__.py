import sys
from pathlib import Path

# Run as a directory (`python tools/token-profile/ingest`): make the package importable.
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from ingest.cli import main  # noqa: E402

sys.exit(main())
