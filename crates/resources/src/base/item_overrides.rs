//! Cimmeria-side overrides for `CookedDataItems.pak` entries.
//!
//! The client's per-item display data — `IconLocation`, `Name`,
//! `MaxStackSize`, etc. — lives in the `_<itemId>` entries inside
//! `CookedDataItems.pak` on disk, not in any wire message the server
//! sends. The `InvItem` wire payload only carries instance state
//! (`stack_size`, `slot_id`, ammo, charges); the client looks up
//! display + stacking caps from its own cooked catalogue keyed by
//! `dbid`. So changing a row in `resources.items` server-side has
//! zero in-game effect for the icon or the client's stack cap — the
//! client never asks.
//!
//! Same trick as [`super::mission_overrides`]: patch the XML in
//! memory at server startup and lean on the cooked-data wire path
//! (`versionInfoRequest` → `onVersionInfo(InvalidKeys=[...])` →
//! `resourceFragment(_key, patched XML)`) to push the patched entry
//! to the client. Self-healing — runtime cache invalidation, no
//! manual client steps.
//!
//! This module produces patched XML bytes for items whose canonical
//! cooked entry shipped with the wrong icon or a stack cap that
//! doesn't match Cimmeria's chosen value. The XML shape follows the
//! Server-Build conventions documented in
//! [`docs/engine/cooked-data-pak-format.md`]: alphabetized attributes
//! on `<COOKED_ITEM>`, `MaxStackSize` attribute on the nested
//! `<InventorySet>` element.

/// One item's override: which item to patch and which display
/// attributes to rewrite. Either / both of `new_icon_location` and
/// `new_max_stack_size` may be `Some` — the patcher applies only
/// the populated fields and leaves the rest of the XML intact.
///
/// `item_id` is the cooked catalogue key (`_<item_id>` in the PAK),
/// which is the same as `resources.items.item_id` and the wire
/// `dbid` carried by `InvItem`.
pub struct ItemOverride {
    pub item_id: u32,
    /// Replacement value for the `IconLocation="..."` attribute on
    /// the `<COOKED_ITEM>` root. Use a `set:<atlas> image:<sprite>`
    /// string the cooked client GFX atlas actually contains; the
    /// server can't add new sprites, only re-point at ones the
    /// client already has baked.
    pub new_icon_location: Option<&'static str>,
    /// Replacement value for the `MaxStackSize="..."` attribute on
    /// the nested `<InventorySet>` element. Must match the
    /// server-side cap in `resources.items.max_stack_size` and the
    /// stacking logic in `handle_grant_item` — drift here surfaces
    /// as either lost pickups (server stacks past the client's cap)
    /// or wasted slots (client never stacks above the server cap).
    pub new_max_stack_size: Option<u32>,
}

/// All Cimmeria-introduced item overrides for the `CookedDataItems`
/// category. Adding an entry here:
///
///   1. Update the matching row in
///      `db/resources/Items/Seed/items.sql` so the server-side
///      catalogue agrees (max_stack_size is read by the
///      grant-stacking path; icon_location is documentary only).
///   2. Add an `ItemOverride` here so the client's UI picks up the
///      new display values via the per-key invalidation handshake.
///   3. If the override's `new_icon_location` points at a sprite
///      the client doesn't have baked, the inventory window
///      renders the placeholder "missing icon" square — verify
///      another item already uses the same string before adding a
///      new one.
///
/// Server seed and client override must agree on `max_stack_size`
/// — the in-game stacking experience requires both the server
/// (which decides whether to merge into an existing slot on
/// pickup) and the client (which enforces a per-display cap) to
/// see the same value.
pub const ITEM_OVERRIDES: &[ItemOverride] = &[
    // Health Slappack TC1 (item 2893). The canonical PAK shipped
    // with `IconLocation="set:CoreWidgets image:IconMissing"` (the
    // placeholder broken-square icon) and `MaxStackSize="1"` —
    // every looted slappack ate its own bag slot and rendered as
    // the missing-icon sprite. Player-reported regression after
    // PR #399's seed-only change had no visible effect.
    //
    // Medkit sprite is already used by Plasma Burn Treatment Kit
    // and Evac-U-Pac, so we know it's in the client's
    // ItemIcon001.upk atlas — no client-side patch needed for the
    // sprite itself, only this redirect.
    ItemOverride {
        item_id: 2893,
        new_icon_location: Some("set:ItemIcon001 image:Medkit"),
        new_max_stack_size: Some(10),
    },
    // TC-18 craftable duplicate (same display name). The inventory
    // UI keys icons + stack caps by item_id, not by name, so the
    // craftable variant has to be patched independently or it
    // would still render with the broken icon and refuse to stack.
    ItemOverride {
        item_id: 4735,
        new_icon_location: Some("set:ItemIcon001 image:Medkit"),
        new_max_stack_size: Some(10),
    },
];

/// Apply a single item override to a chunk of XML bytes from the
/// PAK.
///
/// Replaces:
/// - `IconLocation="<old>"` on `<COOKED_ITEM>` with the new value,
///   if `ov.new_icon_location` is `Some`.
/// - `MaxStackSize="<old>"` on the nested `<InventorySet>` with
///   the new value, if `ov.new_max_stack_size` is `Some`.
///
/// Returns `Some(patched)` on success, `None` if any requested
/// patch couldn't find its anchor — caller logs and keeps the
/// original bytes unmodified rather than risk shipping a partially
/// patched entry that disagrees with the server-side seed.
///
/// Anchor strategy: substring scan for `IconLocation="` and
/// `MaxStackSize="`. The Server-Build attribute order is
/// alphabetical (verified in `docs/engine/cooked-data-pak-format.md`)
/// and the cooked items each have a single `IconLocation` /
/// single `<InventorySet MaxStackSize=...>` per item, so unbounded
/// `str::find` is safe: the next match after the start of the
/// element IS the attribute on the right element.
pub fn apply_override(original: &[u8], ov: &ItemOverride) -> Option<Vec<u8>> {
    let xml = std::str::from_utf8(original).ok()?;
    let mut current = xml.to_string();

    if let Some(new_icon) = ov.new_icon_location {
        current = patch_attr(&current, "IconLocation", new_icon)?;
    }
    if let Some(new_stack) = ov.new_max_stack_size {
        let new_value = new_stack.to_string();
        current = patch_attr(&current, "MaxStackSize", &new_value)?;
    }
    Some(current.into_bytes())
}

/// Replace the value of `<attr_name>="<old>"` with
/// `<attr_name>="<new_value>"` in `xml`. Returns `None` if the
/// attribute isn't found — caller decides whether that's a hard
/// failure or a benign skip.
///
/// Matches only the FIRST occurrence. The cooked item XMLs have a
/// single `IconLocation` and a single `MaxStackSize` per entry, so
/// "first" == "only" in practice. If a future override needs to
/// patch multiple attributes of the same name, this helper grows
/// to take an anchor.
fn patch_attr(xml: &str, attr_name: &str, new_value: &str) -> Option<String> {
    let attr_open = format!("{attr_name}=\"");
    let attr_start = xml.find(&attr_open)?;
    let value_start = attr_start + attr_open.len();
    let value_end = value_start + xml[value_start..].find('"')?;

    let mut out = String::with_capacity(xml.len() - (value_end - value_start) + new_value.len());
    out.push_str(&xml[..value_start]);
    out.push_str(new_value);
    out.push_str(&xml[value_end..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Realistic 2893 (slappack) Server-Build shape, lifted from the
    /// canonical XML conventions in
    /// `docs/engine/cooked-data-pak-format.md`. Anchors the pre-patch
    /// state so a regression that changes the attribute order or
    /// drops an `<InventorySet>` element trips a clear failure here
    /// instead of a subtle in-game miss.
    const SAMPLE_2893: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
        <COOKED_ITEM AppliedScienceID=\"1\" Description=\"Health Slappack TC1 - +500 Health\" \
        ID=\"2893\" IconLocation=\"set:CoreWidgets image:IconMissing\" \
        IsElementaryComponent=\"false\" IsKicker=\"false\" IsResearchable=\"false\" \
        IsReverseEngineerable=\"false\" Name=\"Health Slappack TC1\" QualityID=\"2000\" \
        TechComp=\"1\" Tier=\"1\" ItemFlags=\"36224\">\
        <InventorySet IsDeletable=\"true\" IsSellable=\"true\" MaxStackSize=\"1\" />\
        <RequirementsSet IsUnique=\"false\" />\
        <ItemEventSet AbilityID=\"5001\" EventID=\"5\" />\
        <ContainerSet>1</ContainerSet>\
        </COOKED_ITEM>";

    /// Patcher must rewrite both attributes in one pass and leave
    /// the rest of the XML byte-identical. Negative pin: `Name=`,
    /// `Description=`, the nested `<ItemEventSet>` and
    /// `<ContainerSet>` survive verbatim. Bug shape: a regression
    /// that uses a too-greedy regex (e.g. `MaxStackSize="[^"]*"`
    /// applied across the whole document) could chew through other
    /// attributes if a future cooked entry gains a second
    /// `MaxStackSize` reference (vendor offer XML, etc.).
    #[test]
    fn apply_override_rewrites_icon_and_stack_size_in_place() {
        let ov = ItemOverride {
            item_id: 2893,
            new_icon_location: Some("set:ItemIcon001 image:Medkit"),
            new_max_stack_size: Some(10),
        };
        let patched = apply_override(SAMPLE_2893, &ov).expect("override must apply");
        let patched_str = std::str::from_utf8(&patched).expect("patched is utf-8");

        assert!(
            patched_str.contains("IconLocation=\"set:ItemIcon001 image:Medkit\""),
            "IconLocation must be rewritten to the Medkit sprite"
        );
        assert!(
            patched_str.contains("MaxStackSize=\"10\""),
            "MaxStackSize must be rewritten to 10"
        );
        // Negative pins — surrounding XML unchanged.
        assert!(
            !patched_str.contains("IconMissing"),
            "old placeholder icon must be removed entirely"
        );
        assert!(
            patched_str.contains("Name=\"Health Slappack TC1\""),
            "Name attribute must survive verbatim"
        );
        assert!(
            patched_str.contains("<ItemEventSet AbilityID=\"5001\" EventID=\"5\" />"),
            "nested ItemEventSet must survive verbatim"
        );
        assert!(
            patched_str.contains("ID=\"2893\""),
            "item ID attribute must survive verbatim — patcher must not chew through it"
        );
    }

    /// Icon-only override leaves MaxStackSize alone. Pins that the
    /// `Option` discipline actually gates the patch — a regression
    /// that always rewrites MaxStackSize (regardless of `Some/None`)
    /// would trip this.
    #[test]
    fn apply_override_with_only_icon_leaves_stack_size_untouched() {
        let ov = ItemOverride {
            item_id: 2893,
            new_icon_location: Some("set:ItemIcon001 image:Spray_Injector"),
            new_max_stack_size: None,
        };
        let patched = apply_override(SAMPLE_2893, &ov).expect("override must apply");
        let patched_str = std::str::from_utf8(&patched).expect("patched is utf-8");

        assert!(patched_str.contains("IconLocation=\"set:ItemIcon001 image:Spray_Injector\""));
        assert!(
            patched_str.contains("MaxStackSize=\"1\""),
            "MaxStackSize must be unchanged when new_max_stack_size is None"
        );
    }

    /// Stack-only override leaves IconLocation alone. Companion
    /// to the icon-only case.
    #[test]
    fn apply_override_with_only_stack_leaves_icon_untouched() {
        let ov = ItemOverride {
            item_id: 2893,
            new_icon_location: None,
            new_max_stack_size: Some(5),
        };
        let patched = apply_override(SAMPLE_2893, &ov).expect("override must apply");
        let patched_str = std::str::from_utf8(&patched).expect("patched is utf-8");

        assert!(patched_str.contains("MaxStackSize=\"5\""));
        assert!(
            patched_str.contains("IconLocation=\"set:CoreWidgets image:IconMissing\""),
            "IconLocation must be unchanged when new_icon_location is None"
        );
    }

    /// Missing anchor → None. Defensive — if the cooked XML shape
    /// drifts (attribute renamed, element removed), we refuse to
    /// ship a half-patched entry. Caller logs the skip and keeps
    /// the original bytes intact.
    #[test]
    fn apply_override_returns_none_when_anchor_missing() {
        let no_icon = b"<COOKED_ITEM ID=\"2893\" Name=\"Foo\"><InventorySet MaxStackSize=\"1\"/></COOKED_ITEM>";
        let ov = ItemOverride {
            item_id: 2893,
            new_icon_location: Some("set:ItemIcon001 image:Medkit"),
            new_max_stack_size: Some(10),
        };
        assert!(
            apply_override(no_icon, &ov).is_none(),
            "missing IconLocation anchor must refuse the patch entirely"
        );
    }

    /// Every override entry in `ITEM_OVERRIDES` applies cleanly
    /// against a fixture shaped like the canonical PAK entry.
    /// A new override added to the registry trips here if its
    /// `new_*` fields are populated but the patcher can't find
    /// the anchor on a typical COOKED_ITEM shape.
    #[test]
    fn all_registered_overrides_apply_to_sample() {
        for ov in ITEM_OVERRIDES {
            let patched = apply_override(SAMPLE_2893, ov)
                .unwrap_or_else(|| panic!("override for item {} must apply", ov.item_id));
            let patched_str = std::str::from_utf8(&patched).expect("patched is utf-8");
            if let Some(want_icon) = ov.new_icon_location {
                assert!(
                    patched_str.contains(&format!("IconLocation=\"{want_icon}\"")),
                    "override for item {} must land its icon",
                    ov.item_id,
                );
            }
            if let Some(want_stack) = ov.new_max_stack_size {
                assert!(
                    patched_str.contains(&format!("MaxStackSize=\"{want_stack}\"")),
                    "override for item {} must land its stack size",
                    ov.item_id,
                );
            }
        }
    }
}
