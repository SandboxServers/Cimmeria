//! Window capture by PID for `lab_screenshot`, via the Win32 GDI
//! `PrintWindow` path, returned to the agent as a PNG.
//!
//! The GDI capture is integration-only (needs a live desktop + window)
//! and requires live validation. The pixel-format conversion
//! ([`bgra_to_rgba`]) and PNG/base64 encoding ([`encode_png`],
//! [`png_to_base64`]) are pure and unit-tested off-process.

use base64::Engine;

/// A captured frame in top-down RGBA8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Convert a top-down BGRA buffer (what `GetDIBits` gives for a 32bpp
/// `BI_RGB` DIB) to RGBA and force alpha opaque (GDI leaves alpha 0).
pub fn bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bgra.len());
    for px in bgra.chunks_exact(4) {
        out.push(px[2]); // R
        out.push(px[1]); // G
        out.push(px[0]); // B
        out.push(0xff); // A — GDI captures leave this 0
    }
    out
}

/// Encode an RGBA8 image to PNG bytes.
pub fn encode_png(img: &CapturedImage) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut buf, img.width, img.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(|e| format!("png header: {e}"))?;
        writer
            .write_image_data(&img.rgba)
            .map_err(|e| format!("png data: {e}"))?;
    }
    Ok(buf)
}

/// Base64 (standard) encode PNG bytes for an MCP image content block.
pub fn png_to_base64(png_bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(png_bytes)
}

/// Capture the main window of `pid` and return it as an RGBA image.
#[cfg(windows)]
pub fn capture_pid(pid: u32) -> Result<CapturedImage, String> {
    let hwnd = super::process::find_main_window(pid)
        .ok_or_else(|| format!("no visible top-level window for pid {pid}"))?;
    win::capture_window(hwnd)
}

#[cfg(not(windows))]
pub fn capture_pid(_pid: u32) -> Result<CapturedImage, String> {
    Err("window capture is Windows-only".to_string())
}

#[cfg(windows)]
mod win {
    use super::{bgra_to_rgba, CapturedImage};
    use core::ffi::c_void;

    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetWindowDC,
        ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
    };
    // PrintWindow lives in the Xps namespace in windows-sys (it is the
    // user32 print-to-DC entry point).
    use windows_sys::Win32::Storage::Xps::PrintWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;

    const BI_RGB: u32 = 0;
    const DIB_RGB_COLORS: u32 = 0;
    /// `PW_RENDERFULLCONTENT` — render even for DirectX/occluded content.
    const PW_RENDERFULLCONTENT: u32 = 0x0000_0002;

    pub fn capture_window(hwnd_raw: isize) -> Result<CapturedImage, String> {
        let hwnd = hwnd_raw as HWND;
        // SAFETY: all handles checked; buffers sized from the rect.
        unsafe {
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return Err("GetWindowRect failed".to_string());
            }
            let width = (rect.right - rect.left).max(0) as u32;
            let height = (rect.bottom - rect.top).max(0) as u32;
            if width == 0 || height == 0 {
                return Err("window has zero area".to_string());
            }

            let hdc_window = GetWindowDC(hwnd);
            if hdc_window.is_null() {
                return Err("GetWindowDC failed".to_string());
            }
            let hdc_mem = CreateCompatibleDC(hdc_window);
            let hbmp = CreateCompatibleBitmap(hdc_window, width as i32, height as i32);
            let old = SelectObject(hdc_mem, hbmp as _);

            let printed = PrintWindow(hwnd, hdc_mem, PW_RENDERFULLCONTENT);

            // Top-down 32bpp BI_RGB: negative height.
            let mut bmi: BITMAPINFO = core::mem::zeroed();
            bmi.bmiHeader.biSize = core::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width as i32;
            bmi.bmiHeader.biHeight = -(height as i32);
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB;

            let mut bgra = vec![0u8; (width * height * 4) as usize];
            let scanned = GetDIBits(
                hdc_mem,
                hbmp,
                0,
                height,
                bgra.as_mut_ptr() as *mut c_void,
                &mut bmi,
                DIB_RGB_COLORS,
            );

            // Cleanup regardless of success.
            SelectObject(hdc_mem, old);
            DeleteObject(hbmp as _);
            DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_window);

            if printed == 0 {
                return Err("PrintWindow failed".to_string());
            }
            if scanned == 0 {
                return Err("GetDIBits returned 0 scanlines".to_string());
            }

            Ok(CapturedImage {
                width,
                height,
                rgba: bgra_to_rgba(&bgra),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgra_to_rgba_swaps_and_opaques() {
        // One pixel BGRA = (B=10, G=20, R=30, A=0).
        let out = bgra_to_rgba(&[10, 20, 30, 0]);
        assert_eq!(out, vec![30, 20, 10, 0xff]);
    }

    #[test]
    fn encode_png_produces_valid_signature() {
        let img = CapturedImage {
            width: 2,
            height: 2,
            rgba: vec![255; 2 * 2 * 4],
        };
        let png = encode_png(&img).unwrap();
        // PNG magic.
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    }

    #[test]
    fn png_base64_round_trips() {
        let bytes = b"\x89PNG\r\n\x1a\n";
        let b64 = png_to_base64(bytes);
        let back = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        assert_eq!(back, bytes);
    }
}
