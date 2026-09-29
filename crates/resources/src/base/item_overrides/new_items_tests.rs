//! Guards for the generated `CookedDataItems` additions (AM-07): the bytes the
//! client receives, the served category, and agreement with the AM-F seed.

use super::{
    apply_override, generate_item_xml, NewItem, AMMO_ITEMS, ITEM_ADDITIONS, ITEM_OVERRIDES,
};
use crate::base::dialog_overrides::MAX_COOKED_ELEMENT_ID;
use crate::base::resources::ResourceCache;
use std::io::Read;

const CATEGORY_ITEMS: u32 = 4;

fn data_dir() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache")
}

/// Read the shipped PAK straight from the ZIP, not through `ResourceCache`, so
/// "shipped" can never mean "already overridden".
fn shipped_items_pak() -> zip::ZipArchive<std::fs::File> {
    let file = std::fs::File::open(format!("{}/CookedDataItems.pak", data_dir()))
        .expect("committed CookedDataItems.pak opens");
    zip::ZipArchive::new(file).expect("CookedDataItems.pak is a ZIP")
}

fn shipped_entry(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<Vec<u8>> {
    let mut entry = archive.by_name(name).ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).expect("PAK entry reads");
    Some(buf)
}

fn shipped_metadata(archive: &mut zip::ZipArchive<std::fs::File>) -> u32 {
    let bytes = shipped_entry(archive, "MetaData").expect("PAK has MetaData");
    u32::from_le_bytes(bytes[..4].try_into().expect("4-byte MetaData"))
}

// ── Serialization ─────────────────────────────────────────────────

/// The generator reproduces two shipped entries byte for byte: `_10` (not
/// sellable, one container) and `_1086` (sellable, two containers, a
/// non-default `TechComp`). This is the evidence the added entries are in the
/// exact shape the client already parses. It fails on any change to the
/// attribute order, the namespace prologue, the one `\n`, the explicit
/// `></InventorySet>` close tags or the `ContainerSet` rendering.
#[test]
fn generator_reproduces_shipped_entries_byte_for_byte() {
    let mut pak = shipped_items_pak();
    let cases = [
        NewItem {
            item_id: 10,
            name: "Gopher Head",
            description: "The severed head of a gopher.",
            icon_location: "set:CoreWidgets image:IconMissing",
            max_stack_size: 8,
            tech_comp: 0,
            is_sellable: false,
            container_sets: &[2],
        },
        NewItem {
            item_id: 1086,
            name: "Airburst Rangefinder",
            description: "dec. target cover bonus",
            icon_location: "set:CoreWidgets image:IconMissing",
            max_stack_size: 1,
            tech_comp: 25,
            is_sellable: true,
            container_sets: &[1, 17],
        },
    ];
    for item in &cases {
        let shipped = shipped_entry(&mut pak, &format!("_{}", item.item_id))
            .unwrap_or_else(|| panic!("_{} ships in the PAK", item.item_id));
        assert_eq!(
            String::from_utf8_lossy(&generate_item_xml(item)),
            String::from_utf8_lossy(&shipped),
            "generated _{} must equal the shipped entry",
            item.item_id
        );
    }
}

/// The spike item, Hollow Point Rounds (9001), pinned as the exact bytes the
/// client is sent. Any drift in what the tester sees in the UAT (name, icon,
/// stack cap, containers) fails here first.
#[test]
fn hollow_point_rounds_9001_serializes_byte_exact() {
    let item = AMMO_ITEMS
        .iter()
        .find(|i| i.item_id == 9001)
        .expect("9001 is an ammo item");
    let expected = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <COOKED_ITEM xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" \
        xmlns:SOAP-ENC=\"http://schemas.xmlsoap.org/soap/encoding/\" \
        xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
        xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" xmlns:CookedData1=\"SGW\" \
        IsReverseEngineerable=\"false\" IsResearchable=\"false\" \
        IsElementaryComponent=\"true\" IsKicker=\"false\" TechComp=\"1\" \
        IconLocation=\"set:AmmoType_Icons image:Bullet_Hollow_Point\" Tier=\"1\" \
        AppliedScienceID=\"0\" QualityID=\"2000\" \
        Description=\"Special ammunition. Select Hollow Point in a compatible weapon's ammo menu to fire these rounds.\" \
        Name=\"Hollow Point Rounds\" ID=\"9001\">\
        <InventorySet IsDeletable=\"true\" IsSellable=\"true\" MaxStackSize=\"500\"></InventorySet>\
        <RequirementsSet IsUnique=\"false\"></RequirementsSet>\
        <ContainerSet>1</ContainerSet><ContainerSet>15</ContainerSet><ContainerSet>17</ContainerSet>\
        </COOKED_ITEM>";
    assert_eq!(
        String::from_utf8(generate_item_xml(item)).unwrap(),
        expected
    );
}

/// Attribute values are escaped for a double-quoted attribute; a bare `'` is
/// left alone, as the shipped PAK does (`_2031` "Ba'taur War Beads").
#[test]
fn generator_escapes_attribute_values() {
    let item = NewItem {
        item_id: 1,
        name: "A \"B\" & <C>",
        description: "Ba'taur",
        icon_location: "set:X image:Y",
        max_stack_size: 1,
        tech_comp: 0,
        is_sellable: false,
        container_sets: &[],
    };
    let xml = String::from_utf8(generate_item_xml(&item)).unwrap();
    assert!(
        xml.contains("Name=\"A &quot;B&quot; &amp; &lt;C&gt;\""),
        "{xml}"
    );
    assert!(xml.contains("Description=\"Ba'taur\""), "{xml}");
    assert!(xml.contains("</RequirementsSet></COOKED_ITEM>"), "{xml}");
}

// ── The served category ───────────────────────────────────────────

/// What the version handshake hands a client: the served `CookedDataItems`
/// holds every addition as generated bytes, lists each id among the category's
/// overridden elements, and carries a version the shipped PAK does not, so a
/// client holding the shipped category is resynced (#840) and receives them.
/// The Slappack overrides (#405) are served exactly as before.
///
/// Fails if the additions are not inserted, not listed, or not in the bump.
#[test]
fn served_items_category_carries_the_additions_and_the_old_overrides() {
    let cache = ResourceCache::load_all(data_dir()).expect("committed PAKs load");
    let served = cache.category(CATEGORY_ITEMS).expect("category 4 served");
    let mut pak = shipped_items_pak();

    let listed = cache.overridden_elements(CATEGORY_ITEMS);
    for item in ITEM_ADDITIONS {
        assert!(
            listed.contains(&item.item_id),
            "addition {} must be listed among the overridden items: {listed:?}",
            item.item_id
        );
        assert_eq!(
            cache.get(CATEGORY_ITEMS, item.item_id),
            Some(&generate_item_xml(item)),
            "addition {} must be served as its generated XML",
            item.item_id
        );
    }
    for id in 9000..=9014u32 {
        assert!(listed.contains(&id), "ammo item {id} must be pushed");
    }

    // The existing overrides are unchanged: still listed, still the patched
    // shipped entry.
    for ov in ITEM_OVERRIDES {
        assert!(listed.contains(&ov.item_id));
        let shipped = shipped_entry(&mut pak, &format!("_{}", ov.item_id))
            .unwrap_or_else(|| panic!("_{} ships", ov.item_id));
        assert_eq!(
            cache.get(CATEGORY_ITEMS, ov.item_id),
            apply_override(&shipped, ov).as_ref(),
            "override {} must be served exactly as before the additions",
            ov.item_id
        );
    }
    assert_eq!(
        listed.len(),
        ITEM_OVERRIDES.len() + ITEM_ADDITIONS.len(),
        "exactly the overrides and the additions are listed: {listed:?}"
    );

    // Shipped entries untouched; additions extend the category.
    assert_eq!(
        cache.get(CATEGORY_ITEMS, 1086),
        shipped_entry(&mut pak, "_1086").as_ref()
    );
    let shipped_count = pak.file_names().filter(|n| n.starts_with('_')).count();
    assert_eq!(served.elements.len(), shipped_count + ITEM_ADDITIONS.len());

    // A client holding the shipped category must see a mismatch.
    assert_ne!(served.metadata, shipped_metadata(&mut pak));
}

/// Each addition is a genuinely new key: absent from the shipped PAK (so it
/// shadows nothing, and the skip branch never fires on the committed data),
/// at most 65535 (ids above that crashed the client, see
/// `cooked-dialog-override-crash.md`), unique, and disjoint from the patches.
#[test]
fn additions_are_new_unique_16_bit_keys() {
    let mut pak = shipped_items_pak();
    let mut seen = std::collections::HashSet::new();
    for item in ITEM_ADDITIONS {
        assert!(item.item_id <= MAX_COOKED_ELEMENT_ID, "{}", item.item_id);
        assert!(
            seen.insert(item.item_id),
            "duplicate addition {}",
            item.item_id
        );
        assert!(
            shipped_entry(&mut pak, &format!("_{}", item.item_id)).is_none(),
            "addition {} ships in the PAK; patch it with an ItemOverride instead",
            item.item_id
        );
        assert!(
            ITEM_OVERRIDES.iter().all(|ov| ov.item_id != item.item_id),
            "{} is both patched and added",
            item.item_id
        );
    }
}

// ── Agreement with the seed ───────────────────────────────────────

const AMMO_ITEMS_SEED: &str = include_str!("../../../../../db/resources/Items/Seed/ammo_items.sql");
const AMMO_ITEM_TYPES_SEED: &str =
    include_str!("../../../../../db/resources/Items/Seed/ammo_item_types.sql");

/// Image names in the client's
/// `Content/UI/CEGUIData/imagesets/AmmoType_Icons.imageset`, declared in
/// `TaharezLook.scheme`. Copied here because the client is not in git.
const AMMO_TYPE_ICONS_IMAGES: &[&str] = &[
    "Bullet_Default",
    "Bullet_Hollow_Point",
    "Bullet_Armor_Piercing",
    "Bullet_Incendiary",
    "Bullet_Explosive",
    "Bullet_EMP",
    "Dart_Default",
    "Dart_Poison",
    "Dart_Disease",
    "Dart_Tranquilizer",
    "Dart_EMP",
    "Dart_Radioactive",
    "Dart_Stim",
    "Dart_Coagulant",
    "Dart_Nanites",
    "Dart_Antidote",
    "Dart_Adrenaline",
    "Dagger_Default",
    "Dagger_Metallic",
    "Dagger_Poison",
    "Dagger_Electrical",
    "Dagger_Disease",
    "Dagger_Plasma",
];

/// Split one SQL tuple body on commas outside single quotes.
fn split_sql_list(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in body.chars() {
        match c {
            '\'' => {
                quoted = !quoted;
                cur.push(c);
            }
            ',' if !quoted => out.push(std::mem::take(&mut cur).trim().to_string()),
            _ => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

fn unquote(v: &str) -> &str {
    v.trim_matches('\'')
}

/// One row of `ammo_items.sql`, as column name -> raw value.
fn seed_rows() -> Vec<std::collections::HashMap<String, String>> {
    AMMO_ITEMS_SEED
        .lines()
        .filter(|l| l.starts_with("INSERT INTO items"))
        .map(|line| {
            let cols_start = line.find('(').unwrap() + 1;
            let cols_end = line.find(") VALUES (").unwrap();
            let vals_start = cols_end + ") VALUES (".len();
            let vals_end = line.rfind(");").unwrap();
            let cols = split_sql_list(&line[cols_start..cols_end]);
            let vals = split_sql_list(&line[vals_start..vals_end]);
            assert_eq!(cols.len(), vals.len(), "{line}");
            cols.into_iter().zip(vals).collect()
        })
        .collect()
}

/// Every seeded ammo item has a cooked addition that says the same thing, and
/// vice versa: name, stack cap, `TechComp`, sellable flag and bags. A drift
/// makes the client stack or name an item differently from the server.
#[test]
fn ammo_additions_agree_with_the_ammo_items_seed() {
    const ITEM_FLAG_CAN_BE_SOLD: i64 = 1 << 10;
    let rows = seed_rows();
    assert_eq!(rows.len(), 15, "ammo_items.sql seeds 15 items");
    assert_eq!(AMMO_ITEMS.len(), rows.len());
    for row in &rows {
        let id: u32 = row["item_id"].parse().unwrap();
        let item = AMMO_ITEMS
            .iter()
            .find(|i| i.item_id == id)
            .unwrap_or_else(|| panic!("seeded ammo item {id} has no cooked addition"));
        assert_eq!(item.name, unquote(&row["name"]), "{id} name");
        assert_eq!(
            item.max_stack_size.to_string(),
            row["max_stack_size"],
            "{id} max_stack_size"
        );
        assert_eq!(
            item.tech_comp.to_string(),
            row["tech_comp"],
            "{id} tech_comp"
        );
        let flags: i64 = row["flags"].parse().unwrap();
        assert_eq!(
            item.is_sellable,
            flags & ITEM_FLAG_CAN_BE_SOLD != 0,
            "{id} sellable"
        );
        let containers: Vec<u32> = unquote(&row["container_sets"])
            .trim_matches(|c| c == '{' || c == '}')
            .split(',')
            .map(|c| c.parse().unwrap())
            .collect();
        assert_eq!(
            item.container_sets,
            containers.as_slice(),
            "{id} container_sets"
        );
    }
}

/// Each ammo item's icon is its own `EAmmoType`'s image in `AmmoType_Icons`,
/// per the `ammo_item_types.sql` mapping, and that image exists in the
/// client's imageset. A swapped row (Hollow Point showing the Armor Piercing
/// icon) fails here.
#[test]
fn ammo_icons_follow_the_ammo_item_types_mapping() {
    let mut mapped = 0;
    for line in AMMO_ITEM_TYPES_SEED.lines() {
        let line = line.trim();
        let Some(body) = line.strip_prefix("('") else {
            continue;
        };
        let (label, rest) = body.split_once("', ").expect("('Label', id) row");
        let id: u32 = rest
            .trim_end_matches([')', ',', ';'])
            .parse()
            .expect("item id");
        assert!(
            cimmeria_entity::ammo_type::LABELS.contains(&label),
            "{label} is an EAmmoType label"
        );
        assert!(
            AMMO_TYPE_ICONS_IMAGES.contains(&label),
            "{label} in AmmoType_Icons"
        );
        let item = AMMO_ITEMS
            .iter()
            .find(|i| i.item_id == id)
            .unwrap_or_else(|| panic!("mapped item {id} has no cooked addition"));
        assert_eq!(
            item.icon_location,
            format!("set:AmmoType_Icons image:{label}"),
            "item {id} ({label})"
        );
        mapped += 1;
    }
    assert_eq!(mapped, 15, "ammo_item_types.sql maps 15 types");
}
