//! Whether the UI owns the mouse (`ui_owned` on `client.input.button`).
//!
//! The game's "CEGUI owns the mouse" test (`0x0056b890`) is a hit test, not a
//! stored flag, so nothing exact can be polled. This reads the nearest field:
//! the CEGUI `System::d_wndWithMouse` (`System+0x28`), which `injectMouseMove`
//! writes, and asks whether it is a real window. The chain is
//! `g_SGWUIManager_ptr` -> `SGWUIManager+0xc` (the `System*`) -> `+0x28`
//! (the window) and `+0x2c` (the active sheet), then the window's name at
//! `+0x1e8`, an MSVC `std::wstring` (RE `controller-mouse-input-model.md` § 6.1).
//!
//! The answer is an approximation, with three caveats: the field is stale while
//! the cursor is still or hidden (mouselook); a capture or modal window counts
//! as owned; and the window-name layout is inferred. Any unreadable link, a
//! null manager or system, or an implausible string header gives `None`, which
//! drops the field from the row. There is no panic path and no call into the
//! game. Fallback: if LN-13 shows the chain is wrong, restore the LN-07 stub
//! body (`None`).

use crate::hooks::entity_trace::map::Mem;

/// `g_SGWUIManager_ptr`: the SGWUIManager singleton pointer (RE § 6.1, static; getter 0x0056f980).
pub const UI_MANAGER_PTR_VA: u32 = 0x01ee_2a58;
/// SGWUIManager + 0xc: the CEGUI `System*` (RE § 6.1, static).
pub const UIMGR_SYSTEM_OFF: u32 = 0x0c;
/// CEGUI System + 0x28: `d_wndWithMouse`, written by `injectMouseMove` 0x011ae5b0 (RE § 6.1, static).
pub const SYSTEM_WND_WITH_MOUSE_OFF: u32 = 0x28;
/// CEGUI System + 0x2c: the active sheet (RE § 6.1, static).
pub const SYSTEM_SHEET_OFF: u32 = 0x2c;
/// CEGUI Window + 0x1e8: the window name, an MSVC `std::wstring` (RE § 6.1; layout inferred).
pub const WINDOW_NAME_OFF: u32 = 0x1e8;
/// MSVC (VC8/9) `std::wstring`, inferred: +0x04 inline buffer or heap pointer,
/// +0x14 length, +0x18 capacity; the text is inline while capacity < 8.
pub const WSTR_BUF_OFF: u32 = 0x04;
pub const WSTR_LEN_OFF: u32 = 0x14;
pub const WSTR_CAP_OFF: u32 = 0x18;
pub const WSTR_INLINE_CAP: u32 = 8;
/// A length or capacity above this means the layout guess is wrong: answer unknown.
pub const WSTR_SANE_MAX: u32 = 0x400;
/// The world backdrop window; the cursor over it is not "UI owns the mouse" (0x0183f8c8).
pub const BACKGROUND_WIN: &str = "BackgroundWin";

/// Whether the UI owns the mouse, or `None` when it is not known.
pub(super) fn read_ui_owned(mem: &impl Mem) -> Option<bool> {
    let mgr = mem.u32_at(UI_MANAGER_PTR_VA).filter(|&p| p != 0)?;
    let system = mem
        .u32_at(mgr.wrapping_add(UIMGR_SYSTEM_OFF))
        .filter(|&p| p != 0)?;
    let wnd = mem.u32_at(system.wrapping_add(SYSTEM_WND_WITH_MOUSE_OFF))?;
    let sheet = mem.u32_at(system.wrapping_add(SYSTEM_SHEET_OFF))?;
    if wnd == 0 || wnd == sheet {
        return Some(false);
    }
    Some(!is_named(
        mem,
        wnd.wrapping_add(WINDOW_NAME_OFF),
        BACKGROUND_WIN,
    )?)
}

/// Whether the `std::wstring` at `wstr` spells `want`. `None` when the header
/// is implausible or the text cannot be read.
fn is_named(mem: &impl Mem, wstr: u32, want: &str) -> Option<bool> {
    let len = mem.u32_at(wstr.wrapping_add(WSTR_LEN_OFF))?;
    let cap = mem.u32_at(wstr.wrapping_add(WSTR_CAP_OFF))?;
    if len > cap || cap > WSTR_SANE_MAX {
        return None;
    }
    let units: Vec<u16> = want.encode_utf16().collect();
    if len as usize != units.len() {
        return Some(false);
    }
    let text = if cap < WSTR_INLINE_CAP {
        wstr.wrapping_add(WSTR_BUF_OFF)
    } else {
        mem.u32_at(wstr.wrapping_add(WSTR_BUF_OFF))
            .filter(|&p| p != 0)?
    };
    let bytes = mem.bytes_at(text, units.len() * 2)?;
    let decoded: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    Some(decoded == units)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::entity_trace::map::fake::FakeMem;

    const MGR: u32 = 0x3000_0000;
    const SYS: u32 = 0x3000_1000;
    const SHEET: u32 = 0x3000_2000;
    const WND: u32 = 0x3000_3000;
    const HEAP: u32 = 0x3000_4000;

    /// The manager, system and window chain, with `wnd` as the mouse window.
    fn world(mem: &mut FakeMem, wnd: u32) {
        mem.set(UI_MANAGER_PTR_VA, MGR);
        mem.set(MGR + UIMGR_SYSTEM_OFF, SYS);
        mem.set(SYS + SYSTEM_WND_WITH_MOUSE_OFF, wnd);
        mem.set(SYS + SYSTEM_SHEET_OFF, SHEET);
    }

    /// Lay out `text` as a `std::wstring` at `at`: inline when short, else on
    /// the heap at `HEAP`. Text is packed two UTF-16 units per little-endian word.
    fn put_wstr(mem: &mut FakeMem, at: u32, text: &str) {
        let len = text.encode_utf16().count() as u32;
        let (text_at, cap) = if len < WSTR_INLINE_CAP {
            (at + WSTR_BUF_OFF, WSTR_INLINE_CAP - 1)
        } else {
            mem.set(at + WSTR_BUF_OFF, HEAP);
            (HEAP, len)
        };
        put_units(mem, text_at, text);
        mem.set(at + WSTR_LEN_OFF, len);
        mem.set(at + WSTR_CAP_OFF, cap);
    }

    /// Pack the UTF-16 units of `text` at `at`, padding an odd unit with 0.
    fn put_units(mem: &mut FakeMem, at: u32, text: &str) {
        let mut units: Vec<u16> = text.encode_utf16().collect();
        if units.len() % 2 == 1 {
            units.push(0);
        }
        for (i, pair) in units.chunks_exact(2).enumerate() {
            let word = u32::from(pair[0]) | (u32::from(pair[1]) << 16);
            mem.set(at + 4 * i as u32, word);
        }
    }

    #[test]
    fn no_manager_is_unknown() {
        let mem = FakeMem::default();
        assert_eq!(read_ui_owned(&mem), None);
        let mut mem = FakeMem::default();
        mem.set(UI_MANAGER_PTR_VA, 0);
        assert_eq!(read_ui_owned(&mem), None);
    }

    #[test]
    fn null_system_is_unknown() {
        let mut mem = FakeMem::default();
        mem.set(UI_MANAGER_PTR_VA, MGR);
        mem.set(MGR + UIMGR_SYSTEM_OFF, 0);
        assert_eq!(read_ui_owned(&mem), None);
    }

    #[test]
    fn no_window_is_not_owned() {
        let mut mem = FakeMem::default();
        world(&mut mem, 0);
        assert_eq!(read_ui_owned(&mem), Some(false));
    }

    #[test]
    fn sheet_is_not_owned() {
        let mut mem = FakeMem::default();
        world(&mut mem, SHEET);
        assert_eq!(read_ui_owned(&mem), Some(false));
    }

    #[test]
    fn background_win_is_not_owned() {
        let mut mem = FakeMem::default();
        world(&mut mem, WND);
        put_wstr(&mut mem, WND + WINDOW_NAME_OFF, "BackgroundWin");
        assert_eq!(read_ui_owned(&mem), Some(false));
    }

    #[test]
    fn named_window_is_owned() {
        let mut mem = FakeMem::default();
        world(&mut mem, WND);
        put_wstr(&mut mem, WND + WINDOW_NAME_OFF, "ChatWindow");
        assert_eq!(read_ui_owned(&mem), Some(true));

        let mut mem = FakeMem::default();
        world(&mut mem, WND);
        put_wstr(&mut mem, WND + WINDOW_NAME_OFF, "Chat");
        assert_eq!(read_ui_owned(&mem), Some(true));
    }

    #[test]
    fn same_length_other_name_is_owned() {
        let mut mem = FakeMem::default();
        world(&mut mem, WND);
        put_wstr(&mut mem, WND + WINDOW_NAME_OFF, "BackgroundWiX");
        assert_eq!(read_ui_owned(&mem), Some(true));
    }

    #[test]
    fn bad_header_is_unknown() {
        let mut mem = FakeMem::default();
        world(&mut mem, WND);
        mem.set(WND + WINDOW_NAME_OFF + WSTR_LEN_OFF, 0x5000);
        mem.set(WND + WINDOW_NAME_OFF + WSTR_CAP_OFF, 0x5000);
        assert_eq!(read_ui_owned(&mem), None);

        let mut mem = FakeMem::default();
        world(&mut mem, WND);
        mem.set(WND + WINDOW_NAME_OFF + WSTR_LEN_OFF, 13);
        mem.set(WND + WINDOW_NAME_OFF + WSTR_CAP_OFF, 9);
        assert_eq!(read_ui_owned(&mem), None);
    }

    #[test]
    fn unreadable_text_is_unknown() {
        let mut mem = FakeMem::default();
        world(&mut mem, WND);
        let name = WND + WINDOW_NAME_OFF;
        mem.set(name + WSTR_LEN_OFF, 13);
        mem.set(name + WSTR_CAP_OFF, 15);
        // The heap pointer names an address with no words behind it.
        mem.set(name + WSTR_BUF_OFF, 0x3000_9000);
        assert_eq!(read_ui_owned(&mem), None);
    }
}
