//! Asset browsing and texture thumbnail commands.
//!
//! Provides package scanning, cross-package search, and texture thumbnail
//! generation for the scene editor's asset browser panel.

use std::io::Cursor;

use crate::state::AppState;
use cimmeria_upk::Package;
use cimmeria_upk_objects::{PackageIndex, PixelFormat, deserialize_texture2d};
use serde::Serialize;

// ---- Response types --------------------------------------------------------

#[derive(Serialize)]
pub struct PackageScanResult {
    pub package_count: usize,
    pub total_exports: usize,
    pub class_counts: Vec<(String, usize)>,
}

#[derive(Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub export_count: usize,
}

#[derive(Serialize)]
pub struct ExportInfo {
    pub index: usize,
    pub class_name: String,
    pub object_name: String,
    pub full_path: String,
    pub serial_size: i32,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub package_name: String,
    pub export_index: usize,
    pub class_name: String,
    pub object_name: String,
    pub full_path: String,
}

#[derive(Serialize)]
pub struct TextureThumbnail {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub data_base64: String,
}

// ---- Commands --------------------------------------------------------------

/// Scan all packages in `{cooked_pc}/Packages/` and build the package index.
/// Returns package count and class distribution summary.
#[tauri::command]
pub async fn scan_packages(
    state: tauri::State<'_, AppState>,
) -> Result<PackageScanResult, String> {
    let cooked_pc = {
        let guard = state.cooked_pc_path.lock().unwrap();
        guard.clone().ok_or("CookedPC path not set")?
    };

    let packages_dir = cooked_pc.join("Packages");
    if !packages_dir.is_dir() {
        return Err(format!(
            "Packages directory not found: {}",
            packages_dir.display()
        ));
    }

    tracing::info!("Scanning packages in {}...", packages_dir.display());

    let idx = PackageIndex::build(&packages_dir)
        .map_err(|e| format!("Failed to build package index: {e}"))?;

    let package_count = idx.package_count as usize;
    let total_exports = idx.export_count as usize;

    // Build class distribution, sorted by count descending
    let mut class_counts: Vec<(String, usize)> = idx
        .by_class
        .iter()
        .map(|(class, entries)| (class.clone(), entries.len()))
        .collect();
    class_counts.sort_by(|a, b| b.1.cmp(&a.1));

    // Save index to disk for subsequent launches
    let index_path = cooked_pc.join("package_index.bin");
    if let Err(e) = idx.save(&index_path) {
        tracing::warn!("Failed to save package index: {}", e);
    }

    tracing::info!(
        "Package scan complete: {} packages, {} exports",
        package_count,
        total_exports
    );

    // Store in state
    {
        let mut guard = state.package_index.lock().unwrap();
        *guard = Some(idx);
    }

    Ok(PackageScanResult {
        package_count,
        total_exports,
        class_counts,
    })
}

/// List all packages that were scanned.
#[tauri::command]
pub async fn list_packages(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PackageInfo>, String> {
    let idx_guard = state.package_index.lock().unwrap();
    let idx = idx_guard.as_ref().ok_or("Package index not built — run scan_packages first")?;

    // Collect unique package names with their export counts.
    // The exports map is keyed by (package_name, object_name), so we group by
    // the first element.
    let mut pkg_map: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (pkg_name, _obj_name) in idx.exports.keys() {
        *pkg_map.entry(pkg_name.as_str()).or_default() += 1;
    }

    let mut packages: Vec<PackageInfo> = pkg_map
        .into_iter()
        .map(|(name, export_count)| PackageInfo {
            name: name.to_string(),
            export_count,
        })
        .collect();
    packages.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(packages)
}

/// List exports in a specific package, optionally filtered by class.
#[tauri::command]
pub async fn list_package_exports(
    state: tauri::State<'_, AppState>,
    package_name: String,
    class_filter: Option<String>,
) -> Result<Vec<ExportInfo>, String> {
    // Find the file path for this package from the index. We grab the first
    // export location that matches the package name to get the file path.
    let file_path = {
        let idx_guard = state.package_index.lock().unwrap();
        let idx = idx_guard
            .as_ref()
            .ok_or("Package index not built — run scan_packages first")?;

        idx.exports
            .iter()
            .find(|((pkg, _), _)| pkg == &package_name)
            .map(|(_, loc)| loc.file_path.clone())
            .ok_or_else(|| format!("Package '{}' not found in index", package_name))?
    };

    let pkg = Package::open(&file_path)
        .map_err(|e| format!("Failed to open {}: {e}", file_path.display()))?;

    let class_filter_lower = class_filter.as_deref().map(|s| s.to_lowercase());

    let mut exports = Vec::new();
    for (i, export) in pkg.exports.iter().enumerate() {
        if export.serial_size <= 0 {
            continue;
        }
        let class_name = pkg.export_class_name(export).to_string();

        if let Some(ref filter) = class_filter_lower {
            if class_name.to_lowercase() != *filter {
                continue;
            }
        }

        exports.push(ExportInfo {
            index: i,
            class_name,
            object_name: export.object_name.clone(),
            full_path: pkg.export_full_path(export),
            serial_size: export.serial_size,
        });
    }

    Ok(exports)
}

/// Search across all indexed packages for assets matching a query string.
///
/// Matches against object names (case-insensitive substring). Optionally
/// filtered by class name.
#[tauri::command]
pub async fn search_assets(
    state: tauri::State<'_, AppState>,
    query: String,
    class_filter: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    let idx_guard = state.package_index.lock().unwrap();
    let idx = idx_guard.as_ref().ok_or("Package index not built — run scan_packages first")?;

    let query_lower = query.to_lowercase();
    let class_filter_lower = class_filter.as_deref().map(|s| s.to_lowercase());
    let max_results = limit.unwrap_or(200);

    let mut results = Vec::new();

    for ((pkg_name, obj_name), loc) in &idx.exports {
        if !obj_name.to_lowercase().contains(&query_lower) {
            continue;
        }
        if let Some(ref cf) = class_filter_lower {
            if loc.class_name.to_lowercase() != *cf {
                continue;
            }
        }

        results.push(SearchResult {
            package_name: pkg_name.clone(),
            export_index: loc.export_index,
            class_name: loc.class_name.clone(),
            object_name: obj_name.clone(),
            // Build a synthetic path from the index data. The full_path from
            // Package::export_full_path isn't available without opening the file,
            // so we use pkg.obj as a reasonable approximation.
            full_path: format!("{}.{}", pkg_name, obj_name),
        });

        if results.len() >= max_results {
            break;
        }
    }

    // Sort by package name then object name for deterministic output
    results.sort_by(|a, b| {
        a.package_name
            .cmp(&b.package_name)
            .then(a.object_name.cmp(&b.object_name))
    });

    Ok(results)
}

/// Generate a thumbnail PNG for a Texture2D asset.
///
/// Finds the texture via the package index, deserializes its mip chain,
/// decompresses the DXT data, and returns a base64-encoded PNG.
#[tauri::command]
pub async fn get_texture_thumbnail(
    state: tauri::State<'_, AppState>,
    package_name: String,
    object_name: String,
    max_size: Option<u32>,
) -> Result<TextureThumbnail, String> {
    let max_size = max_size.unwrap_or(128);

    // Look up the export location from the package index
    let loc = {
        let idx_guard = state.package_index.lock().unwrap();
        let idx = idx_guard
            .as_ref()
            .ok_or("Package index not built — run scan_packages first")?;

        idx.find(&package_name, &object_name)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "Texture '{}' not found in package '{}'",
                    object_name, package_name
                )
            })?
    };

    // Open the package and read export data
    let pkg = Package::open(&loc.file_path)
        .map_err(|e| format!("Failed to open {}: {e}", loc.file_path.display()))?;

    let export = pkg
        .exports
        .get(loc.export_index)
        .ok_or_else(|| format!("Export index {} out of range", loc.export_index))?;

    let data = pkg
        .read_export_data(export)
        .map_err(|e| format!("Read export data: {e}"))?;

    let texture = deserialize_texture2d(&data, &pkg.names)
        .map_err(|e| format!("Deserialize Texture2D: {e}"))?;

    // Pick the best mip: smallest one that is >= max_size, or the largest available.
    // Mips are ordered largest-first in UE3.
    let mip = pick_mip(&texture.mips, max_size)
        .ok_or("No mip levels with data (texture may use external .tfc storage)")?;

    if mip.data.is_empty() {
        return Err("Mip data is empty (external .tfc reference)".into());
    }

    if mip.is_bulk_compressed {
        return Err("Mip data is LZO/ZLIB compressed — decompression not yet supported for thumbnails".into());
    }

    let mip_w = mip.size_x;
    let mip_h = mip.size_y;
    let format_str = format!("{:?}", texture.format);

    // Decompress pixel data to RGBA8
    let rgba = decode_to_rgba(&mip.data, mip_w, mip_h, texture.format)?;

    // Build the image
    let img = image::RgbaImage::from_raw(mip_w, mip_h, rgba)
        .ok_or("Failed to create image from decoded pixels")?;

    // Resize if larger than max_size
    let img = if mip_w > max_size || mip_h > max_size {
        let (new_w, new_h) = fit_dimensions(mip_w, mip_h, max_size);
        image::imageops::resize(&img, new_w, new_h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let (final_w, final_h) = (img.width(), img.height());

    // Encode to PNG
    let mut png_buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(Cursor::new(&mut png_buf));
    img.write_with_encoder(encoder)
        .map_err(|e| format!("PNG encode: {e}"))?;

    // Base64 encode
    use base64::Engine;
    let data_base64 = base64::engine::general_purpose::STANDARD.encode(&png_buf);

    Ok(TextureThumbnail {
        width: final_w,
        height: final_h,
        format: format_str,
        data_base64,
    })
}

// ---- Helpers ---------------------------------------------------------------

/// Pick the best mip level for a thumbnail.
///
/// We want the smallest mip that is still >= max_size on its largest dimension,
/// so we avoid decoding a 2048x2048 texture when a 256x256 mip exists.
/// Falls back to the largest mip with data if none meet the threshold.
fn pick_mip(
    mips: &[cimmeria_upk_objects::MipLevel],
    max_size: u32,
) -> Option<&cimmeria_upk_objects::MipLevel> {
    // Filter to mips that actually have data
    let with_data: Vec<&cimmeria_upk_objects::MipLevel> =
        mips.iter().filter(|m| !m.data.is_empty()).collect();

    if with_data.is_empty() {
        return None;
    }

    // Find the smallest mip where max(w,h) >= max_size
    let candidate = with_data
        .iter()
        .rev() // smallest first (mips are largest-first in UE3)
        .find(|m| m.size_x >= max_size || m.size_y >= max_size);

    // Fall back to the largest available mip
    Some(candidate.copied().unwrap_or(with_data[0]))
}

/// Decode compressed or raw texture data to RGBA8 bytes.
fn decode_to_rgba(
    data: &[u8],
    width: u32,
    height: u32,
    format: PixelFormat,
) -> Result<Vec<u8>, String> {
    let w = width as usize;
    let h = height as usize;

    match format {
        PixelFormat::DXT1 => {
            let mut pixels = vec![0u32; w * h];
            texture2ddecoder::decode_bc1(data, w, h, &mut pixels)
                .map_err(|e| format!("BC1 decode: {e}"))?;
            Ok(u32_pixels_to_rgba(&pixels))
        }
        PixelFormat::DXT3 => {
            // DXT3 = BC2. texture2ddecoder v0.0.5 doesn't have BC2, so we use
            // BC3 as an approximation — both are 16-byte blocks, only the alpha
            // encoding differs. Good enough for thumbnails.
            let mut pixels = vec![0u32; w * h];
            texture2ddecoder::decode_bc3(data, w, h, &mut pixels)
                .map_err(|e| format!("BC3 decode (DXT3 approx): {e}"))?;
            Ok(u32_pixels_to_rgba(&pixels))
        }
        PixelFormat::DXT5 => {
            let mut pixels = vec![0u32; w * h];
            texture2ddecoder::decode_bc3(data, w, h, &mut pixels)
                .map_err(|e| format!("BC3 decode: {e}"))?;
            Ok(u32_pixels_to_rgba(&pixels))
        }
        PixelFormat::A8R8G8B8 => {
            // Raw BGRA → RGBA swizzle
            if data.len() < w * h * 4 {
                return Err(format!(
                    "A8R8G8B8 data too short: need {} bytes, have {}",
                    w * h * 4,
                    data.len()
                ));
            }
            let mut rgba = Vec::with_capacity(w * h * 4);
            for pixel in data[..w * h * 4].chunks_exact(4) {
                // Source is BGRA (A8R8G8B8 in D3D byte order)
                rgba.push(pixel[2]); // R
                rgba.push(pixel[1]); // G
                rgba.push(pixel[0]); // B
                rgba.push(pixel[3]); // A
            }
            Ok(rgba)
        }
        PixelFormat::G8 => {
            // Grayscale → RGBA
            if data.len() < w * h {
                return Err(format!(
                    "G8 data too short: need {} bytes, have {}",
                    w * h,
                    data.len()
                ));
            }
            let mut rgba = Vec::with_capacity(w * h * 4);
            for &g in &data[..w * h] {
                rgba.push(g);
                rgba.push(g);
                rgba.push(g);
                rgba.push(255);
            }
            Ok(rgba)
        }
        PixelFormat::Unknown(id) => {
            Err(format!("Unsupported pixel format: Unknown({})", id))
        }
    }
}

/// Convert texture2ddecoder's u32 pixel array (0xAARRGGBB) to RGBA8 bytes.
fn u32_pixels_to_rgba(pixels: &[u32]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(pixels.len() * 4);
    for &pixel in pixels {
        rgba.push(((pixel >> 16) & 0xFF) as u8); // R
        rgba.push(((pixel >> 8) & 0xFF) as u8);  // G
        rgba.push((pixel & 0xFF) as u8);          // B
        rgba.push(((pixel >> 24) & 0xFF) as u8);  // A
    }
    rgba
}

/// Calculate dimensions that fit within max_size while preserving aspect ratio.
fn fit_dimensions(w: u32, h: u32, max_size: u32) -> (u32, u32) {
    let scale = if w >= h {
        max_size as f64 / w as f64
    } else {
        max_size as f64 / h as f64
    };
    let new_w = (w as f64 * scale).round().max(1.0) as u32;
    let new_h = (h as f64 * scale).round().max(1.0) as u32;
    (new_w, new_h)
}
