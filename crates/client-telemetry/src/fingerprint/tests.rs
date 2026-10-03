use super::*;
use std::collections::HashMap;

/// A fake address space holding exactly the QA build's bytes at every
/// site, which individual tests then disturb.
fn stock_build() -> HashMap<usize, Vec<u8>> {
    let mut mem = HashMap::new();
    for site in CODE_SITES {
        mem.insert(site.address, site.expected.to_vec());
    }
    for slot in SLOT_SITES {
        mem.insert(slot.address, slot.expected.to_le_bytes().to_vec());
    }
    mem
}

fn reader(mem: &HashMap<usize, Vec<u8>>) -> impl Fn(usize, usize) -> Option<Vec<u8>> + '_ {
    move |addr, len| {
        mem.get(&addr)
            .filter(|b| b.len() >= len)
            .map(|b| b[..len].to_vec())
    }
}

#[test]
fn the_stock_build_passes() {
    let mem = stock_build();
    let report = check(reader(&mem), &[]);
    assert!(report.is_usable(), "{:#?}", report.lines());
    assert_eq!(report.code.len(), CODE_SITES.len());
    assert_eq!(report.slots.len(), SLOT_SITES.len());
}

/// A process that is not `SGW.exe` (the test host, another build) has
/// nothing readable at these addresses: nothing is hooked.
#[test]
fn an_empty_address_space_fails_closed() {
    let report = check(|_, _| None, &[]);
    assert!(!report.is_usable());
    assert!(report.lines().iter().all(|l| l.ends_with("unreadable")));
}

#[test]
fn one_changed_byte_anywhere_fails_the_whole_gate() {
    for site in CODE_SITES {
        let mut mem = stock_build();
        mem.get_mut(&site.address).unwrap()[3] ^= 0xFF;
        assert!(!check(reader(&mem), &[]).is_usable(), "{}", site.name);
    }
}

#[test]
fn a_moved_vtable_slot_fails_the_gate() {
    for slot in SLOT_SITES {
        let mut mem = stock_build();
        mem.insert(slot.address, 0x1234_5678u32.to_le_bytes().to_vec());
        let report = check(reader(&mem), &[]);
        assert!(!report.is_usable(), "{}", slot.name);
        assert!(report
            .lines()
            .iter()
            .any(|l| l.contains("expected 0x") && l.contains("found 0x12345678")));
    }
}

/// The client-patches DLL got to Tick and the drop callee first: its
/// MinHook jumps are chained, but only into its own image.
#[test]
fn a_client_patches_hook_on_a_shared_site_is_chained() {
    const PATCHES: Range<usize> = 0x6000_0000..0x6010_0000;
    let mut mem = stock_build();
    for site in [
        cimmeria_client_hookgate::ENGINE_TICK,
        cimmeria_client_hookgate::DROP_CALLEE,
    ] {
        let bytes = mem.get_mut(&site.address).unwrap();
        let rel = 0x6000_1000u32.wrapping_sub(site.address as u32 + 5);
        bytes[0] = 0xE9;
        bytes[1..5].copy_from_slice(&rel.to_le_bytes());
    }
    let report = check(reader(&mem), &[PATCHES]);
    assert!(report.is_usable(), "{:#?}", report.lines());
    assert_eq!(
        report
            .verdicts()
            .iter()
            .filter(|(_, v)| *v == "chained")
            .count(),
        2
    );
    assert!(!check(reader(&mem), &[]).is_usable(), "owner not loaded");
}

/// The same jump on a site only this DLL hooks is not a chain.
#[test]
fn a_jump_on_a_telemetry_only_site_is_a_mismatch() {
    let mut mem = stock_build();
    mem.get_mut(&BINK_TICK.address).unwrap()[0] = 0xE9;
    let everywhere: Range<usize> = 0..usize::MAX;
    assert!(!check(reader(&mem), &[everywhere]).is_usable());
}

/// The CEGUI slot is slot 1 of the vtable, not the RTTI locator's
/// neighbour (the destructor) the old anchor swapped.
#[test]
fn cegui_slot_is_log_event_not_the_destructor() {
    assert_eq!(CEGUI_LOG_EVENT_SLOT.address, 0x01ac_1ba8 + 2 * 4);
    assert_eq!(CEGUI_LOG_EVENT_SLOT.expected, 0x0121_29e0);
}

#[test]
fn verdicts_name_every_site() {
    let mem = stock_build();
    let verdicts = check(reader(&mem), &[]).verdicts();
    assert_eq!(verdicts.len(), CODE_SITES.len() + SLOT_SITES.len());
    assert!(verdicts.iter().all(|(_, v)| *v == "stock"));
}

#[test]
fn every_code_site_can_be_chained_over_by_minhook() {
    // MinHook needs at least five bytes, and a chained site compares
    // what follows them.
    for site in CODE_SITES {
        assert!(site.expected.len() > 5, "{}", site.name);
    }
}
