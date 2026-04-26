#!/usr/bin/env python3
import os
import re
from pathlib import Path

vendor_dir = Path("crates/services/src/base/world_entry_methods/vendor")

replacements = [
    (r'use super::super::', 'use super::super::super::'),
    (r'use super::inventory_core::', 'use super::super::inventory::core::'),
    (r'use super::inventory_grant::', 'use super::super::inventory::grant::'),
    (r'use super::inventory_move::', 'use super::super::inventory::move_::'),
    (r'use super::vendor_helpers::', 'use super::helpers::'),
    (r'use super::vendor_store::', 'use super::store::'),
    (r'use super::vendor_data::', 'use super::data::'),
    (r'use super::vendor_serializers::', 'use super::serializers::'),
    (r'use super::vendor_purchase_helpers::', 'use super::purchase_helpers::'),
    (r'use super::vendor_purchase::', 'use super::purchase::'),
    (r'use super::vendor_sell::', 'use super::sell::'),
    (r'use super::vendor_sell_helpers::', 'use super::sell_helpers::'),
    (r'use super::vendor_buyback::', 'use super::buyback::'),
    (r'use super::vendor_buyback_helpers::', 'use super::buyback_helpers::'),
    (r'use super::vendor_repair::', 'use super::repair::'),
    (r'use super::vendor_paid_repair::', 'use super::paid_repair::'),
    (r'use super::vendor_recharge::', 'use super::recharge::'),
    (r'use super::vendor_paid_recharge::', 'use super::paid_recharge::'),
    (r'use super::player_load_core::', 'use super::super::player_load::core::'),
    (r'use super::player_load_meta::', 'use super::super::player_load::meta::'),
    (r'use super::progression::', 'use super::super::progression::'),
]

for rs_file in vendor_dir.glob("*.rs"):
    if rs_file.name == "mod.rs":
        continue

    content = rs_file.read_text()
    original = content

    for old, new in replacements:
        content = content.replace(old, new)

    if content != original:
        rs_file.write_text(content)
        print(f"Updated {rs_file.name}")

print("Done!")
