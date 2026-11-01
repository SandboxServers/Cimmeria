//! The one place a lab hold becomes a window message. Declared locally
//! (not through a `windows-sys` feature) so `Cargo.toml` and the default
//! build stay untouched (C13 item 1).

use super::hold::Post;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[link(name = "user32")]
extern "system" {
    fn PostMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> i32;
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
const WM_KEYDOWN: u32 = 0x0100;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
const WM_KEYUP: u32 = 0x0101;

/// Post one key message. A no-op off the i686 client (host tests).
pub(crate) fn post(p: &Post) {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        let (msg, hwnd, vk, lparam) = match *p {
            Post::Down { hwnd, vk, lparam } => (WM_KEYDOWN, hwnd, vk, lparam),
            Post::Up { hwnd, vk, lparam } => (WM_KEYUP, hwnd, vk, lparam),
        };
        // SAFETY: PostMessageW is thread-safe and takes plain values; a
        // stale hwnd makes it fail, which returns 0 and is ignored.
        unsafe {
            PostMessageW(hwnd as isize, msg, usize::from(vk), lparam as i32 as isize);
        }
    }
    #[cfg(not(all(target_os = "windows", target_arch = "x86")))]
    let _ = p;
}

pub(crate) fn post_all(ps: &[Post]) {
    for p in ps {
        post(p);
    }
}
