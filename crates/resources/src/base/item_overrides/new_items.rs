//! Brand-new `CookedDataItems` entries: item ids the shipped PAK does not
//! contain at all.
//!
//! [`super::ITEM_OVERRIDES`] patches attributes of an entry the client
//! already has. An addition has nothing to patch, so it is generated whole,
//! the same way `dialog_overrides`, `sequence_overrides` and
//! `world_info_overrides` add dialogs, Kismet sequences and worlds the client
//! never shipped. `ResourceCache::apply_item_overrides` inserts each one into
//! the served category and folds it into the category's metadata bump, so a
//! client holding the shipped `CookedDataItems` sees a version mismatch and
//! the full-category resync (#840) streams the new entries with the rest.
//!
//! First user: the ammo campaign's 15 special-ammo reserve items, ids
//! 9000-9014 (AM-07, issue #1026), in `ammo_items.rs`.
//!
//! The XML is byte-for-byte the shape the shipped PAK uses for a plain
//! stackable item with no event set, moniker or discipline list (for
//! example `_10` "Gopher Head" and `_1086` "Airburst Rangefinder"). The
//! attribute order is the cooked tool's, not alphabetical; a unit test
//! regenerates both of those shipped entries and compares the bytes.

/// One item Cimmeria adds to `CookedDataItems`.
///
/// Only the attributes that vary between the items Cimmeria adds are fields.
/// The rest are fixed by [`generate_item_xml`] to the values almost every
/// shipped item carries: `IsReverseEngineerable="false"`,
/// `IsResearchable="false"`, `IsElementaryComponent="true"` (every one of the
/// 6059 shipped items), `IsKicker="false"`, `Tier="1"`,
/// `AppliedScienceID="0"`, `QualityID="2000"` (normal quality, 5907 of 6059),
/// `IsDeletable="true"` and `IsUnique="false"`.
#[derive(Debug)]
pub struct NewItem {
    /// The cooked key (`_<item_id>`), `resources.items.item_id` and the wire
    /// `dbid`. Must not already exist in the shipped PAK and must be at most
    /// `dialog_overrides::MAX_COOKED_ELEMENT_ID` (65535).
    pub item_id: u32,
    /// `Name`, shown in the bags and as the tooltip title. Must match
    /// `resources.items.name`.
    pub name: &'static str,
    /// `Description`, the tooltip body. Cooked-only: the server never sends
    /// `resources.items.description` to the client.
    pub description: &'static str,
    /// `IconLocation`, a `set:<imageset> image:<name>` pair. The imageset must
    /// be declared in the client's `TaharezLook.scheme`; the server can only
    /// point at sprites the client already has.
    pub icon_location: &'static str,
    /// `MaxStackSize` on `<InventorySet>`. Must match
    /// `resources.items.max_stack_size`, or the server and the client stack
    /// differently (see [`super::ItemOverride::new_max_stack_size`]).
    pub max_stack_size: u32,
    /// `TechComp`. Must match `resources.items.tech_comp`.
    pub tech_comp: u32,
    /// `IsSellable` on `<InventorySet>`: the item's `CanBeSold` flag.
    pub is_sellable: bool,
    /// One `<ContainerSet>` per id, in this order: the bags the client lets
    /// the item sit in. Must match `resources.items.container_sets`.
    pub container_sets: &'static [u32],
}

/// The fixed prefix of every shipped `COOKED_ITEM`: the XML declaration, one
/// `\n` (the only newline in a cooked item), and the five namespace
/// declarations the cooking tool wrote on every root element.
const COOKED_ITEM_PROLOGUE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
    <COOKED_ITEM xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" \
    xmlns:SOAP-ENC=\"http://schemas.xmlsoap.org/soap/encoding/\" \
    xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
    xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" xmlns:CookedData1=\"SGW\"";

/// Render one [`NewItem`] as the `COOKED_ITEM` XML the client parses.
///
/// Infallible: the whole entry is generated, so unlike
/// [`super::apply_override`] there is no anchor to miss.
pub fn generate_item_xml(item: &NewItem) -> Vec<u8> {
    let mut xml = String::with_capacity(1024);
    xml.push_str(COOKED_ITEM_PROLOGUE);
    xml.push_str(
        " IsReverseEngineerable=\"false\" IsResearchable=\"false\" \
         IsElementaryComponent=\"true\" IsKicker=\"false\"",
    );
    xml.push_str(&format!(" TechComp=\"{}\"", item.tech_comp));
    xml.push_str(&format!(
        " IconLocation=\"{}\"",
        escape_attr(item.icon_location)
    ));
    xml.push_str(" Tier=\"1\" AppliedScienceID=\"0\" QualityID=\"2000\"");
    xml.push_str(&format!(
        " Description=\"{}\"",
        escape_attr(item.description)
    ));
    xml.push_str(&format!(" Name=\"{}\"", escape_attr(item.name)));
    xml.push_str(&format!(" ID=\"{}\">", item.item_id));
    xml.push_str(&format!(
        "<InventorySet IsDeletable=\"true\" IsSellable=\"{}\" MaxStackSize=\"{}\"></InventorySet>",
        item.is_sellable, item.max_stack_size
    ));
    xml.push_str("<RequirementsSet IsUnique=\"false\"></RequirementsSet>");
    for container in item.container_sets {
        xml.push_str(&format!("<ContainerSet>{container}</ContainerSet>"));
    }
    xml.push_str("</COOKED_ITEM>");
    xml.into_bytes()
}

/// Escape a value for a double-quoted XML attribute. The shipped PAK leaves
/// `'` bare inside `"..."` (`_2031` "Ba'taur War Beads"), so only the four
/// characters that would break the attribute or the element are escaped.
fn escape_attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}
