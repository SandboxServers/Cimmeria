"""The registered families, in run order.

AB-02 ships ``heal``, AB-03 ``damage``, AB-04 ``stat``, AB-10 ``shield`` and
``cleanse``: one module each beside ``heal.py``, one entry here, and the
family's range in ``NVP_RANGES``. The ranges are fixed by the ledger
(docs/analysis/ability-mechanics/work-packets.md, "Contract"); ``cleanse``
takes the next free range, 24500-24999.
"""

from __future__ import annotations

from typing import Dict, Tuple

from family import Family
from families.cleanse import CleanseFamily
from families.damage import DamageFamily
from families.heal import HealFamily
from families.shield import ShieldFamily
from families.stat import StatFamily

NVP_RANGES: Dict[str, Tuple[int, int]] = {
    "heal": (20000, 20999),
    "damage": (21000, 22999),
    "stat": (23000, 23999),
    "shield": (24000, 24499),
    "cleanse": (24500, 24999),
}

FAMILIES: Dict[str, Family] = {
    f.name: f for f in (HealFamily(), DamageFamily(), StatFamily(), ShieldFamily(), CleanseFamily())
}
